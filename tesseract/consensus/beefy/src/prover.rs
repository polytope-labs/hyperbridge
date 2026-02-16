// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::anyhow;
use codec::{Decode, Encode};
use ethers::abi::{AbiEncode, Token, Tokenizable};
use hex_literal::hex;
use polkadot_sdk::*;
use primitive_types::H256;
use serde::{Deserialize, Serialize};
use sp_consensus_beefy::{
	ecdsa_crypto::Signature, known_payloads::MMR_ROOT_ID, SignedCommitment, VersionedFinalityProof,
};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, Header as _, Keccak256},
};
use std::{
	collections::{HashMap, HashSet},
	marker::PhantomData,
	sync::Arc,
	time::Duration,
};
use subxt::{
	backend::legacy::LegacyRpcMethods,
	config::{ExtrinsicParams, HashFor, Hasher, Header},
	ext::subxt_rpcs::rpc_params,
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature},
};

use beefy_prover::{
	relay::{fetch_latest_beefy_justification, parachain_header_storage_key},
	BEEFY_VALIDATOR_SET_ID,
};
use beefy_verifier_primitives::ConsensusState;
use ismp::{
	consensus::ConsensusStateId, events::Event, host::StateMachine, messaging::ConsensusMessage,
};
use ismp_solidity_abi::beefy::BeefyConsensusProof;
use pallet_ismp_rpc::{BlockNumberOrHash, EventWithMetadata};
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::SubstrateClient;
use zk_beefy::BeefyProver as Sp1BeefyProverTrait;

