use crate::{
	BoxStream, ByzantineHandler, EstimateGasReturnParams, HyperbridgeClaim, IsmpHost, IsmpProvider,
	Query, Signature, StateMachineUpdated, StateProofQueryType, TxResult, WithdrawFundsResult,
};
use anyhow::{anyhow, Error};
use ismp::{
	consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
	events::{Event, StateCommitmentVetoed},
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use pallet_ismp_host_executive::HostParam;
use pallet_ismp_relayer::withdrawal::{Key, WithdrawalProof};
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
impl<T: Codec + Send + Sync> HyperbridgeClaim for MockHost<T> {
	async fn accumulate_fees(&self, _proof: WithdrawalProof) -> anyhow::Result<()> {
		Err(anyhow!("Unimplemented"))
	}

	async fn withdraw_funds(
		&self,
		_client: Arc<dyn IsmpProvider>,
		_chain: StateMachine,
	) -> anyhow::Result<WithdrawFundsResult> {
		Err(anyhow!("Unimplemented"))
	}

	async fn check_claimed(&self, _key: Key) -> anyhow::Result<bool> {
		Ok(false)
	}
}

#[async_trait::async_trait]
impl<C: Codec + Send + Sync> ByzantineHandler for MockHost<C> {
	async fn check_for_byzantine_attack(
		&self,
		_coprocessor: StateMachine,
		_counterparty: Arc<dyn IsmpProvider>,
		_challenge_event: StateMachineUpdated,
	) -> Result<(), Error> {
		Err(anyhow!("No byzantine faults"))
	}

	async fn state_machine_updates(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<Vec<StateMachineUpdated>>, anyhow::Error> {
		Err(anyhow!("No byzantine faults"))
	}
}

#[async_trait::async_trait]
impl<C: Codec + Send + Sync> IsmpHost for MockHost<C> {
	async fn start_consensus(&self, _counterparty: Arc<dyn IsmpProvider>) -> Result<(), Error> {
		Ok(())
	}

	async fn query_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
		todo!()
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
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

	async fn query_finalized_height(&self) -> Result<u64, Error> {
		Ok(*self.latest_height.lock().unwrap())
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

	async fn query_challenge_period(&self, _id: StateMachineId) -> Result<Duration, Error> {
		Ok(Duration::from_secs(0))
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		Ok(Duration::from_secs(0))
	}

	async fn query_requests_proof(
		&self,
		_at: u64,
		_keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		Ok(Default::default())
	}

	async fn query_responses_proof(
		&self,
		_at: u64,
		_keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		Ok(Default::default())
	}

	async fn query_state_proof(
		&self,
		_at: u64,
		_keys: StateProofQueryType,
	) -> Result<Vec<u8>, Error> {
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
		Ok(U256::from(1))
	}

	async fn state_machine_update_notification(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, Error> {
		todo!()
	}

	async fn state_commitment_vetoed_notification(
		&self,
		_from: u64,
		_height: StateMachineHeight,
	) -> BoxStream<StateCommitmentVetoed> {
		todo!()
	}

	async fn submit(
		&self,
		_messages: Vec<Message>,
		_coprocessor: StateMachine,
	) -> Result<TxResult, Error> {
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

	async fn set_latest_finalized_height(
		&mut self,
		_counterparty: Arc<dyn IsmpProvider>,
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
		Ok(U256::from(1))
	}

	async fn veto_state_commitment(&self, _height: StateMachineHeight) -> Result<(), Error> {
		todo!()
	}

	async fn query_host_params(
		&self,
		_state_machine: StateMachine,
	) -> Result<HostParam<u128>, anyhow::Error> {
		todo!()
	}

	async fn query_request_receipt(&self, _hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		todo!()
	}

	async fn query_response_receipt(&self, _hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		todo!()
	}

	async fn fee_token_decimals(&self) -> Result<u8, anyhow::Error> {
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
