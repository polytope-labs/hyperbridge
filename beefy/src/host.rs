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

use beefy_verifier_primitives::ConsensusState;
use codec::{Decode, Encode};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sp_core::H256;
use std::{pin::Pin, sync::Arc};
use subxt::{
	config::{extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams},
	ext::sp_runtime::MultiSignature,
};
use tesseract_substrate::SubstrateClient;

use crate::{
	prover::{Prover, REDIS_CONSENSUS_STATE_KEY},
	rsmq::{self, RedisConfig},
};
use futures::{stream::TryStreamExt, Stream, StreamExt};
use ismp::{
	consensus::ConsensusStateId,
	host::StateMachine,
	messaging::{ConsensusMessage, CreateConsensusState, Message},
};
use redis_async::client::{ConnectionBuilder, PubsubConnection};
use rsmq_async::{RedisBytes, Rsmq, RsmqConnection, RsmqMessage};
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyHostConfig {
	/// Redis configuration for message queues
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
	pubsub: PubsubConnection,
	/// Rsmq for interacting with the queue
	rsmq: Arc<Mutex<Rsmq>>,
	/// Host configuration options
	config: BeefyHostConfig,
	/// Consensus prover
	prover: Prover<R, P>,
	/// The underlying substrate client
	pub(crate) client: SubstrateClient<P>,
}

impl<R, P> BeefyHost<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
{
	/// Construct an implementation of the [`BeefyHost`]
	pub async fn new(
		mut config: BeefyHostConfig,
		prover: Prover<R, P>,
		client: SubstrateClient<P>,
	) -> Result<Self, anyhow::Error> {
		let mut builder = ConnectionBuilder::new(&config.redis.url, config.redis.port)?;
		if let Some(ref username) = config.redis.username {
			builder.username(username.as_str());
		}
		if let Some(ref password) = config.redis.password {
			builder.password(password.as_str());
		}
		if config.redis.tls {
			builder.tls();
		}
		let pubsub = builder.pubsub_connect().await?;
		// we will not be pushing messages to the queue in the host
		config.redis.realtime = false;
		let rsmq = Arc::new(Mutex::new(rsmq::client(&config.redis).await?));

		Ok(BeefyHost { pubsub, rsmq, prover, client, config })
	}