use crate::{
	backend::{ConsensusProof, ProofBackend},
	extract_para_id, VALIDATOR_SET_ID_KEY,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyProverConfig {
	/// Consensus state id for the host on the counterparty
	pub consensus_state_id: ConsensusStateId,
	/// Minimum height that must be enacted before we prove finality for new messages
	pub minimum_finalization_height: u64,
	/// State machines we are proving for
	pub state_machines: Vec<StateMachine>,
	/// Optional Redis configuration for proof backend (if None, uses InMemoryProofBackend)
	pub redis: Option<crate::backend::RedisConfig>,
}

/// Selects which proof strategy the BEEFY prover uses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofVariant {
	/// Verify the full 2/3+1 supermajority of signatures on-chain (BeefyV1).
	#[default]
	Naive,
	/// Delegate signature verification to an SP1 ZK program (SP1Beefy).
	Zk,
	/// Deterministically sample a small subset of signatures via Fiat-Shamir
	/// (BeefyV1FiatShamir).
	FiatShamir,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverConfig {
	/// RPC ws url for a relay chain
	pub relay_rpc_ws: String,
	/// RPC ws url for the parachain
	pub para_rpc_ws: String,
	/// para Id for the parachain
	pub para_ids: Vec<u32>,
	/// Which proof variant to produce.
	#[serde(default)]
	pub proof_variant: ProofVariant,
	/// Maximum size in bytes for the rpc payloads, both requests & responses.
	pub max_rpc_payload_size: Option<u32>,
	/// Query batch size for mmr leaves
	pub query_batch_size: Option<u32>,
}

/// The BEEFY prover produces BEEFY consensus proofs using either the naive or zk variety. Consensus
/// proofs are produced when new messages are observed on the hyperbridge chain or when the
/// authority set changes.
pub struct BeefyProver<R: subxt::Config, P: subxt::Config, B: Sp1BeefyProverTrait, Q: ProofBackend>
{
	/// The prover's consensus state
	consensus_state: ProverConsensusState,
	/// The hyperbridge substrate client
	client: SubstrateClient<P>,
	/// The beefy prover instance
	prover: Prover<R, P, B>,
	/// Unified backend for queue and state storage
	backend: Arc<Q>,
	/// Prover configuration options
	config: BeefyProverConfig,
}

/// Global key in redis for the prover consensus state. The prover will write it's consensus state
/// to redis as frequently as they change. Ensuring that it can always be rehydrated.
pub const REDIS_CONSENSUS_STATE_KEY: &'static str = "consensus_state";

/// Proof type identifier for naive proofs (BeefyV1)
pub const PROOF_TYPE_NAIVE: u8 = 0x00;

/// Proof type identifier for ZK proofs (SP1Beefy)
pub const PROOF_TYPE_ZK: u8 = 0x01;

/// Proof type identifier for Fiat-Shamir sampled proofs (BeefyV1FiatShamir)
pub const PROOF_TYPE_FIAT_SHAMIR: u8 = 0x02;

impl<R, P, B, Q> BeefyProver<R, P, B, Q>
where
	R: subxt::Config + Send + Sync + Clone,
	P: subxt::Config + Send + Sync + Clone,
	B: Sp1BeefyProverTrait,
	Q: ProofBackend,
	P::Header: Send + Sync,
	<P::ExtrinsicParams as ExtrinsicParams<P>>::Params: Send + Sync + DefaultParams,
	P::AccountId: From<AccountId32> + Into<P::Address> + Clone + Send + Sync,
	P::Signature: From<MultiSignature> + Send + Sync,
	H256: From<HashFor<P>>,
	HashFor<R>: From<H256>,
{
	/// Constructs an instance of the [`BeefyProver`]
	pub async fn new(
		config: BeefyProverConfig,
		client: SubstrateClient<P>,
		prover: Prover<R, P, B>,
		backend: Arc<Q>,
	) -> Result<Self, anyhow::Error> {
		let consensus_state = backend.load_state().await?;

		log::info!("Loaded consensus state: {consensus_state:#?}");

		// Initialize queues for the configured state machines
		backend.init_queues(&config.state_machines).await?;

		Ok(BeefyProver { consensus_state, prover, client, config, backend })
	}

	/// Rotate the prover's known authority set, using the network view at the provided hash
	async fn rotate_authorities(&mut self, hash: HashFor<R>) -> Result<(), anyhow::Error> {
		self.consensus_state.inner.current_authorities =
			self.consensus_state.inner.next_authorities.clone();
		self.consensus_state.inner.next_authorities =
			beefy_prover::relay::beefy_mmr_leaf_next_authorities(
				&self.prover.inner().relay_rpc,
				Some(hash),
			)
			.await?;

		tracing::info!(
			"Rotated authority set. Current {}, Next: {}",
			self.consensus_state.inner.current_authorities.id,
			self.consensus_state.inner.next_authorities.id,
		);

		Ok(())
	}

	/// Generate an encoded proof
	pub async fn consensus_proof(
		&self,
		signed_commitment: SignedCommitment<u32, Signature>,
		consensus_state: ConsensusState,
	) -> Result<Vec<u8>, anyhow::Error> {
		let encoded = match self.prover {
			Prover::Naive(ref naive, _) => {
				let message: BeefyConsensusProof =
					naive.consensus_proof(signed_commitment).await?.into();
				[&[PROOF_TYPE_NAIVE], AbiEncode::encode(message).as_slice()].concat()
			},
			Prover::ZK(ref zk) => {
				let message = zk.consensus_proof(signed_commitment, consensus_state).await?;
				[&[PROOF_TYPE_ZK], AbiEncode::encode(message).as_slice()].concat()
			},
			Prover::FiatShamir(ref fs, _) => {
				let (consensus_message, bitmap) =
					fs.consensus_proof_fiat_shamir(signed_commitment, &consensus_state).await?;
				let message: BeefyConsensusProof = consensus_message.into();
				// The FiatShamir verifier expects abi.encode(RelayChainProof, ParachainProof,
				// uint256[4])
				let bitmap_token = Token::FixedArray(
					bitmap
						.words
						.iter()
						.map(|w| {
							Token::Uint(ethers::types::U256::from_big_endian(&w.to_big_endian()))
						})
						.collect(),
				);
				let encoded = ethers::abi::encode(&[
					message.relay.into_token(),
					message.parachain.into_token(),
					bitmap_token,
				]);
				[&[PROOF_TYPE_FIAT_SHAMIR], encoded.as_slice()].concat()
			},
		};

		Ok(encoded)
	}

	/// Returns the latest set of ismp messages that have been finalized and the latest finalized
	/// parachain block that was queried.
	pub async fn latest_ismp_message_events(
		&self,
		finalized: HashFor<R>,
	) -> Result<(u64, Vec<EventWithMetadata>), anyhow::Error> {
		let latest_height = self.consensus_state.finalized_parachain_height;
		let para_id = extract_para_id(self.client.state_machine_id().state_id)?;
		let header =
			query_parachain_header(&self.prover.inner().relay_rpc, finalized, para_id).await?;
		let finalized_height = header.number.into();
		if finalized_height <= latest_height {
			return Ok((latest_height, vec![]));
		}
		let hyperbridge = self.client.state_machine_id().state_id;

		let events = self.query_ismp_events_with_metadata(latest_height, finalized_height).await?;
		let events = events
			.into_iter()
			.filter_map(|event| {
				let dest = match &event.event {
					Event::PostRequest(post) => post.dest.clone(),
					Event::PostResponse(resp) => resp.dest_chain(),
					Event::GetResponse(resp) => resp.get.source.clone(),
					Event::PostRequestTimeoutHandled(req) if req.source != hyperbridge =>
						req.source,
					Event::PostResponseTimeoutHandled(res) if res.source != hyperbridge =>
						res.source,
					_ => None?,
				};

				// filter out destinations that the prover isn't configured for
				self.config.state_machines.iter().find(|s| **s == dest).map(|_| event)
			})
			.collect::<Vec<_>>();

		return Ok((finalized_height, events));
	}

	/// Query the ismp events emitted within the block range (inclusive)
	async fn query_ismp_events_with_metadata(
		&self,
		previous_height: u64,
		latest_height: u64,
	) -> Result<Vec<EventWithMetadata>, anyhow::Error> {
		let range = (previous_height + 1)..=latest_height;

		if range.is_empty() {
			return Ok(Default::default());
		}

		let params = rpc_params![
			BlockNumberOrHash::<H256>::Number(previous_height.saturating_add(1) as u32),
			BlockNumberOrHash::<H256>::Number(latest_height as u32)
		];
		let response: HashMap<String, Vec<EventWithMetadata>> =
			self.client.rpc_client.request("ismp_queryEventsWithMetadata", params).await?;
		let events = response.into_values().flatten().collect();
		Ok(events)
	}

	/// Queries for any authority set changes in between the latest relay chain block finalized
	/// by beefy and the last known finalized block.
	pub async fn query_next_finalized_epoch(
		&self,
	) -> Result<(Option<(HashFor<R>, u64)>, generic::Header<u32, BlakeTwo256>), anyhow::Error> {
		let initial_height = self.consensus_state.inner.latest_beefy_height;
		let relay_rpc = self.prover.inner().relay_rpc.clone();
		let from = relay_rpc
			.chain_get_block_hash(Some(initial_height.into()))
			.await?
			.ok_or_else(|| anyhow!("Block hash should exist"))?;

		let header = {
			let hash = self
				.prover
				.inner()
				.relay_rpc_client
				.request::<HashFor<R>>("beefy_getFinalizedHead", rpc_params![])
				.await?;
			let h = relay_rpc
				.chain_get_header(Some(hash))
				.await?
				.ok_or_else(|| anyhow!("Block hash should exist"))?;

			generic::Header::<u32, BlakeTwo256>::decode(&mut &*h.encode()).unwrap()
		};

		let changes = relay_rpc
			.state_query_storage(vec![&VALIDATOR_SET_ID_KEY[..]], from, Some(header.hash().into()))
			.await?;

		let mut block_hash_and_set_id = changes
			.into_iter()
			.filter_map(|change| {
				change.changes[0]
					.clone()
					.1
					.and_then(|data| u64::decode(&mut &*data.0).ok())
					.map(|id| (change.block, id))
			})
			.filter(|(_, set_id)| *set_id >= self.consensus_state.inner.next_authorities.id);

		Ok((block_hash_and_set_id.next(), header))
	}

	/// Performs a linear search for the BEEFY justification which finalizes the given epoch
	/// boundary
	pub async fn epoch_justification_for(
		&self,
		start: u64,
	) -> anyhow::Result<Option<SignedCommitment<u32, Signature>>> {
		let relay_rpc = self.prover.inner().relay_rpc.clone();
		tracing::info!("Scanning for BEEFY justifications at {start}");

		for i in start..=(start + 2400) {
			let hash = if let Some(hash) = relay_rpc.chain_get_block_hash(Some(i.into())).await? {
				hash
			} else {
				continue;
			};

			if let Some(justifications) = relay_rpc
				.chain_get_block(Some(hash))
				.await?
				.ok_or_else(|| anyhow!("failed to find block for {hash:?}"))?
				.justifications
			{
				tracing::info!(
					"Found some justification at block: {i}: {:?}",
					justifications
						.iter()
						.map(|(id, _)| String::from_utf8(id.as_slice().to_vec()))
						.collect::<Result<Vec<_>, _>>()
				);
				let beefy = justifications
					.into_iter()
					.find(|justfication| justfication.0 == sp_consensus_beefy::BEEFY_ENGINE_ID);

				if let Some((_, proof)) = beefy {
					let VersionedFinalityProof::V1(commitment) =
						VersionedFinalityProof::<u32, Signature>::decode(&mut &*proof)
							.expect("Beefy justification should decode correctly");
					return Ok(Some(commitment));
				}
			} else {
				tracing::trace!("No BEEFY justifications found at {i}");
			}
		}

		Ok(None)
	}

	/// Runs the proving task. Will internally notify the appropriate channels of new epoch
	/// justifications as well as new proofs for ISMP messages.
	pub async fn run(&mut self) {
		let hyperbridge = self.client.state_machine_id().state_id;
		let para_id = extract_para_id(hyperbridge)
			.expect("StateMachine should be either one of Polkadot or Kusama");
		let relay_rpc = self.prover.inner().relay_rpc.clone();

		loop {
			let future = async {
				loop {
					// tick the interval
					tokio::time::sleep(Duration::from_secs(10)).await;

					let (update, mut latest_beefy_header) =
						self.query_next_finalized_epoch().await?;

					if let Some((epoch_change_block_hash, next_set_id)) = update {
						// invariant, update should always be for the next set
						assert_eq!(next_set_id, self.consensus_state.inner.next_authorities.id);
						tracing::info!("Next authority set: {next_set_id}");

						let epoch_change_header = relay_rpc
							.chain_get_header(Some(epoch_change_block_hash))
							.await?
							.expect("Epoch change header exists");
						if let Some(commitment) = self
							.epoch_justification_for(epoch_change_header.number().into())
							.await?
						{
							tracing::info!(
								"Fetched next authority set justification: {:?}",
								commitment.commitment
							);
							let consensus_proof = self
								.consensus_proof(
									commitment.clone(),
									self.consensus_state.inner.clone(),
								)
								.await?;

							let message = ConsensusProof {
								finalized_height: commitment.commitment.block_number,
								set_id: next_set_id,
								message: ConsensusMessage {
									consensus_proof,
									consensus_state_id: self.config.consensus_state_id,
									signer: H256::random().encode(),
								},
							};

							for state_machine in self.config.state_machines.iter() {
								tracing::info!(
									"Sending mandatory consensus proof to {state_machine}"
								);

								self.backend
									.send_mandatory_proof(state_machine, message.clone())
									.await?;
							}

							// update consesnsus state
							self.consensus_state.finalized_parachain_height = {
								let finalized_hash = relay_rpc
									.chain_get_block_hash(Some(
										commitment.commitment.block_number.into(),
									))
									.await?
									.expect("Epoch change header exists");
								let para_header = query_parachain_header(
									&self.prover.inner().relay_rpc,
									finalized_hash,
									para_id,
								)
								.await?;
								para_header.number.into()
							};
							self.consensus_state.inner.latest_beefy_height =
								commitment.commitment.block_number;
							self.rotate_authorities(epoch_change_block_hash).await?;
							self.backend.save_state(&self.consensus_state).await?;
						}
					}

					let (latest_parachain_height, messages) = self
						.latest_ismp_message_events(latest_beefy_header.parent_hash.into())
						.await?;

					if messages.is_empty() {
						self.consensus_state.finalized_parachain_height = latest_parachain_height;
						self.backend.save_state(&self.consensus_state).await?;
						continue;
					}

					let lowest_message_height = messages
						.iter()
						.min_by(|a, b| a.meta.block_number.cmp(&b.meta.block_number))
						.expect("Messages is not empty; qed")
						.meta
						.block_number;

					let minimum_height =
						lowest_message_height + self.config.minimum_finalization_height;
					if minimum_height > latest_parachain_height {
						tracing::info!(
							"Waiting for {} blocks before proving finality for messages in the range: {lowest_message_height}..{latest_parachain_height}",
							minimum_height - latest_parachain_height
						);

						loop {
							tokio::time::sleep(Duration::from_secs(10)).await;
							let header = {
								let hash = self
									.prover
									.inner()
									.relay_rpc_client
									.request::<HashFor<R>>("beefy_getFinalizedHead", rpc_params![])
									.await?;
								let h = relay_rpc
									.chain_get_header(Some(hash))
									.await?
									.ok_or_else(|| anyhow!("Block hash should exist"))?;

								generic::Header::<u32, BlakeTwo256>::decode(&mut &h.encode()[..])?
							};

							let para_header = query_parachain_header(
								&self.prover.inner().relay_rpc,
								header.parent_hash.into(),
								para_id,
							)
							.await?;

							if para_header.number as u64 >= minimum_height {
								latest_beefy_header = header;
								break;
							}
						}
					}

					let (latest_parachain_height, messages) = self
						.latest_ismp_message_events(latest_beefy_header.parent_hash.into())
						.await?;

					let state_machines = messages
						.iter()
						.filter_map(|e| {
							let event = match &e.event {
								Event::PostRequest(req) => req.dest,
								Event::PostResponse(res) => res.dest_chain(),
								Event::GetResponse(res) => res.get.source,
								Event::PostRequestTimeoutHandled(req)
									if req.source != hyperbridge =>
									req.source,
								Event::PostResponseTimeoutHandled(res)
									if res.source != hyperbridge =>
									res.source,
								_ => None?,
							};
							Some(event)
						})
						.collect::<HashSet<_>>();

					tracing::trace!("State machines: {state_machines:?}");

					if state_machines.len() == 0 {
						tracing::trace!("No new messages in the range: {lowest_message_height}..{latest_parachain_height}");

						continue;
					}

					tracing::info!("Proving finality for messages in the range: {lowest_message_height}..{latest_parachain_height}");

					let latest_beefy_header_hash = latest_beefy_header.hash().into();
					let (commitment, _) = fetch_latest_beefy_justification(
						&self.prover.inner().relay_rpc,
						latest_beefy_header_hash,
					)
					.await?;
					let consensus_proof = self
						.consensus_proof(commitment.clone(), self.consensus_state.inner.clone())
						.await?;

					let set_id = relay_rpc
						.state_get_storage(
							BEEFY_VALIDATOR_SET_ID.as_slice(),
							Some(latest_beefy_header_hash),
						)
						.await?
						.map(|data| u64::decode(&mut data.as_ref()))
						.transpose()?
						.ok_or_else(|| anyhow!("Couldn't fetch latest beefy authority set"))?;

					let message = ConsensusProof {
						finalized_height: commitment.commitment.block_number,
						set_id,
						message: ConsensusMessage {
							consensus_proof,
							consensus_state_id: self.config.consensus_state_id,
							signer: H256::random().encode(),
						},
					};

					// notify all relevant state machines
					for state_machine in state_machines {
						tracing::info!("Sending consensus proof for new messages in range {lowest_message_height}..{latest_parachain_height} to {state_machine}");
						self.backend.send_messages_proof(&state_machine, message.clone()).await?; // fatal
					}

					self.consensus_state.inner.latest_beefy_height = *latest_beefy_header.number();
					self.consensus_state.finalized_parachain_height = latest_parachain_height;
					self.backend.save_state(&self.consensus_state).await?;
				}

				#[allow(unreachable_code)]
				Ok::<_, anyhow::Error>(())
			};

			if let Err(err) = future.await {
				tracing::error!("Prover error: {err:?}");
				// The queue and state storage implementations should handle reconnection internally
				// or the error will propagate and the loop will retry
			}
		}
	}
}

/// Beefy prover, can either produce zk proofs or naive proofs
pub enum Prover<R: subxt::Config, P: subxt::Config, B: Sp1BeefyProverTrait> {
	/// The naive prover — verifies all 2/3+1 signatures on-chain
	Naive(beefy_prover::Prover<R, P>, PhantomData<B>),
	/// ZK prover — delegates signature verification to an SP1 program
	ZK(zk_beefy::Prover<R, P, B>),
	/// Fiat-Shamir prover — deterministically samples SAMPLE_SIZE signatures for on-chain
	/// verification
	FiatShamir(beefy_prover::Prover<R, P>, PhantomData<B>),
}

impl<R, P, B> Clone for Prover<R, P, B>
where
	R: subxt::Config,
	P: subxt::Config,
	B: Sp1BeefyProverTrait,
	beefy_prover::Prover<R, P>: Clone,
	zk_beefy::Prover<R, P, B>: Clone,
{
	fn clone(&self) -> Self {
		match self {
			Prover::Naive(p, _) => Prover::Naive(p.clone(), PhantomData),
			Prover::ZK(p) => Prover::ZK(p.clone()),
			Prover::FiatShamir(p, _) => Prover::FiatShamir(p.clone(), PhantomData),
		}
	}
}

// Implementation for LocalProver
impl<R, P> Prover<R, P, zk_beefy::LocalProver>
where
	R: subxt::Config,
	P: subxt::Config,
{
	pub async fn new(config: ProverConfig) -> Result<Self, anyhow::Error> {
		let max_rpc_payload_size = config.max_rpc_payload_size.unwrap_or(15 * 1024 * 1024);
		let (relay_chain, relay_rpc_client) =
			subxt_utils::client::ws_client::<R>(&config.relay_rpc_ws, max_rpc_payload_size).await?;
		let (parachain, parachain_rpc_client) =
			subxt_utils::client::ws_client::<P>(&config.para_rpc_ws, max_rpc_payload_size).await?;

		let relay_rpc = LegacyRpcMethods::<R>::new(relay_rpc_client.clone());

		let header = relay_rpc
			.chain_get_header(None)
			.await?
			.ok_or_else(|| anyhow!("No blocks on the relay chain?"))?;

		let parachain_rpc = LegacyRpcMethods::<P>::new(parachain_rpc_client.clone());

		let metadata = relay_chain.metadata();
		let hasher = R::Hasher::new(&metadata);
		let header_hash = header.hash_with(hasher);

		let leaves = relay_rpc
			.state_get_storage(
				hex!("a8c65209d47ee80f56b0011e8fd91f508156209906244f2341137c136774c91d").as_slice(),
				Some(header_hash),
			)
			.await?
			.map(|data| u64::decode(&mut data.as_ref()))
			.transpose()?
			.ok_or_else(|| anyhow!("Couldn't fetch latest beefy authority set"))?;

		let prover = beefy_prover::Prover {
			beefy_activation_block: (header.number().into() - leaves) as u32,
			relay: relay_chain,
			relay_rpc,
			relay_rpc_client,
			para: parachain,
			para_rpc: parachain_rpc,
			para_rpc_client: parachain_rpc_client,
			para_ids: config.para_ids,
			query_batch_size: config.query_batch_size,
		};

		let prover = match config.proof_variant {
			ProofVariant::FiatShamir => Prover::FiatShamir(prover, PhantomData),
			ProofVariant::Zk => {
				let sp1_prover = zk_beefy::LocalProver::new(true);
				Prover::ZK(zk_beefy::Prover::new(prover, sp1_prover))
			},
			ProofVariant::Naive => Prover::Naive(prover, PhantomData),
		};

		Ok(prover)
	}
}

// Common implementation for all variants
impl<R, P, B> Prover<R, P, B>
where
	R: subxt::Config,
	P: subxt::Config,
	B: Sp1BeefyProverTrait,
{
	/// Return the inner prover
	pub fn inner(&self) -> &beefy_prover::Prover<R, P> {
		match self {
			Prover::ZK(ref p) => &p.inner,
			Prover::Naive(ref p, _) => p,
			Prover::FiatShamir(ref p, _) => p,
		}
	}

	/// Construct the initial [`ProverConsensusState`] for use by both the verifier and prover.
	pub async fn query_initial_consensus_state(
		&self,
		hash: Option<HashFor<R>>,
	) -> Result<ProverConsensusState, anyhow::Error> {
		let inner = self.inner();
		let latest_finalized_head = match hash {
			Some(hash) => hash,
			None => inner.relay_rpc_client.request("beefy_getFinalizedHead", rpc_params!()).await?,
		};
		let (signed_commitment, _) =
			fetch_latest_beefy_justification(&inner.relay_rpc, latest_finalized_head).await?;
		let para_header =
			query_parachain_header(&inner.relay_rpc, latest_finalized_head, inner.para_ids[0])
				.await?;

		// Encoding and decoding to fix dependency version conflicts
		let next_authority_set = beefy_prover::relay::beefy_mmr_leaf_next_authorities(
			&inner.relay_rpc,
			Some(latest_finalized_head),
		)
		.await?;

		let current_authority_set =
			inner.mmr_leaf_current_authorities(Some(latest_finalized_head)).await?;

		let mmr_root_hash = signed_commitment
			.commitment
			.payload
			.get_decoded::<H256>(&MMR_ROOT_ID)
			.expect("Mmr root hash should decode correctly");

		let consensus_state = ConsensusState {
			mmr_root_hash,
			beefy_activation_block: inner.beefy_activation_block,
			latest_beefy_height: signed_commitment.commitment.block_number,
			current_authorities: current_authority_set.clone(),
			next_authorities: next_authority_set.clone(),
		};

		Ok(ProverConsensusState {
			inner: consensus_state,
			finalized_parachain_height: para_header.number.into(),
		})
	}
}

#[derive(Debug, Clone, Decode, Encode)]
pub struct ProverConsensusState {
	/// Inner consensus state tracked by the onchain light clients
	pub inner: ConsensusState,
	/// latest parachain height that has been finalized by BEEFY
	pub finalized_parachain_height: u64,
}

/// Query the parachain header that is finalized at the given relay chain block hash
pub async fn query_parachain_header<R: subxt::Config>(
	rpc: &LegacyRpcMethods<R>,
	hash: HashFor<R>,
	para_id: u32,
) -> Result<polkadot_sdk::sp_runtime::generic::Header<u32, Keccak256>, anyhow::Error> {
	let head_data = rpc
		.state_get_storage(parachain_header_storage_key(para_id).as_ref(), Some(hash))
		.await?
		.map(|data| Vec::<u8>::decode(&mut data.as_ref()))
		.transpose()?
		.ok_or_else(|| {
			anyhow!(
				"Could not fetch header for parachain with id {para_id} at block height {hash:?}"
			)
		})?;

	let header =
		polkadot_sdk::sp_runtime::generic::Header::<u32, Keccak256>::decode(&mut &head_data[..])?;

	Ok(header)
}
