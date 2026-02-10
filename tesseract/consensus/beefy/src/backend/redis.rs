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

//! Redis-based implementation of the ProofBackend trait

use super::{ConsensusProof, ProofBackend, QueueMessage, StreamMessage};
use anyhow::Context;
use codec::{Decode, Encode};
use futures::Stream;
use ismp::host::StateMachine;
use redis::AsyncCommands;
use redis_async::client::{ConnectionBuilder, PubsubConnection};
use rsmq_async::{RedisBytes, Rsmq, RsmqConnection, RsmqError, RsmqMessage, RsmqOptions};
use serde::{Deserialize, Serialize};
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::sync::Mutex;

// Redis-specific conversions for ConsensusProof
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
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
	/// Enables publishing pubsub events for messages added to the queue
	pub realtime: bool,
	/// Queue name for mandatory consensus proofs
	pub mandatory_queue: String,
	/// Queue name for messages consensus proofs
	pub messages_queue: String,
}

impl From<RedisConfig> for redis::ConnectionAddr {
	fn from(value: RedisConfig) -> Self {
		redis::ConnectionAddr::Tcp(value.url.clone(), value.port)
	}
}

impl RedisConfig {
	pub fn mandatory_queue(&self, state_machine: &StateMachine) -> String {
		format!("{}-{}", self.mandatory_queue, state_machine.to_string())
	}

	pub fn messages_queue(&self, state_machine: &StateMachine) -> String {
		format!("{}-{}", self.messages_queue, state_machine.to_string())
	}
}

/// Redis-based implementation of the unified proof backend
pub struct RedisProofBackend {
	rsmq: Arc<Mutex<Rsmq>>,
	config: RedisConfig,
	connection: Arc<Mutex<redis::aio::ConnectionManager>>,
	pubsub: Arc<Mutex<PubsubConnection>>,
}

impl RedisProofBackend {
	pub async fn new(config: RedisConfig) -> Result<Self, anyhow::Error> {
		let rsmq = Arc::new(Mutex::new(create_rsmq_client(&config).await?));
		let connection = Arc::new(Mutex::new(
			redis::Client::open(redis::ConnectionInfo {
				addr: config.clone().into(),
				redis: redis::RedisConnectionInfo {
					db: config.db as i64,
					username: config.username.clone(),
					password: config.password.clone(),
				},
			})?
			.get_connection_manager()
			.await?,
		));
		let pubsub = Arc::new(Mutex::new(create_pubsub_client(&config).await?));

		Ok(Self { rsmq, config, connection, pubsub })
	}

	/// Get reference to the config
	pub fn config(&self) -> &RedisConfig {
		&self.config
	}

	/// Recreate the RSMQ client (useful for reconnection after connection drop)
	pub async fn recreate_rsmq(&self) -> Result<(), anyhow::Error> {
		*self.rsmq.lock().await = create_rsmq_client(&self.config).await?;
		Ok(())
	}

	/// Recreate the Redis connection manager (useful for reconnection after connection drop)
	pub async fn recreate_connection(&self) -> Result<(), anyhow::Error> {
		*self.connection.lock().await = redis::Client::open(redis::ConnectionInfo {
			addr: self.config.clone().into(),
			redis: redis::RedisConnectionInfo {
				db: self.config.db as i64,
				username: self.config.username.clone(),
				password: self.config.password.clone(),
			},
		})?
		.get_connection_manager()
		.await?;
		Ok(())
	}

	/// Recreate the pubsub client (useful for reconnection after connection drop)
	pub async fn recreate_pubsub(&self) -> Result<(), anyhow::Error> {
		*self.pubsub.lock().await = create_pubsub_client(&self.config).await?;
		Ok(())
	}

