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

use std::{
	pin::Pin,
	sync::Arc,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use codec::{Decode, Encode};
use futures::{stream::TryStreamExt, Stream, StreamExt};
use redis::AsyncCommands;
use redis_async::client::PubsubConnection;
use rsmq_async::{RedisBytes, Rsmq, RsmqConnection, RsmqError, RsmqMessage};
use serde::{Deserialize, Serialize};
use subxt::{
	config::{ExtrinsicParams, HashFor},
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature, H256},
};
use tokio::sync::Mutex;

use beefy_verifier_primitives::ConsensusState;
use ismp::{
	consensus::ConsensusStateId,
	host::StateMachine,
	messaging::{ConsensusMessage, CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_solidity_abi::beefy::BeefyConsensusState;
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tesseract_substrate::SubstrateClient;

use crate::{
	prover::{query_parachain_header, Prover, ProverConsensusState, REDIS_CONSENSUS_STATE_KEY},
	redis_utils::{self, RedisConfig},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyHostConfig {
	/// Redis configuration for message queues
	#[serde(flatten)]
	pub redis: RedisConfig,
	/// Consensus state id for the host on the counterparty
	pub consensus_state_id: ConsensusStateId,
}

/// The beefy host is responsible for receiving BEEFY proofs from the redis queue and submitting
/// them to the counterparty.
pub struct BeefyHost<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
{
	/// PubSub connection for receiving notifications when there are new proofs in the queue
	pub(crate) pubsub: PubsubConnection,
	/// Rsmq for interacting with the queue
	rsmq: Arc<Mutex<Rsmq>>,
	/// Host configuration options
	config: BeefyHostConfig,
	/// Consensus prover
	prover: Prover<R, P>,
	/// The underlying substrate client
	pub client: SubstrateClient<P>,
}

impl<R, P> BeefyHost<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
	P: subxt::Config + Send + Sync + Clone,
	<P::ExtrinsicParams as ExtrinsicParams<P>>::Params: Send + Sync + DefaultParams,
	P::Signature: From<MultiSignature> + Send + Sync,
	P::AccountId: From<AccountId32> + Into<P::Address> + Clone + 'static + Send + Sync,
	H256: From<HashFor<P>>,
{
	/// Construct an implementation of the [`BeefyHost`]
	pub async fn new(
		mut config: BeefyHostConfig,
		prover: Prover<R, P>,
		client: SubstrateClient<P>,
	) -> Result<Self, anyhow::Error> {
		let pubsub = redis_utils::pubsub_client(&config.redis).await?;
		// we will not be pushing messages to the queue in the host
		config.redis.realtime = false;
		let rsmq = Arc::new(Mutex::new(redis_utils::rsmq_client(&config.redis).await?));

		Ok(BeefyHost { pubsub, rsmq, prover, client, config })
	}

	/// Initialize all the relevant queues for the configured state machines.
	pub async fn init_queues(
		&self,
		state_machines: Vec<StateMachine>,
	) -> Result<(), anyhow::Error> {
		for state_machine in state_machines.iter() {
			let mut rsmq = self.rsmq.lock().await;
			let result = rsmq
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

			let result = rsmq
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

	/// Construct notifications for the queue for the given counterparty state machine.
	pub async fn queue_notifications(
		&self,
		counterparty_state_machine: StateMachine,
		pubsub: &PubsubConnection,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<StreamMessage, redis_async::error::Error>> + Send>>,
		anyhow::Error,
	> {
		let mandatory_queue = self.config.redis.mandatory_queue(&counterparty_state_machine);
		let messages_queue = self.config.redis.messages_queue(&counterparty_state_machine);

		let mandatory_stream = {
			pubsub
				.subscribe(&format!("{}:rt:{mandatory_queue}", self.config.redis.ns))
				.await? // fatal error
				.map_ok(|_item| StreamMessage::EpochChanged)
		};
		let messages_stream = {
			pubsub
				.subscribe(&format!("{}:rt:{messages_queue}", self.config.redis.ns))
				.await? // fatal error
				.map_ok(|_item| StreamMessage::NewMessages)
		};
		let combined = futures::stream::select(mandatory_stream, messages_stream);
		let yield_once = futures::stream::iter(vec![Ok(StreamMessage::EpochChanged)]);
		Ok(Box::pin(yield_once.chain(combined)))
	}

	/// Initialize the consensus state for the prover where it expects in redis, then returns it.
	pub async fn hydrate_initial_consensus_state(
		&self,
		consensus_state: Option<ConsensusState>,
	) -> Result<CreateConsensusState, anyhow::Error> {
		use ethers::abi::AbiEncode;
		let prover_consensus_state = match consensus_state {
			Some(state) => {
				let inner = self.prover.inner();
				let hash = inner
					.relay_rpc
					.chain_get_block_hash(Some(state.latest_beefy_height.into()))
					.await?
					.ok_or_else(|| {
						anyhow!("Failed to find block hash for num: {}", state.latest_beefy_height)
					})?;
				let para_header =
					query_parachain_header(&inner.relay_rpc, hash, inner.para_ids[0]).await?;

				ProverConsensusState {
					inner: state,
					finalized_parachain_height: para_header.number.into(),
				}
			},
			None => self.prover.query_initial_consensus_state(None).await?,
		};
		let consensus_state: BeefyConsensusState = prover_consensus_state.clone().inner.into();

		let mut connection = redis::Client::open(redis::ConnectionInfo {
			addr: self.config.redis.clone().into(),
			redis: redis::RedisConnectionInfo {
				db: self.config.redis.db as i64,
				username: self.config.redis.username.clone(),
				password: self.config.redis.password.clone(),
			},
		})?
		.get_connection_manager()
		.await?;
		connection
			.set::<_, _, ()>(REDIS_CONSENSUS_STATE_KEY, prover_consensus_state.encode())
			.await?;

		let start = SystemTime::now();
		let timestamp = start.duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
		Ok(CreateConsensusState {
			consensus_state: consensus_state.encode(),
			consensus_client_id: *b"BEEF",
			consensus_state_id: self.config.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_periods: vec![(self.client.state_machine_id().state_id, 5 * 60)]
				.into_iter()
				.collect(),
			state_machine_commitments: vec![(
				self.client.state_machine_id(),
				StateCommitmentHeight {
					height: prover_consensus_state.finalized_parachain_height,
					commitment: ismp::consensus::StateCommitment {
						timestamp,
						overlay_root: Some(H256::zero()),
						state_root: H256::zero(),
					},
				},
			)],
		})
	}

	/// Retuns a reference to underlying [`Rsmq`] instance
	pub fn rsmq(&self) -> Arc<Mutex<Rsmq>> {
		self.rsmq.clone()
	}

	/// Retuns a reference to underlying [`SubstrateClient`] instance
	pub fn client(&self) -> &SubstrateClient<P> {
		&self.client
	}
}

/// Convenience enum for the queue message kinds
#[derive(Debug, Eq, PartialEq)]
pub enum StreamMessage {
	/// The current authority set has handed over to the next. This is neccessary so that light
	/// clients can follow the chain
	EpochChanged,
	/// Some new messages can now be finalized.
	NewMessages,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ConsensusProof {
	/// The height that is now finalized by this consensus message
	pub finalized_height: u32,
	/// The validator set id responsible for signing this message
	pub set_id: u64,
	/// The consensus message in question
	pub message: ConsensusMessage,
}

impl TryFrom<RedisBytes> for ConsensusProof {
	type Error = Vec<u8>;

	fn try_from(value: RedisBytes) -> Result<Self, Self::Error> {
		let bytes = value.into_bytes();
		Self::decode(&mut &bytes[..]).map_err(|err| {
			tracing::error!("Failed to decode ConsensusProof: {err:?}");
			bytes
		})
	}
}

impl Into<RedisBytes> for ConsensusProof {
	fn into(self) -> RedisBytes {
		self.encode().into()
	}
}

#[async_trait::async_trait]
impl<R, P> IsmpHost for BeefyHost<R, P>
where
	R: subxt::Config + Send + Sync + Clone,
	P: subxt::Config + Send + Sync + Clone,
	<P::ExtrinsicParams as ExtrinsicParams<P>>::Params: Send + Sync + DefaultParams,
	P::Signature: From<MultiSignature> + Send + Sync,
	P::AccountId: From<AccountId32> + Into<P::Address> + Clone + 'static + Send + Sync,
	H256: From<HashFor<P>>,
{
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let counterparty_state_machine = counterparty.state_machine_id().state_id;
		let mandatory_queue = self.config.redis.mandatory_queue(&counterparty_state_machine);
		let messages_queue = self.config.redis.messages_queue(&counterparty_state_machine);
		let mut pubsub = self.pubsub.clone();
		let mut notifications =
			self.queue_notifications(counterparty_state_machine, &pubsub).await?;

		// this will yield whenever the prover writes to either the mandatory or messages queue
		while let Some(item) = notifications.next().await {
			let Ok(ref message) = item else {
				let error = item.unwrap_err();
				tracing::error!("Error in redis pubsub stream: {:?}", error); // non-fatal error
				if matches!(
					error,
					redis_async::error::Error::Connection(_) | redis_async::error::Error::IO(_)
				) {
					// if connection error, reconnect & resubscribe
					pubsub = redis_utils::pubsub_client(&self.config.redis).await?;
					notifications = self
						.queue_notifications(counterparty.state_machine_id().state_id, &pubsub)
						.await?;
				}
				continue;
			};

			if *message == StreamMessage::EpochChanged {
				// try to consume all mandatory updates
				loop {
					let item = self
						.rsmq
						.lock()
						.await
						.receive_message::<ConsensusProof>(&mandatory_queue, None)
						.await;

					let RsmqMessage { id, message: ConsensusProof { message, set_id, .. }, .. } =
						match item {
							Ok(Some(message)) => message,
							// no new items in the queue, continue to process messages queue
							Ok(None) => break,
							Err(err) => {
								tracing::error!(
									"{counterparty_state_machine} error pulling from mandatory queue: {err:?}"
								);
								// non-fatal error, keep trying
								continue;
							},
						};

					tracing::info!("{counterparty_state_machine} got authority set handover proof for {set_id}");
					let encoded = counterparty
						.query_consensus_state(None, self.config.consensus_state_id)
						.await
						.context("Could not fetch consenus state")?; // somewhat fatal
					let consensus_state = ConsensusState::decode(&mut &encoded[..])
						.expect("Infallible, consensus state was encoded correctly");

					// just some sanity checks
					if set_id < consensus_state.next_authorities.id {
						tracing::error!(
							"{counterparty_state_machine} got proof with set_id: {set_id} < next_set_id:{}",
							consensus_state.next_authorities.id
						);
						self.rsmq.lock().await.delete_message(&mandatory_queue, &id).await?; // this would be a fatal error
						continue;
					}

					// just some sanity checks
					if set_id != consensus_state.next_authorities.id {
						tracing::error!(
							"{counterparty_state_machine} consensus proof with set_id: {set_id} does not match next_set_id: {}",
							consensus_state.next_authorities.id
						);
						// try to pull something else
						continue;
					}

					if let Err(err) = counterparty
						.submit(
							vec![Message::Consensus(message)],
							self.client.state_machine_id().state_id,
						)
						.await
					{
						tracing::error!(
							"Error submitting consensus message to {counterparty_state_machine}: {err:?}",
						);

						// non-fatal error, keep trying. This will pull it from the queue once more
						continue;
					};

					tracing::info!(
						"Submitted mandatory proof to {counterparty_state_machine} for {set_id}"
					);
					// this would be a fatal error
					self.rsmq.lock().await.delete_message(&mandatory_queue, &id).await?;
				}
			}

			// must be for a new message, try to consume all updates.
			loop {
				let item = self
					.rsmq
					.lock()
					.await
					.receive_message::<ConsensusProof>(&messages_queue, None)
					.await;

				let RsmqMessage {
					id,
					message: ConsensusProof { message, finalized_height, set_id },
					..
				} = match item {
					Ok(Some(message)) => message,
					Ok(None) => break, // no new items in the queue
					Err(err) => {
						tracing::error!("{counterparty_state_machine} error pulling from messages queue: {err:?}");
						// non-fatal error, keep trying
						continue;
					},
				};

				tracing::info!(
					"{counterparty_state_machine} got messages proof for {finalized_height}"
				);
				let encoded = counterparty
					.query_consensus_state(None, self.config.consensus_state_id)
					.await?; // somewhat fatal
				let consensus_state = ConsensusState::decode(&mut &encoded[..])
					.expect("Infallible, consensus state was encoded correctly");

				// check if the update is relevant to us.
				if consensus_state.latest_beefy_height >= finalized_height {
					tracing::info!(
						"{counterparty_state_machine} saw proof for stale height {finalized_height}, current: {}",
						consensus_state.latest_beefy_height
					);
					// delete the message and pull another one
					self.rsmq.lock().await.delete_message(&messages_queue, &id).await?; // this would be a fatal error
					continue;
				}

				if set_id != consensus_state.current_authorities.id &&
					set_id != consensus_state.next_authorities.id
				{
					tracing::info!(
						"{counterparty_state_machine} saw proof for unknown set_id {set_id}, current: {}, next: {}",
						consensus_state.current_authorities.id,
						consensus_state.next_authorities.id,
					);

					if set_id > consensus_state.next_authorities.id {
						tracing::info!(
							"{counterparty_state_machine} proof was for future set: {set_id}, next: {}",
							consensus_state.next_authorities.id,
						);
						// break so that we can process a mandatory update
						break;
					} else if set_id < consensus_state.current_authorities.id {
						tracing::info!(
							"{counterparty_state_machine} proof was for older set: {set_id}, current: {}",
							consensus_state.current_authorities.id,
						);
						self.rsmq.lock().await.delete_message(&mandatory_queue, &id).await?; // this would be a fatal error
						continue; // move on to the next proof
					}
				}

				if let Err(err) = counterparty
					.submit(
						vec![Message::Consensus(message)],
						self.client.state_machine_id().state_id,
					)
					.await
				{
					tracing::error!(
						"Error submitting consensus message to {counterparty_state_machine}: {err:?}",
					);
					// non-fatal error, keep trying. This will pull it from the queue once more
					continue;
				};

				self.rsmq.lock().await.delete_message(&mandatory_queue, &id).await?; // this would be a fatal
				                                                         // error
			}
		}

		Ok(())
	}

	/// Queries the consensus state at the latest height
	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		use ethers::abi::AbiEncode;
		let consensus_state: BeefyConsensusState =
			self.prover.query_initial_consensus_state(None).await?.inner.into();

		Ok(Some(CreateConsensusState {
			consensus_state: consensus_state.encode(),
			consensus_client_id: *b"BEEF",
			consensus_state_id: self.config.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_periods: vec![(self.client.state_machine_id().state_id, 5 * 60)]
				.into_iter()
				.collect(),
			state_machine_commitments: vec![],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		Arc::new(self.client.clone())
	}
}
