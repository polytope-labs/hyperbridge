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

use ismp::host::StateMachine;
use redis_async::client::{ConnectionBuilder, PubsubConnection};
use rsmq_async::{Rsmq, RsmqOptions};
use serde::{Deserialize, Serialize};

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

/// Constructs an [`Rsmq`] client given a [`RedisConfig`]
pub async fn rsmq_client(config: &RedisConfig) -> Result<Rsmq, anyhow::Error> {
	let options = RsmqOptions {
		host: config.url.clone(),
		port: config.port.clone(),
		username: config.username.clone(),
		password: config.password.clone(),
		db: config.db.clone(),
		ns: config.ns.clone(),
		realtime: config.realtime,
	};
	let rsmq = Rsmq::new(options).await?;

	Ok(rsmq)
}

/// Builds a [`PubSubConnection`] to redis for queue notifications
pub async fn pubsub_client(config: &RedisConfig) -> Result<PubsubConnection, anyhow::Error> {
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