	/// Handle Redis errors and attempt reconnection if connection was dropped
	async fn handle_redis_error(&self, err: &anyhow::Error) {
		// Check if it's an RsmqError with connection drop
		if let Some(RsmqError::RedisError(redis_error)) = err.downcast_ref::<RsmqError>() {
			if redis_error.is_connection_dropped() {
				tracing::trace!("Redis connection dropped, recreating RSMQ client");
				if let Err(e) = self.recreate_rsmq().await {
					tracing::error!("Failed to recreate RSMQ client: {e:?}");
				} else {
					tracing::trace!("Successfully recreated RSMQ client");
				}
			} else {
				tracing::error!("Unhandled RSMQ error: {redis_error:?}");
			}
		}

		// Check if it's a direct RedisError with connection drop
		if let Some(redis_error) = err.downcast_ref::<redis::RedisError>() {
			if redis_error.is_connection_dropped() {
				tracing::trace!("Redis connection dropped, recreating connection manager");
				if let Err(e) = self.recreate_connection().await {
					tracing::error!("Failed to recreate connection manager: {e:?}");
				} else {
					tracing::trace!("Successfully recreated connection manager");
				}
			} else {
				tracing::error!("Unhandled Redis error: {redis_error:?}");
			}
		}
	}
}

#[async_trait::async_trait]
impl ProofBackend for RedisProofBackend {
	async fn init_queues(&self, state_machines: &[StateMachine]) -> Result<(), anyhow::Error> {
		for state_machine in state_machines {
			let mut rsmq = self.rsmq.lock().await;
			let result = rsmq
				.create_queue(
					self.config.mandatory_queue(state_machine).as_str(),
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
					self.config.messages_queue(state_machine).as_str(),
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

	async fn send_mandatory_proof(
		&self,
		state_machine: &StateMachine,
		proof: ConsensusProof,
	) -> Result<(), anyhow::Error> {
		let queue = self.config.mandatory_queue(state_machine);
		let result = self
			.rsmq
			.lock()
			.await
			.send_message(queue.as_str(), proof, Some(Duration::ZERO))
			.await;

		if let Err(err) = result {
			let anyhow_err = anyhow::Error::from(err);
			self.handle_redis_error(&anyhow_err).await;
			return Err(anyhow_err);
		}

		Ok(())
	}

	async fn send_messages_proof(
		&self,
		state_machine: &StateMachine,
		proof: ConsensusProof,
	) -> Result<(), anyhow::Error> {
		let queue = self.config.messages_queue(state_machine);
		let result = self
			.rsmq
			.lock()
			.await
			.send_message(queue.as_str(), proof, Some(Duration::ZERO))
			.await;

		if let Err(err) = result {
			let anyhow_err = anyhow::Error::from(err);
			self.handle_redis_error(&anyhow_err).await;
			return Err(anyhow_err);
		}

		Ok(())
	}

	async fn save_state(
		&self,
		state: &crate::prover::ProverConsensusState,
	) -> Result<(), anyhow::Error> {
		let result = self
			.connection
			.lock()
			.await
			.set::<_, _, ()>(crate::prover::REDIS_CONSENSUS_STATE_KEY, state.encode())
			.await;

		if let Err(err) = result {
			let anyhow_err = anyhow::Error::from(err);
			self.handle_redis_error(&anyhow_err).await;
			return Err(anyhow_err);
		}

		Ok(())
	}

	async fn load_state(&self) -> Result<crate::prover::ProverConsensusState, anyhow::Error> {
		use bytes::Buf;
		use codec::Decode;
		let result = self
			.connection
			.lock()
			.await
			.get::<_, bytes::Bytes>(crate::prover::REDIS_CONSENSUS_STATE_KEY)
			.await;

		let encoded = match result {
			Ok(encoded) => encoded,
			Err(err) => {
				let anyhow_err = anyhow::Error::from(err);
				self.handle_redis_error(&anyhow_err).await;
				return Err(anyhow_err);
			},
		};

		let state = crate::prover::ProverConsensusState::decode(&mut encoded.chunk())?;
		Ok(state)
	}

	async fn queue_notifications(
		&self,
		state_machine: StateMachine,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<StreamMessage, anyhow::Error>> + Send>>,
		anyhow::Error,
	> {
		use futures::{stream::TryStreamExt, StreamExt};

		let mandatory_queue = self.config.mandatory_queue(&state_machine);
		let messages_queue = self.config.messages_queue(&state_machine);
		let pubsub = self.pubsub.lock().await;

		let mandatory_stream = {
			pubsub
				.subscribe(&format!("{}:rt:{mandatory_queue}", self.config.ns))
				.await
				.context("Failed to subscribe to mandatory queue")?
				.map_ok(|_item| StreamMessage::EpochChanged)
				.map_err(|e| anyhow::anyhow!("Redis error: {:?}", e))
		};
		let messages_stream = {
			pubsub
				.subscribe(&format!("{}:rt:{messages_queue}", self.config.ns))
				.await
				.context("Failed to subscribe to messages queue")?
				.map_ok(|_item| StreamMessage::NewMessages)
				.map_err(|e| anyhow::anyhow!("Redis error: {:?}", e))
		};
		let combined = futures::stream::select(mandatory_stream, messages_stream);
		let yield_once = futures::stream::iter(vec![Ok(StreamMessage::EpochChanged)]);
		Ok(Box::pin(yield_once.chain(combined)))
	}

	async fn receive_mandatory_proof(
		&self,
		state_machine: &StateMachine,
	) -> Result<Option<QueueMessage>, anyhow::Error> {
		let queue = self.config.mandatory_queue(state_machine);
		let result = self.rsmq.lock().await.receive_message::<ConsensusProof>(&queue, None).await;

		let msg_result = match result {
			Ok(msg) => msg,
			Err(err) => {
				let anyhow_err = anyhow::Error::from(err);
				self.handle_redis_error(&anyhow_err).await;
				return Err(anyhow_err);
			},
		};

		Ok(msg_result.map(|msg: RsmqMessage<ConsensusProof>| QueueMessage {
			id: msg.id,
			proof: msg.message,
		}))
	}

	async fn receive_messages_proof(
		&self,
		state_machine: &StateMachine,
	) -> Result<Option<QueueMessage>, anyhow::Error> {
		let queue = self.config.messages_queue(state_machine);
		let result = self.rsmq.lock().await.receive_message::<ConsensusProof>(&queue, None).await;

		let msg_result = match result {
			Ok(msg) => msg,
			Err(err) => {
				let anyhow_err = anyhow::Error::from(err);
				self.handle_redis_error(&anyhow_err).await;
				return Err(anyhow_err);
			},
		};

		Ok(msg_result.map(|msg: RsmqMessage<ConsensusProof>| QueueMessage {
			id: msg.id,
			proof: msg.message,
		}))
	}

	async fn delete_message(
		&self,
		state_machine: &StateMachine,
		message_id: &str,
		message_type: StreamMessage,
	) -> Result<(), anyhow::Error> {
		let queue = match message_type {
			StreamMessage::EpochChanged => self.config.mandatory_queue(state_machine),
			StreamMessage::NewMessages => self.config.messages_queue(state_machine),
		};

		let result = self.rsmq.lock().await.delete_message(&queue, message_id).await;

		if let Err(err) = result {
			let anyhow_err = anyhow::Error::from(err);
			self.handle_redis_error(&anyhow_err).await;
			return Err(anyhow_err);
		}

		Ok(())
	}

	async fn reconnect_notifier(&self) -> Result<(), anyhow::Error> {
		self.recreate_pubsub().await
	}
}

/// Constructs an [`Rsmq`] client given a [`RedisConfig`]
async fn create_rsmq_client(config: &RedisConfig) -> Result<Rsmq, anyhow::Error> {
	let options = RsmqOptions {
		host: config.url.clone(),
		port: config.port,
		username: config.username.clone(),
		password: config.password.clone(),
		db: config.db,
		ns: config.ns.clone(),
		realtime: config.realtime,
	};
	let rsmq = Rsmq::new(options).await?;

	Ok(rsmq)
}

/// Builds a [`PubsubConnection`] to redis for queue notifications
async fn create_pubsub_client(config: &RedisConfig) -> Result<PubsubConnection, anyhow::Error> {
	let mut builder = ConnectionBuilder::new(&config.url, config.port)?;
	if let Some(ref username) = config.username {
		builder.username(username.as_str());
	}
	if let Some(ref password) = config.password {
		builder.password(password.as_str());
	}
	let pubsub = builder.pubsub_connect().await?;

	Ok(pubsub)
}
