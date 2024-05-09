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
use std::sync::Arc;

use crate::prover::Prover;
use futures::{stream::TryStreamExt, StreamExt};
use ismp::{
	consensus::ConsensusStateId,
	events::StateMachineUpdated,
	messaging::{ConsensusMessage, CreateConsensusState, Message},
};
use redis_async::client::{ConnectionBuilder, PubsubConnection};
use rsmq_async::{RedisBytes, Rsmq, RsmqConnection, RsmqMessage, RsmqOptions};
use tesseract_primitives::{ByzantineHandler, IsmpHost, IsmpProvider};

pub struct BeefyHostConfig {
	/// Redis configuration for message queues
	pub redis: RedisConfig,
	/// Consensus state id for the host on the counterparty
	pub consensus_state_id: ConsensusStateId,
}

pub struct RedisConfig {
	/// Redis host
	pub url: String,
	/// Redis port
	pub port: u16,
	/// Redis username
	pub username: Option<String>,
	/// Redis password
	pub password: Option<String>,
	/// Redis db
	pub db: u8,
	/// RSMQ namespace (you can have several. "rsmq" by default)
	pub ns: String,
	/// Queue name for mandatory consensus proofs
	pub mandatory_queue: String,
	/// Queue name for messages consensus proofs
	pub messages_queue: String,
}

pub struct BeefyHost<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
{
	/// PubSub connection for receiving notifications when there are new proofs in the queueu
	pubsub: PubsubConnection,
	/// Rsmq for interacting with the queue
	rsmq: Rsmq,
	/// Config options for redis
	redis: RedisConfig,
	/// Consensus state id for the host on the counterparty
	consensus_state_id: ConsensusStateId,
	/// Consensus prover
	prover: Prover<R, P>,
}

impl<R, P> BeefyHost<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
{
	/// Construct an implementation of the [`BeefyHost`]
	pub async fn new(config: BeefyHostConfig, prover: Prover<R, P>) -> Result<Self, anyhow::Error> {
		let mut builder = ConnectionBuilder::new(&config.redis.url, config.redis.port)?;
		if let Some(ref username) = config.redis.username {
			builder.username(username.as_str());
		}
		if let Some(ref password) = config.redis.password {
			builder.password(password.as_str());
		}
		let pubsub = builder.pubsub_connect().await?;

		let options = RsmqOptions {
			host: config.redis.url.clone(),
			port: config.redis.port.clone(),
			username: config.redis.username.clone(),
			password: config.redis.password.clone(),
			db: config.redis.db.clone(),
			ns: config.redis.ns.clone(),
			// we will not be publishing messages here
			realtime: false,
		};
		let rsmq = Rsmq::new(options).await?;

		Ok(BeefyHost {
			pubsub,
			rsmq,
			redis: config.redis,
			prover,
			consensus_state_id: config.consensus_state_id,
		})
	}
}

/// Convenience enum for the queue message kinds
#[derive(Debug, Eq, PartialEq)]
enum StreamMessage {
	/// The current authority set has handed over to the next. This is neccessary so that light
	/// clients can follow the chain
	EpochChanged,
	/// Some new messages can now be finalized.
	NewMessages,
}

#[derive(Encode, Decode)]
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

