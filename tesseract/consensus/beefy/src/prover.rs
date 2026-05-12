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

use alloy_sol_types::SolValue;
use anyhow::anyhow;
use codec::{Decode, Encode};
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
use ismp_solidity_abi::ecdsa_beefy::BeefyConsensusProof;
use pallet_ismp_rpc::{BlockNumberOrHash, EventWithMetadata};
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::SubstrateClient;
use zk_beefy::BeefyProver as Sp1BeefyProverTrait;

use crate::{
	backend::{ConsensusProof, ProofBackend},
	extract_para_id,
};

/// Deserializes a 4-byte `ConsensusStateId` from a string (e.g. `"DOT0"` or `"PAS0"`).
fn deserialize_consensus_state_id<'de, D>(deserializer: D) -> Result<ConsensusStateId, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let s = String::deserialize(deserializer)?;
	let bytes = s.as_bytes();
	if bytes.len() != 4 {
		return Err(serde::de::Error::custom(format!(
			"consensus_state_id must be exactly 4 bytes, got {} (\"{}\")",
			bytes.len(),
			s
		)));
	}
	let mut id = [0u8; 4];
	id.copy_from_slice(bytes);
	Ok(id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyProverConfig {
	/// Consensus state id for the host on the counterparty (e.g. "BEEF" or "PAS0")
	#[serde(deserialize_with = "deserialize_consensus_state_id")]
	pub consensus_state_id: ConsensusStateId,
	/// Minimum height that must be enacted before we prove finality for new messages
	pub minimum_finalization_height: u64,
	/// State machines we are proving for. If empty or omitted, prove all.
	#[serde(default)]
	pub state_machines: Vec<StateMachine>,
	/// Which proof backend the prover (and the corresponding host) should use.
	/// Defaults to `Onchain` if not specified.
	#[serde(default)]
	pub backend: crate::backend::ProofBackendConfig,
}

/// Selects which proof strategy the BEEFY prover uses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofVariant {
	/// Verify the full 2/3+1 supermajority of ECDSA signatures on-chain (EcdsaBeefy).
	#[default]
	#[serde(alias = "naive")]
	Ecdsa,
	/// Delegate signature verification to an SP1 zero-knowledge proof (SP1Beefy).
	#[serde(alias = "zk")]
	Sp1,
	/// Deterministically sample a small subset of signatures via Fiat-Shamir
	/// (FiatShamirBeefy).
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
pub struct BeefyProver<
	R: subxt::Config,
	P: subxt::Config,
	B: Sp1BeefyProverTrait,
	Q: ProofBackend + ?Sized,
> {
	/// The hyperbridge substrate client
	client: SubstrateClient<P>,
	/// The beefy prover instance
	prover: Prover<R, P, B>,
	/// Unified backend for queue and state storage. Consensus state is always loaded
	/// fresh from this backend at each point of use rather than cached locally.
	backend: Arc<Q>,
	/// Prover configuration options
	config: BeefyProverConfig,
}

/// Global key in redis for the prover consensus state. The prover will write it's consensus state
/// to redis as frequently as they change. Ensuring that it can always be rehydrated.
pub const REDIS_CONSENSUS_STATE_KEY: &'static str = "consensus_state";

/// Proof type identifier for naive proofs (BeefyV1)
pub const PROOF_TYPE_ECDSA: u8 = 0x00;

/// Proof type identifier for ZK proofs (SP1Beefy)
pub const PROOF_TYPE_SP1: u8 = 0x01;

/// Proof type identifier for Fiat-Shamir sampled proofs (BeefyV1FiatShamir)
pub const PROOF_TYPE_FIAT_SHAMIR: u8 = 0x02;

impl<R, P, B, Q> BeefyProver<R, P, B, Q>
where
	R: subxt::Config + Send + Sync + Clone,
	P: subxt::Config + Send + Sync + Clone,
	B: Sp1BeefyProverTrait,
	Q: ProofBackend + ?Sized,
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
		// Sanity-check that the backend has a consensus state we can load.
		let consensus_state = backend.load_state().await?;
		log::info!(target: crate::LOG_TARGET, "Loaded consensus state: {consensus_state:#?}");

		// Initialize queues for the configured state machines
		backend.init_queues(&config.state_machines).await?;

		Ok(BeefyProver { prover, client, config, backend })
	}

	/// Generate an encoded proof
	pub async fn consensus_proof(
		&self,
		signed_commitment: SignedCommitment<u32, Signature>,
		consensus_state: ConsensusState,
	) -> Result<Vec<u8>, anyhow::Error> {
		let encoded = match self.prover {
			Prover::Ecdsa(ref naive, _) => {
				let message: BeefyConsensusProof =
					naive.consensus_proof(signed_commitment).await?.into();
				[&[PROOF_TYPE_ECDSA], message.abi_encode_params().as_slice()].concat()
			},
			Prover::Sp1(ref zk) => {
				let message = zk.consensus_proof(signed_commitment, consensus_state).await?;
				[&[PROOF_TYPE_SP1], message.abi_encode_params().as_slice()].concat()
			},
			Prover::FiatShamir(ref fs, _) => {
				let (consensus_message, bitmap) =
					fs.consensus_proof_fiat_shamir(signed_commitment, &consensus_state).await?;
				let message: BeefyConsensusProof = consensus_message.into();
				// FiatShamir expects abi.encode(RelayChainProof, ParachainProof,uint256[4])
				let bitmap_words: [alloy_primitives::U256; 4] = bitmap
					.words
					.iter()
					.map(|w| {
						let buf = w.to_big_endian();
						alloy_primitives::U256::from_be_bytes(buf)
					})
					.collect::<Vec<_>>()
					.try_into()
					.expect("bitmap should have exactly 4 words");
				let encoded = (message.relay, message.parachain, bitmap_words).abi_encode_params();
				[&[PROOF_TYPE_FIAT_SHAMIR], encoded.as_slice()].concat()
			},
		};

		Ok(encoded)
	}

	/// Returns the latest set of ismp messages that have been finalized and the latest finalized
	/// parachain block that was queried.
	pub async fn latest_ismp_message_events(
		&self,
		consensus_state: &ProverConsensusState,
		finalized: HashFor<R>,
	) -> Result<(u64, Vec<EventWithMetadata>), anyhow::Error> {
		let latest_height = consensus_state.finalized_parachain_height;
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
				if self.config.state_machines.is_empty() {
					matches!(dest, StateMachine::Evm(_)).then_some(event)
				} else {
					self.config.state_machines.iter().find(|s| **s == dest).map(|_| event)
				}
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
		consensus_state: &ProverConsensusState,
	) -> Result<(Option<(HashFor<R>, u64)>, generic::Header<u32, BlakeTwo256>), anyhow::Error> {
		let initial_height = consensus_state.inner.latest_beefy_height;
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
			.state_query_storage(
				vec![&BEEFY_VALIDATOR_SET_ID[..]],
				from,
				Some(header.hash().into()),
			)
			.await?;

		let changes_iter = changes.into_iter().filter_map(|change| {
			change.changes[0]
				.clone()
				.1
				.and_then(|data| u64::decode(&mut &*data.0).ok())
				.map(|id| (change.block, id))
		});

		tracing::trace!(target: crate::LOG_TARGET, "Latest set ID: {:#?}", changes_iter.clone().next_back());

		let block_hash_and_set_id = changes_iter
			.filter(|(_, set_id)| *set_id >= consensus_state.inner.next_authorities.id)
			.next();

		tracing::trace!(target: crate::LOG_TARGET, "Block hash and set id: {:#?}", block_hash_and_set_id);

		Ok((block_hash_and_set_id, header))
	}

	/// Performs a linear search for the BEEFY justification which finalizes the given epoch
	/// boundary
	pub async fn epoch_justification_for(
		&self,
		start: u64,
	) -> anyhow::Result<Option<SignedCommitment<u32, Signature>>> {
		let relay_rpc = self.prover.inner().relay_rpc.clone();
		tracing::info!(target: crate::LOG_TARGET, "Scanning for BEEFY justifications at {start}");

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
					target: crate::LOG_TARGET, "Found some justification at block: {i}: {:?}",
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
				tracing::trace!(target: crate::LOG_TARGET, "No BEEFY justifications found at {i}");
			}
		}

		Ok(None)
	}

	/// Runs the proving task. Will internally notify the appropriate channels of new epoch
	/// justifications as well as new proofs for ISMP messages.
	pub async fn run(&self) {
		let hyperbridge = self.client.state_machine_id().state_id;
		let para_id = extract_para_id(hyperbridge)
			.expect("StateMachine should be either one of Polkadot or Kusama");
		let relay_rpc = self.prover.inner().relay_rpc.clone();

		loop {
			tokio::time::sleep(Duration::from_secs(10)).await;

			let result: Result<_, anyhow::Error> = async {
				// Load fresh consensus state from the backend each iteration so we never act
				// on stale state — for the on-chain backend this reflects the latest accepted
				// proof on the pallet; for Redis / in-memory it's the last saved progress.
				let mut consensus_state = self.backend.load_state().await?;

				let (update, latest_beefy_header) =
					self.query_next_finalized_epoch(&consensus_state).await?;

				match update {
					Some((epoch_change_block_hash, next_set_id)) => {
						// invariant, update should always be for the next set
						tracing::info!(target: crate::LOG_TARGET, "Next authority set: {next_set_id}");
						assert_eq!(next_set_id, consensus_state.inner.next_authorities.id);

						let epoch_change_header = relay_rpc
							.chain_get_header(Some(epoch_change_block_hash))
							.await?
							.expect("Epoch change header exists");

						let Some(commitment) = self
							.epoch_justification_for(epoch_change_header.number().into())
							.await?
						else {
							// justification not yet available, retry next tick
							return Ok(());
						};

						tracing::info!(
							target: crate::LOG_TARGET, "Fetched next authority set justification: {:?}",
							commitment.commitment
						);

						let consensus_proof = self
							.consensus_proof(commitment.clone(), consensus_state.inner.clone())
							.await?;

						let finalized_hash = relay_rpc
							.chain_get_block_hash(Some(commitment.commitment.block_number.into()))
							.await?
							.expect("Epoch change header exists");
						let para_header = query_parachain_header(
							&self.prover.inner().relay_rpc,
							finalized_hash,
							para_id,
						)
						.await?;
						let finalized_parachain_height: u64 = para_header.number.into();

						let message = ConsensusProof {
							finalized_height: commitment.commitment.block_number,
							set_id: next_set_id,
							message: ConsensusMessage {
								consensus_proof,
								consensus_state_id: self.config.consensus_state_id,
								signer: H256::random().encode(),
							},
						};

						let destinations: Vec<StateMachine> = self.config.state_machines.clone();
						tracing::info!("Sending mandatory consensus proof");
						self.backend.send_mandatory_proof(&destinations, message).await?;

						// Advance the locally-computed view and persist it to the backend.
						// Rotate authorities inline: new current = old next, new next =
						// queried from the relay at the epoch-change hash.
						consensus_state.finalized_parachain_height = finalized_parachain_height;
						consensus_state.inner.latest_beefy_height =
							commitment.commitment.block_number;
						consensus_state.inner.current_authorities =
							consensus_state.inner.next_authorities.clone();
						consensus_state.inner.next_authorities =
							beefy_prover::relay::beefy_mmr_leaf_next_authorities(
								&self.prover.inner().relay_rpc,
								Some(epoch_change_block_hash),
							)
							.await?;
						tracing::info!(
							target: crate::LOG_TARGET, "Rotated authority set. Current {}, Next: {}",
							consensus_state.inner.current_authorities.id,
							consensus_state.inner.next_authorities.id,
						);
						self.backend.save_state(&consensus_state).await?;
						return Ok(()); // check for next updates
					},
					None => {},
				}

				let (latest_parachain_height, messages) = self
					.latest_ismp_message_events(
						&consensus_state,
						latest_beefy_header.parent_hash.into(),
					)
					.await?;

				if messages.is_empty() {
					consensus_state.finalized_parachain_height = latest_parachain_height;
					self.backend.save_state(&consensus_state).await?;
					return Ok(());
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
						target: crate::LOG_TARGET, "Waiting for {} blocks before proving finality for messages in the range: {lowest_message_height}..{latest_parachain_height}",
						minimum_height - latest_parachain_height
					);
					return Ok(());
				}

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

				tracing::trace!(target: crate::LOG_TARGET, "State machines: {state_machines:?}");

				if state_machines.is_empty() {
					tracing::trace!(
						target: crate::LOG_TARGET, "No new messages in the range: {lowest_message_height}..{latest_parachain_height}"
					);
					return Ok(());
				}

				tracing::info!(
					target: crate::LOG_TARGET, "Proving finality for messages in the range: {lowest_message_height}..{latest_parachain_height}"
				);

				let latest_beefy_header_hash = latest_beefy_header.hash().into();
				let (commitment, _) = fetch_latest_beefy_justification(
					&self.prover.inner().relay_rpc,
					latest_beefy_header_hash,
				)
				.await?;
				let consensus_proof = self
					.consensus_proof(commitment.clone(), consensus_state.inner.clone())
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

				let destinations: Vec<StateMachine> = state_machines.into_iter().collect();
				tracing::info!(
					"Sending consensus proof for new messages in range {lowest_message_height}..{latest_parachain_height} to {destinations:?}"
				);
				self.backend.send_messages_proof(&destinations, message).await?;

				consensus_state.inner.latest_beefy_height = *latest_beefy_header.number();
				consensus_state.finalized_parachain_height = latest_parachain_height;
				self.backend.save_state(&consensus_state).await?;

				Ok(())
			}
			.await;

			if let Err(err) = result {
				tracing::error!(target: crate::LOG_TARGET, "Prover error: {err:?}");
			}
		}
	}
}

/// Beefy prover, can produce ECDSA, SP1, or Fiat-Shamir proofs
pub enum Prover<R: subxt::Config, P: subxt::Config, B: Sp1BeefyProverTrait> {
	/// ECDSA prover — verifies all 2/3+1 signatures on-chain
	Ecdsa(beefy_prover::Prover<R, P>, PhantomData<B>),
	/// SP1 prover — delegates signature verification to an SP1 ZK program
	Sp1(zk_beefy::Prover<R, P, B>),
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
			Prover::Ecdsa(p, _) => Prover::Ecdsa(p.clone(), PhantomData),
			Prover::Sp1(p) => Prover::Sp1(p.clone()),
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
			ProofVariant::Sp1 => {
				let sp1_prover = zk_beefy::LocalProver::new().await?;
				Prover::Sp1(zk_beefy::Prover::new(prover, sp1_prover))
			},
			ProofVariant::Ecdsa => Prover::Ecdsa(prover, PhantomData),
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
			Prover::Sp1(ref p) => &p.inner,
			Prover::Ecdsa(ref p, _) => p,
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