	/// Construct notifications for the queue for the given counterparty state machine.
	pub async fn queue_notifications(
		&self,
		counterparty_state_machine: StateMachine,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<StreamMessage, redis_async::error::Error>> + Send>>,
		anyhow::Error,
	> {
		let mandatory_queue = self.config.redis.mandatory_queue(&counterparty_state_machine);
		let messages_queue = self.config.redis.messages_queue(&counterparty_state_machine);

		let mandatory_stream = {
			self.pubsub
				.subscribe(&format!("{}:rt:{mandatory_queue}", self.config.redis.ns))
				.await? // fatal error
				.map_ok(|_item| StreamMessage::EpochChanged)
		};
		let messages_stream = {
			self.pubsub
				.subscribe(&format!("{}:rt:{messages_queue}", self.config.redis.ns))
				.await? // fatal error
				.map_ok(|_item| StreamMessage::NewMessages)
		};

		let combined = futures::stream::select(mandatory_stream, messages_stream);

		Ok(Box::pin(combined))
	}

	/// Initialize the consensus state for the prover where it expects in redis, then returns it.
	pub async fn hydrate_initial_consensus_state(
		&self,
	) -> Result<CreateConsensusState, anyhow::Error> {
		let consensus_state = self.prover.query_initial_consensus_state(None).await?;
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
		connection.set(REDIS_CONSENSUS_STATE_KEY, consensus_state.encode()).await?;

		Ok(CreateConsensusState {
			consensus_state: consensus_state.inner.encode(),
			consensus_client_id: *b"BEEF",
			consensus_state_id: self.config.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_period: 5 * 60,
			state_machine_commitments: vec![],
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
	<P::ExtrinsicParams as ExtrinsicParams<P::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<P, PlainTip>>,
	P::Signature: From<MultiSignature> + Send + Sync,
	P::AccountId:
		From<sp_core::crypto::AccountId32> + Into<P::Address> + Clone + 'static + Send + Sync,
	H256: From<<P as subxt::Config>::Hash>,
{
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let counterparty_state_machine = counterparty.state_machine_id().state_id;
		let mandatory_queue = self.config.redis.mandatory_queue(&counterparty_state_machine);
		let messages_queue = self.config.redis.messages_queue(&counterparty_state_machine);
		let mut notifications = self.queue_notifications(counterparty_state_machine).await?;

		// this will yield whenever the prover writes to either the mandatory or messages queue
		while let Some(item) = notifications.next().await {
			let Ok(ref message) = item else {
				let error = item.unwrap_err();
				tracing::error!("Error in redis pubsub stream: {:?}", error); // non-fatal error
				if matches!(error, redis_async::error::Error::Connection(_)) {
					// if connection error, resubscribe
					notifications =
						self.queue_notifications(counterparty.state_machine_id().state_id).await?;
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
									"{counterparty_state_machine:?} error pulling from mandatory queue: {err:?}"
								);
								// non-fatal error, keep trying
								continue;
							},
						};

					tracing::info!("{counterparty_state_machine:?} got authority set handover proof for {set_id}");
					let encoded = counterparty
						.query_consensus_state(None, self.config.consensus_state_id)
						.await?; // somewhat fatal
					let consensus_state = ConsensusState::decode(&mut &encoded[..])
						.expect("Infallible, consensus state was encoded correctly");

					// just some sanity checks
					if set_id < consensus_state.next_authorities.id {
						tracing::error!(
							"{counterparty_state_machine:?} got proof with set_id: {set_id} < next_set_id:{}",
							consensus_state.next_authorities.id
						);
						self.rsmq.lock().await.delete_message(&mandatory_queue, &id).await?; // this would be a fatal error
						continue;
					}

					// just some sanity checks
					if set_id != consensus_state.next_authorities.id {
						tracing::error!(
							"{counterparty_state_machine:?} consensus proof with set_id: {set_id} does not match next_set_id: {}",
							consensus_state.next_authorities.id
						);
						// try to pull something else
						continue;
					}

					if let Err(err) = counterparty.submit(vec![Message::Consensus(message)]).await {
						tracing::error!(
							"Error submitting consensus message to {counterparty_state_machine:?}: {err:?}",
						);
						// non-fatal error, keep trying. This will pull it from the queue once more
						continue;
					};
					tracing::info!(
						"Submitted mandatory proof to {counterparty_state_machine:?} for {set_id}"
					);
					self.rsmq.lock().await.delete_message(&mandatory_queue, &id).await?; // this would be a fatal error
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
						tracing::error!("{counterparty_state_machine:?} error pulling from messages queue: {err:?}");
						// non-fatal error, keep trying
						continue;
					},
				};

				tracing::info!(
					"{counterparty_state_machine:?} got messages proof for {finalized_height}"
				);
				let encoded = counterparty
					.query_consensus_state(None, self.config.consensus_state_id)
					.await?; // somewhat fatal
				let consensus_state = ConsensusState::decode(&mut &encoded[..])
					.expect("Infallible, consensus state was encoded correctly");

				// check if the update is relevant to us.
				if consensus_state.latest_beefy_height >= finalized_height {
					tracing::info!(
						"{counterparty_state_machine:?} saw proof for stale height {finalized_height}, current: {}",
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
						"{counterparty_state_machine:?} saw proof for unknown set_id {set_id}, current: {}, next: {}",
						consensus_state.current_authorities.id,
						consensus_state.next_authorities.id,
					);

					if set_id > consensus_state.next_authorities.id {
						tracing::info!(
							"{counterparty_state_machine:?} proof was for future set: {set_id}, next: {}",
							consensus_state.next_authorities.id,
						);
						// break so that we can process a mandatory update
						break;
					} else if set_id < consensus_state.current_authorities.id {
						tracing::info!(
							"{counterparty_state_machine:?} proof was for older set: {set_id}, current: {}",
							consensus_state.current_authorities.id,
						);
						self.rsmq.lock().await.delete_message(&mandatory_queue, &id).await?; // this would be a fatal error
						continue; // move on to the next proof
					}
				}

				if let Err(err) = counterparty.submit(vec![Message::Consensus(message)]).await {
					tracing::error!(
						"Error submitting consensus message to {counterparty_state_machine:?}: {err:?}",
					);
					// non-fatal error, keep trying. This will pull it from the queue once more
					continue;
				};

				self.rsmq.lock().await.delete_message(&mandatory_queue, &id).await?; // this would be a fatal error
			}
		}

		Ok(())
	}

	/// Queries the consensus state at the latest height
	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		let consensus_state = self.prover.query_initial_consensus_state(None).await?.inner;

		Ok(Some(CreateConsensusState {
			consensus_state: consensus_state.encode(),
			consensus_client_id: *b"BEEF",
			consensus_state_id: self.config.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_period: 5 * 60,
			state_machine_commitments: vec![],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		Arc::new(self.client.clone())
	}
}
