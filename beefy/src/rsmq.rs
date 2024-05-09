use ismp::host::StateMachine;
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

impl RedisConfig {
	pub fn mandatory_queue(&self, state_machine: &StateMachine) -> String {
		format!("{}-{}", self.mandatory_queue, state_machine.to_string())
	}

	pub fn messages_queue(&self, state_machine: &StateMachine) -> String {
		format!("{}-{}", self.messages_queue, state_machine.to_string())
	}
}

/// Constructs an [`Rsmq`] client given a [`RedisConfig`]
pub async fn client(config: &RedisConfig) -> Result<Rsmq, anyhow::Error> {
	let options = RsmqOptions {
		host: config.url.clone(),
		port: config.port.clone(),
		username: config.username.clone(),
		password: config.password.clone(),
		db: config.db.clone(),
		ns: config.ns.clone(),
		// we will not be publishing messages here
		realtime: config.realtime,
	};
	let rsmq = Rsmq::new(options).await?;

	Ok(rsmq)
}
