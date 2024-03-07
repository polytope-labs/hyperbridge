use crate::{
	BoxStream, ByzantineHandler, EstimateGasReturnParams, IsmpHost, IsmpProvider, Query, Signature,
	StateMachineUpdated, TxReceipt,
};
use anyhow::{anyhow, Error};
use futures::stream;
use ismp::{
	consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
	events::Event,
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use parity_scale_codec::Codec;
use primitive_types::{H256, U256};
use std::{
	sync::{Arc, Mutex},
	time::Duration,
};

pub struct MockHost<C> {
	pub consensus_state: Arc<Mutex<C>>,
	pub latest_height: Arc<Mutex<u64>>,
	pub state_machine: StateMachine,
}

impl<C> MockHost<C> {
	pub fn new(consensus_state: C, latest_height: u64, state_machine: StateMachine) -> Self {
		Self {
			consensus_state: Arc::new(Mutex::new(consensus_state)),
			latest_height: Arc::new(Mutex::new(latest_height)),
			state_machine,
		}
	}
}

#[async_trait::async_trait]
impl<C: Codec + Send + Sync> ByzantineHandler for MockHost<C> {
	async fn query_consensus_message(
		&self,
		_challenge_event: StateMachineUpdated,
	) -> Result<ismp::messaging::ConsensusMessage, Error> {
		Err(anyhow!("No consensus messages"))
	}

	async fn check_for_byzantine_attack<T: IsmpHost + IsmpProvider>(
		&self,
		_counterparty: &T,
		_consensus_message: ismp::messaging::ConsensusMessage,
	) -> Result<(), Error> {
		Err(anyhow!("No byzantine faults"))
	}
}

#[async_trait::async_trait]
impl<C: Codec + Send + Sync> IsmpHost for MockHost<C> {
	async fn consensus_notification<I>(
		&self,
		_counterparty: I,
	) -> Result<BoxStream<ismp::messaging::ConsensusMessage>, Error>
	where
		I: IsmpHost + IsmpProvider + Clone + 'static,
	{
		Ok(Box::pin(stream::pending()))
	}

	async fn query_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
		todo!()
	}
}

#[async_trait::async_trait]
impl<C: Codec + Send + Sync> IsmpProvider for MockHost<C> {
	async fn query_consensus_state(
		&self,
		_at: Option<u64>,
		_id: ConsensusStateId,
	) -> Result<Vec<u8>, Error> {
		Ok(self.consensus_state.lock().unwrap().encode())
	}

	async fn query_latest_height(&self, _id: StateMachineId) -> Result<u32, Error> {
		Ok(*self.latest_height.lock().unwrap() as u32)
	}

	async fn query_state_machine_commitment(
		&self,
		_height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		todo!()
	}

	async fn query_state_machine_update_time(
		&self,
		_height: StateMachineHeight,
	) -> Result<Duration, Error> {
		Ok(Duration::from_secs(0))
	}

	async fn query_challenge_period(&self, _id: ConsensusStateId) -> Result<Duration, Error> {
		Ok(Duration::from_secs(0))
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		Ok(Duration::from_secs(0))
	}

	async fn query_requests_proof(&self, _at: u64, _keys: Vec<Query>) -> Result<Vec<u8>, Error> {
		Ok(Default::default())
	}

	async fn query_responses_proof(&self, _at: u64, _keys: Vec<Query>) -> Result<Vec<u8>, Error> {
		Ok(Default::default())
	}

	async fn query_state_proof(&self, _at: u64, _keys: Vec<Vec<u8>>) -> Result<Vec<u8>, Error> {
		Ok(Default::default())
	}

	async fn query_ismp_events(
		&self,
		_previous_height: u64,
		_event: StateMachineUpdated,
	) -> Result<Vec<Event>, Error> {
		todo!()
	}

	fn name(&self) -> String {
		"Mock".to_string()
	}

	fn state_machine_id(&self) -> StateMachineId {
		StateMachineId { state_id: self.state_machine, consensus_state_id: *b"Mock" }
	}

	fn block_max_gas(&self) -> u64 {
		todo!()
	}

	fn initial_height(&self) -> u64 {
		0
	}

	async fn estimate_gas(
		&self,
		_msg: Vec<Message>,
	) -> Result<Vec<EstimateGasReturnParams>, anyhow::Error> {
		todo!()
	}

	async fn query_request_fee_metadata(&self, _hash: H256) -> Result<U256, anyhow::Error> {
		todo!()
	}

	async fn state_machine_update_notification(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, Error> {
		todo!()
	}

	async fn submit(&self, _messages: Vec<Message>) -> Result<Vec<TxReceipt>, Error> {
		todo!()
	}

	fn request_commitment_full_key(&self, _commitment: H256) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn request_receipt_full_key(&self, _commitment: H256) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn response_commitment_full_key(&self, _commitment: H256) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn response_receipt_full_key(&self, _commitment: H256) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn address(&self) -> Vec<u8> {
		Default::default()
	}

	fn sign(&self, _msg: &[u8]) -> Signature {
		todo!()
	}

	async fn set_latest_finalized_height<P: IsmpProvider + 'static>(
		&mut self,
		_counterparty: &P,
	) -> Result<(), anyhow::Error> {
		todo!()
	}

	async fn set_initial_consensus_state(
		&self,
		_message: CreateConsensusState,
	) -> Result<(), Error> {
		todo!()
	}

	async fn query_response_fee_metadata(&self, _hash: H256) -> Result<U256, Error> {
		todo!()
	}

	async fn freeze_state_machine(&self, _id: StateMachineId) -> Result<(), Error> {
		todo!()
	}

	async fn query_host_manager_address(&self) -> Result<Vec<u8>, anyhow::Error> {
		todo!()
	}
}

impl<C: Send + Sync> Clone for MockHost<C> {
	fn clone(&self) -> Self {
		Self {
			consensus_state: self.consensus_state.clone(),
			latest_height: self.latest_height.clone(),
			state_machine: self.state_machine.clone(),
		}
	}
}
