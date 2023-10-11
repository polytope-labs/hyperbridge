use anyhow::anyhow;
use codec::Encode;
use consensus_client::types::ConsensusState;
use futures::stream;
use ismp::{
	consensus::{ConsensusStateId, StateMachineId},
	events::Event,
	messaging::Message,
	router::Get,
};
use std::{
	sync::{Arc, Mutex},
	time::Duration,
};
use tesseract_primitives::{
	BoxStream, ByzantineHandler, ChallengePeriodStarted, IsmpHost, IsmpProvider, Query,
	StateMachineUpdated,
};

#[derive(Clone)]
pub struct MockHost {
	pub consensus_state: Arc<Mutex<ConsensusState>>,
	pub latest_height: Arc<Mutex<u64>>,
}

impl MockHost {
	pub fn new(consensus_state: ConsensusState, latest_height: u64) -> Self {
		Self {
			consensus_state: Arc::new(Mutex::new(consensus_state)),
			latest_height: Arc::new(Mutex::new(latest_height)),
		}
	}
}

#[async_trait::async_trait]
impl ByzantineHandler for MockHost {
	async fn query_consensus_message(
		&self,
		_challenge_event: ChallengePeriodStarted,
	) -> Result<ismp::messaging::ConsensusMessage, anyhow::Error> {
		Err(anyhow!("No consensus messages"))
	}

	async fn check_for_byzantine_attack<T: IsmpHost>(
		&self,
		_counterparty: &T,
		_consensus_message: ismp::messaging::ConsensusMessage,
	) -> Result<(), anyhow::Error> {
		Err(anyhow!("No byzantine faults"))
	}
}

#[async_trait::async_trait]
impl IsmpHost for MockHost {
	async fn consensus_notification<I>(
		&self,
		_counterparty: I,
	) -> Result<BoxStream<ismp::messaging::ConsensusMessage>, anyhow::Error>
	where
		I: IsmpHost + IsmpProvider + Clone + 'static,
	{
		Ok(Box::pin(stream::pending()))
	}
}

#[async_trait::async_trait]
impl IsmpProvider for MockHost {
	async fn query_consensus_state(
		&self,
		_at: Option<u64>,
		_id: ConsensusStateId,
	) -> Result<Vec<u8>, anyhow::Error> {
		Ok(self.consensus_state.lock().unwrap().encode())
	}

	async fn query_latest_height(&self, _id: StateMachineId) -> Result<u32, anyhow::Error> {
		Ok(*self.latest_height.lock().unwrap() as u32)
	}

	async fn query_latest_messaging_height(
		&self,
		_id: StateMachineId,
	) -> Result<u64, anyhow::Error> {
		todo!()
	}

	async fn query_consensus_update_time(
		&self,
		_id: ConsensusStateId,
	) -> Result<Duration, anyhow::Error> {
		Ok(Duration::from_secs(0))
	}

	async fn query_challenge_period(
		&self,
		_id: ConsensusStateId,
	) -> Result<Duration, anyhow::Error> {
		Ok(Duration::from_secs(0))
	}

	async fn query_timestamp(&self) -> Result<Duration, anyhow::Error> {
		Ok(Duration::from_secs(0))
	}

	async fn query_requests_proof(
		&self,
		_at: u64,
		_keys: Vec<Query>,
	) -> Result<Vec<u8>, anyhow::Error> {
		todo!()
	}

	async fn query_responses_proof(
		&self,
		_at: u64,
		_keys: Vec<Query>,
	) -> Result<Vec<u8>, anyhow::Error> {
		todo!()
	}

	async fn query_state_proof(
		&self,
		_at: u64,
		_keys: Vec<Vec<u8>>,
	) -> Result<Vec<u8>, anyhow::Error> {
		todo!()
	}

	async fn query_ismp_events(
		&self,
		_event: StateMachineUpdated,
	) -> Result<Vec<Event>, anyhow::Error> {
		todo!()
	}

	async fn query_pending_get_requests(&self, _height: u64) -> Result<Vec<Get>, anyhow::Error> {
		todo!()
	}

	fn name(&self) -> String {
		"Mock".to_string()
	}

	fn state_machine_id(&self) -> StateMachineId {
		todo!()
	}

	fn block_max_gas(&self) -> u64 {
		todo!()
	}

	async fn estimate_gas(&self, _msg: Vec<Message>) -> Result<u64, anyhow::Error> {
		todo!()
	}

	async fn state_machine_update_notification(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> BoxStream<StateMachineUpdated> {
		todo!()
	}

	async fn submit(&self, _messages: Vec<Message>) -> Result<(), anyhow::Error> {
		todo!()
	}
}
