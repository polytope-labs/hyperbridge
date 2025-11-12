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

use anyhow::{anyhow, Context};
use bytes::Buf;
use codec::{Decode, Encode};
use ethers::abi::AbiEncode;
use hex_literal::hex;
use polkadot_sdk::*;
use primitive_types::H256;
use redis::{AsyncCommands, RedisError};
use rsmq_async::{Rsmq, RsmqConnection, RsmqError};
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

use crate::{
	extract_para_id, host::ConsensusProof, redis_utils, redis_utils::RedisConfig,
	VALIDATOR_SET_ID_KEY,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyProverConfig {
	/// Redis configuration for message queues
	pub redis: RedisConfig,
	/// Consensus state id for the host on the counterparty
	pub consensus_state_id: ConsensusStateId,
	/// Minimum height that must be enacted before we prove finality for new messages
	pub minimum_finalization_height: u64,
	/// State machines we are proving for
	pub state_machines: Vec<StateMachine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProverConfig {
	/// RPC ws url for a relay chain
	pub relay_rpc_ws: String,
	/// RPC ws url for the parachain
	pub para_rpc_ws: String,
	/// para Id for the parachain
	pub para_ids: Vec<u32>,
	/// Generate zk BEEFY proofs?
	pub zk_beefy: bool,
	/// Maximum size in bytes for the rpc payloads, both requests & responses.
	pub max_rpc_payload_size: Option<u32>,
	/// Query batch size for mmr leaves
	pub query_batch_size: Option<u32>,
}

/// The BEEFY prover produces BEEFY consensus proofs using either the naive or zk variety. Consensus
/// proofs are produced when new messages are observed on the hyperbridge chain or when the
/// authority set changes.
pub struct BeefyProver<R: subxt::Config, P: subxt::Config> {
	/// The prover's consensus state, this is persisted to redis
	consensus_state: ProverConsensusState,
	/// The hyperbridge substrate client
	client: SubstrateClient<P>,
	/// The beefy prover instance
	prover: Prover<R, P>,
	/// Rsmq for interacting with the queue
	rsmq: Rsmq,
	/// Prover configuration options
	config: BeefyProverConfig,
	/// redis connection for reading and writing consensus state
	connection: redis::aio::ConnectionManager,
}

/// Global key in redis for the prover consensus state. The prover will write it's consensus state
/// to redis as frequently as they change. Ensuring that it can always be rehydrated.
pub const REDIS_CONSENSUS_STATE_KEY: &'static str = "consensus_state";

impl<R, P> BeefyProver<R, P>
where
	R: subxt::Config + Send + Sync + Clone,
	P: subxt::Config + Send + Sync + Clone,
	P::Header: Send + Sync,
	<P::ExtrinsicParams as ExtrinsicParams<P>>::Params: Send + Sync + DefaultParams,
	P::AccountId: From<AccountId32> + Into<P::Address> + Clone + Send + Sync,
	P::Signature: From<MultiSignature> + Send + Sync,
	H256: From<HashFor<P>>,
	HashFor<R>: From<H256>,
{
	/// Constructs an instance of the [`BeefyProver`]
	pub async fn new(
		mut config: BeefyProverConfig,
		client: SubstrateClient<P>,
		prover: Prover<R, P>,
	) -> Result<Self, anyhow::Error> {
		let mut connection = redis::Client::open(redis::ConnectionInfo {
			addr: config.redis.clone().into(),
			redis: redis::RedisConnectionInfo {
				db: config.redis.db as i64,
				username: config.redis.username.clone(),
				password: config.redis.password.clone(),
			},
		})?
		.get_connection_manager()
		.await?;
		let consensus_state = {
			let encoded = connection.get::<_, bytes::Bytes>(REDIS_CONSENSUS_STATE_KEY).await?;
			ProverConsensusState::decode(&mut encoded.chunk())?
		};

		log::info!("Rehydrated consensus state from redis: {consensus_state:#?}");

		// we want to publish queue notifications from the prover
		config.redis.realtime = true;
		let rsmq = redis_utils::rsmq_client(&config.redis).await?;

		Ok(BeefyProver { consensus_state, rsmq, prover, client, config, connection })
	}

	/// Initialize all the relevant queues for the configured state machines.
	pub async fn init_queues(&mut self) -> Result<(), anyhow::Error> {
		for state_machine in self.config.state_machines.iter() {
			let result = self
				.rsmq
				.create_queue(
					self.config.redis.mandatory_queue(state_machine).as_str(),
					Some(Duration::ZERO),
					Some(Duration::ZERO),
					Some(-1),
				)
				.await;

			if !(matches!(result, Ok(_) | Err(RsmqError::QueueExists))) {
				result.context(format!("Failed to create mandatory queue for {state_machine}"))?
			}

			let result = self
				.rsmq
				.create_queue(
					self.config.redis.messages_queue(state_machine).as_str(),
					Some(Duration::ZERO),
					Some(Duration::ZERO),
					Some(-1),
				)
				.await;

			if !(matches!(result, Ok(_) | Err(RsmqError::QueueExists))) {
				result.context(format!("Failed to create messages queue for {state_machine}"))?
			}
		}

		Ok(())
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
			Prover::Naive(ref naive) => {
				let message: BeefyConsensusProof =
					naive.consensus_proof(signed_commitment).await?.into();
				AbiEncode::encode(message)
			},
			Prover::ZK(ref zk) => {
				let message = zk.consensus_proof(signed_commitment, consensus_state).await?;
				let encoded = AbiEncode::encode(message);
				encoded
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

								let mandatory_queue =
									self.config.redis.mandatory_queue(&state_machine);
								self.rsmq
									.send_message(
										mandatory_queue.as_str(),
										message.clone(),
										Some(Duration::ZERO),
									)
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
							self.connection
								.set::<_, _, ()>(
									REDIS_CONSENSUS_STATE_KEY,
									self.consensus_state.encode(),
								)
								.await?;
						}
					}

					let (latest_parachain_height, messages) = self
						.latest_ismp_message_events(latest_beefy_header.parent_hash.into())
						.await?;

					if messages.is_empty() {
						self.consensus_state.finalized_parachain_height = latest_parachain_height;
						self.connection
							.set::<_, _, ()>(
								REDIS_CONSENSUS_STATE_KEY,
								self.consensus_state.encode(),
							)
							.await?;
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
						self.rsmq
							.send_message(
								self.config.redis.messages_queue(&state_machine).as_str(),
								message.clone(),
								Some(Duration::ZERO),
							)
							.await?; // fatal
					}

					self.consensus_state.inner.latest_beefy_height = *latest_beefy_header.number();
					self.consensus_state.finalized_parachain_height = latest_parachain_height;
					self.connection
						.set::<_, _, ()>(REDIS_CONSENSUS_STATE_KEY, self.consensus_state.encode())
						.await?;
				}

				#[allow(unreachable_code)]
				Ok::<_, anyhow::Error>(())
			};

			if let Err(err) = future.await {
				tracing::info!("Prover error: {err:?}");
				if let Some(RsmqError::RedisError(redis_error)) = err.downcast_ref::<RsmqError>() {
					if redis_error.is_connection_dropped() {
						tracing::info!("Recreating rsmq client");

						self.rsmq = redis_utils::rsmq_client(&self.config.redis)
							.await
							.expect("Failed to reconnect to redis");

						tracing::info!("Recreated rsmq client");
					} else {
						tracing::error!("Unhandled error {redis_error:?}")
					}
				}

				if let Some(redis_error) = err.downcast_ref::<RedisError>() {
					if redis_error.is_connection_dropped() {
						tracing::info!("Recreating redis client");

						self.connection = redis::Client::open(redis::ConnectionInfo {
							addr: self.config.redis.clone().into(),
							redis: redis::RedisConnectionInfo {
								db: self.config.redis.db as i64,
								username: self.config.redis.username.clone(),
								password: self.config.redis.password.clone(),
							},
						})
						.expect("Failed to reconnect to redis")
						.get_connection_manager()
						.await
						.expect("Failed to reconnect to redis");

						tracing::info!("Recreated redis client");
					} else {
						tracing::error!("Unhandled error {redis_error:?}")
					}
				}
			}
		}
	}
}