#[async_trait::async_trait]
impl<R, P> IsmpHost for BeefyHost<R, P>
where
	R: subxt::Config + Send,
	P: subxt::Config + Send,
{
	async fn start_consensus(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let counterpaty_state_machine = counterparty.state_machine_id().state_id.to_string();
		let mandatory_queue = format!("{}-{counterpaty_state_machine}", self.redis.mandatory_queue);
		let messages_queue = format!("{}-{counterpaty_state_machine}", self.redis.messages_queue);

		let mandatory_stream = {
			self.pubsub
				.subscribe(&format!("{}:rt:{mandatory_queue}", self.redis.ns))
				.await? // fatal error
				.map_ok(|_item| StreamMessage::EpochChanged)
		};
		let messages_stream = {
			self.pubsub
				.subscribe(&format!("{}:rt:{messages_queue}", self.redis.ns))
				.await? // fatal error
				.map_ok(|_item| StreamMessage::NewMessages)
		};
		let mut combined = futures::stream::select(mandatory_stream, messages_stream);

		// this will yield whenever the prover writes to either the mandatory or messages queue
		while let Some(item) = combined.next().await {
			let Ok(ref message) = item else {
				tracing::error!("Error in redis pubsub stream: {:?}", item.unwrap_err()); // non-fatal error
				continue
			};

			if *message == StreamMessage::EpochChanged {
				// try to consume all mandatory updates
				loop {
					let item =
						self.rsmq.receive_message::<ConsensusProof>(&mandatory_queue, None).await;

					let RsmqMessage { id, message: ConsensusProof { message, set_id, .. }, .. } =
						match item {
							Ok(Some(message)) => message,
							// no new items in the queue, continue to process messages queue
							Ok(None) => break,
							Err(err) => {
								tracing::error!(
									"Error pulling from queue {mandatory_queue}: {err:?}"
								);
								// non-fatal error, keep trying
								continue
							},
						};

					let encoded =
						counterparty.query_consensus_state(None, self.consensus_state_id).await?; // somewhat fatal
					let consensus_state = ConsensusState::decode(&mut &encoded[..])
						.expect("Infallible, consensus state was encoded correctly");

					// just some sanity checks
					if set_id != consensus_state.next_authorities.id {
						tracing::error!(
							"Invariant violated, consensus proof with set_id: {set_id} does not match next_set_id:{}",
							consensus_state.next_authorities.id
						);
						continue
					}

					if let Err(err) = counterparty.submit(vec![Message::Consensus(message)]).await {
						tracing::error!(
							"Error submitting consensus message to {}: {err:?}",
							counterparty.name()
						);
						// non-fatal error, keep trying. This will pull it from the queue once more
						continue
					};

					self.rsmq.delete_message(&mandatory_queue, &id).await?; // this would be a fatal error
				}
			}

			// must be for a new message, try to consume all updates.
			loop {
				let item = self.rsmq.receive_message::<ConsensusProof>(&messages_queue, None).await;

				let encoded =
					counterparty.query_consensus_state(None, self.consensus_state_id).await?; // somewhat fatal
				let consensus_state = ConsensusState::decode(&mut &encoded[..])
					.expect("Infallible, consensus state was encoded correctly");

				let RsmqMessage {
					id,
					message: ConsensusProof { message, finalized_height, set_id },
					..
				} = match item {
					Ok(Some(message)) => message,
					Ok(None) => break, // no new items in the queue
					Err(err) => {
						tracing::error!("Error pulling from queue {mandatory_queue}: {err:?}");
						// non-fatal error, keep trying
						continue
					},
				};

				// check if the update is relevant to us.
				if consensus_state.latest_beefy_height >= finalized_height {
					tracing::info!(
						"Saw proof for stale height {finalized_height}, current: {}",
						consensus_state.latest_beefy_height
					);
					// delete the message and pull another one
					self.rsmq.delete_message(&messages_queue, &id).await?; // this would be a fatal error
					continue
				}

				if set_id != consensus_state.current_authorities.id &&
					set_id != consensus_state.next_authorities.id
				{
					tracing::info!(
						"Saw proof for unknown set_id {set_id}, current: {}, next: {}",
						consensus_state.current_authorities.id,
						consensus_state.next_authorities.id,
					);

					if set_id > consensus_state.next_authorities.id {
						tracing::info!(
							"Proof was for future set: {set_id}, next: {}",
							consensus_state.next_authorities.id,
						);
						// break so that we can process a mandatory update
						break
					} else if set_id < consensus_state.current_authorities.id {
						tracing::info!(
							"Proof was for older set: {set_id}, current: {}",
							consensus_state.current_authorities.id,
						);
						self.rsmq.delete_message(&mandatory_queue, &id).await?; // this would be a fatal error
														// move on to the next proof
						continue
					}
				}

				if let Err(err) = counterparty.submit(vec![Message::Consensus(message)]).await {
					tracing::error!(
						"Error submitting consensus message to {}: {err:?}",
						counterparty.name()
					);
					// non-fatal error, keep trying. This will pull it from the queue once more
					continue
				};

				self.rsmq.delete_message(&mandatory_queue, &id).await?; // this would be a fatal error
			}
		}

		Ok(())
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		let consensus_state = self.prover.query_initial_consensus_state(None).await?;

		Ok(Some(CreateConsensusState {
			consensus_state: consensus_state.encode(),
			consensus_client_id: *b"BEEF",
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_period: 5 * 60,
			state_machine_commitments: vec![],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		todo!()
	}
}

#[async_trait::async_trait]
impl<R, P> ByzantineHandler for BeefyHost<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
{
	async fn check_for_byzantine_attack(
		&self,
		_counterparty: Arc<dyn IsmpProvider>,
		_challenge_event: StateMachineUpdated,
	) -> Result<(), anyhow::Error> {
		todo!()
	}
}