/// Beefy prover, can either produce zk proofs or naive proofs
#[derive(Clone)]
pub enum Prover<R: subxt::Config, P: subxt::Config> {
	// The naive prover
	Naive(beefy_prover::Prover<R, P>),
	// zk prover
	ZK(zk_beefy::Prover<R, P>),
}

impl<R, P> Prover<R, P>
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

		let prover = if config.zk_beefy {
			Prover::ZK(zk_beefy::Prover::new(prover))
		} else {
			Prover::Naive(prover)
		};

		Ok(prover)
	}

	/// Return the inner prover
	pub fn inner(&self) -> &beefy_prover::Prover<R, P> {
		match self {
			Prover::ZK(ref p) => &p.inner,
			Prover::Naive(ref p) => p,
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

#[cfg(test)]
mod tests {
	use futures::{StreamExt, TryStreamExt};
	use redis_async::client::pubsub_connect;
	use rsmq_async::{Rsmq, RsmqConnection, RsmqError, RsmqOptions};
	use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};

	use ismp::host::StateMachine;
	use substrate_state_machine::HashAlgorithm;
	use tesseract_substrate::{
		config::{Blake2SubstrateChain, KeccakSubstrateChain},
		SubstrateConfig,
	};

	use crate::host::{BeefyHost, BeefyHostConfig};

	use super::*;

	/// Sets up logging through tracing-subscriber
	pub fn setup_logging() -> Result<(), anyhow::Error> {
		let filter = tracing_subscriber::EnvFilter::from_default_env()
			.add_directive(LevelFilter::INFO.into());
		tracing_subscriber::fmt().with_env_filter(filter).finish().try_init()?;

		Ok(())
	}

	#[tokio::test]
	#[ignore]
	async fn integration_test_prover_and_redis_queues() -> Result<(), anyhow::Error> {
		// set up tracing
		setup_logging()?;

		let substrate_config = SubstrateConfig {
			state_machine: StateMachine::Kusama(4009),
			hashing: Some(HashAlgorithm::Keccak),
			consensus_state_id: None,
			// rpc_ws: "wss://hyperbridge-paseo-rpc.blockops.network:443".to_string(),
			rpc_ws: "ws://localhost:9902".to_string(),
			max_rpc_payload_size: None,
			signer: format!("{:?}", H256::random()),
			initial_height: None,
			max_concurent_queries: None,
			poll_interval: None,
			fee_token_decimals: None,
		};
		let substrate_client =
			SubstrateClient::<KeccakSubstrateChain>::new(substrate_config.clone()).await?;

		let prover_config = ProverConfig {
			relay_rpc_ws: "wss://hyperbridge-paseo-relay.blockops.network:443".to_string(),
			para_rpc_ws: substrate_config.rpc_ws.clone(),
			para_ids: vec![4009],
			zk_beefy: false,
			max_rpc_payload_size: None,
			query_batch_size: None,
		};
		let prover = Prover::new(prover_config).await?;

		let redis = RedisConfig {
			db: 0,
			mandatory_queue: "mandatory".into(),
			messages_queue: "messages".into(),
			ns: "rsmq".into(),
			url: "localhost".into(),
			port: 6379,
			password: None,
			username: None,
			// will be adjusted by the relevant subsystems
			realtime: false,
		};
		let beefy_config = BeefyProverConfig {
			minimum_finalization_height: 25,
			redis: redis.clone(),
			consensus_state_id: [0u8; 4],
			state_machines: vec![
				StateMachine::Polkadot(2000),
				StateMachine::Polkadot(2001),
				StateMachine::Polkadot(2002),
			],
		};

		let beefy_host_config = BeefyHostConfig {
			redis: redis.clone(),
			consensus_state_id: beefy_config.consensus_state_id.clone(),
		};
		let beefy_host =
			BeefyHost::new(beefy_host_config, prover.clone(), substrate_client.clone()).await?;

		// create all the queues
		for state_machine in beefy_config.state_machines.iter() {
			// don't really care about errors
			let result = beefy_host
				.rsmq()
				.lock()
				.await
				.create_queue(
					redis.mandatory_queue(state_machine).as_str(),
					Some(Duration::ZERO),
					Some(Duration::ZERO),
					Some(-1),
				)
				.await;

			tracing::error!("mandatory queue create result for {state_machine}: {result:?}");

			let result = beefy_host
				.rsmq()
				.lock()
				.await
				.create_queue(
					redis.messages_queue(state_machine).as_str(),
					Some(Duration::ZERO),
					Some(Duration::ZERO),
					Some(-1),
				)
				.await;
			tracing::error!("messages queue create result: {result:?}");
		}

		// listen for queue notifications
		let mut notifications = {
			let pubsub = beefy_host.pubsub.clone();
			let stream_0 = {
				let state_machine_0 = beefy_config.state_machines[0].clone();
				beefy_host.queue_notifications(state_machine_0, &pubsub).await?.map_ok(
					move |message| log::info!("{state_machine_0} Got stream message {message:?}",),
				)
			};
			let stream_1 = {
				let state_machine_1 = beefy_config.state_machines[1].clone();

				beefy_host.queue_notifications(state_machine_1, &pubsub).await?.map_ok(
					move |message| log::info!("{state_machine_1} Got stream message {message:?}",),
				)
			};
			let stream_2 = {
				let state_machine_2 = beefy_config.state_machines[2].clone();

				beefy_host.queue_notifications(state_machine_2, &pubsub).await?.map_ok(
					move |message| log::info!("{state_machine_2} Got stream message {message:?}",),
				)
			};

			futures::stream::select(futures::stream::select(stream_0, stream_1), stream_2)
		};
		tracing::info!("Created queue notifications");

		// set consensus state on redis
		#[cfg(feature = "new-consensus-state")]
		{
			use hex_literal::hex;
			let _ancient_hash = H256::from(hex!(
				"6b33f31d9a5e46d0d735926a29e2293934db4acb785432af3184ede3107aa7b0"
			));
			let prover_consensus_state = prover.query_initial_consensus_state(None).await?;
			let mut connection = redis::Client::open(redis::ConnectionInfo {
				addr: redis::ConnectionAddr::Tcp(redis.url.clone(), redis.port),
				redis: redis::RedisConnectionInfo {
					db: redis.db as i64,
					username: redis.username.clone(),
					password: redis.password.clone(),
				},
			})?
			.get_connection_manager()
			.await?;
			connection
				.set(REDIS_CONSENSUS_STATE_KEY, prover_consensus_state.encode())
				.await?;
			tracing::info!("Wrote consensus state to redis: {prover_consensus_state:#?}");
		};

		let mut beefy_prover = BeefyProver::<Blake2SubstrateChain, _>::new(
			beefy_config.clone(),
			substrate_client.clone(),
			prover.clone(),
		)
		.await?;

		// spawn prover
		tokio::spawn(async move { beefy_prover.run().await });
		tracing::info!("Spawned prover");

		// start listening
		while let Some(_item) = notifications.next().await {}

		Ok(())
	}

	#[tokio::test]
	#[ignore]
	async fn test_redis_queues() -> Result<(), anyhow::Error> {
		let mut options = RsmqOptions::default();
		options.realtime = true;
		let mut rsmq = Rsmq::new(options).await.expect("connection failed");

		let queue = "myqueue2";
		if let Err(RsmqError::QueueExists) =
			rsmq.create_queue(queue, Some(Duration::ZERO), None, Some(-1)).await
		{
			println!("Queue already exists")
		}

		let pub_sub = pubsub_connect("localhost", 6379).await?;

		let mut stream = pub_sub.subscribe(&format!("rsmq:rt:{queue}")).await?;

		for i in 0..5 {
			rsmq.send_message(queue, format!("testmessage-{i}"), None)
				.await
				.expect("failed to send message");

			tokio::time::sleep(Duration::from_secs(1)).await;
		}

		let mut count = 0;

		while let Some(value) = stream.next().await {
			println!("Got item from stream {value:?}");
			count += 1;
			if count == 5 {
				break;
			}
		}

		count = 0;

		while let Some(message) = rsmq
			.receive_message::<String>(queue, Some(Duration::ZERO))
			.await
			.expect("cannot receive message")
		{
			count += 1;
			println!("Got: {message:?}, count: {count}");

			if count >= 10 {
				println!("Deleting {}", &message.id);
				rsmq.delete_message(queue, &message.id).await?;
			}
			// tokio::time::sleep(Duration::from_secs(1)).await;
		}

		println!("Ok done");

		Ok(())
	}
}
