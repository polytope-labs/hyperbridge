use crate::{
	BoxStream, ByzantineHandler, Cost, EstimateGasReturnParams, HandleGetResponse,
	HyperbridgeClaim, IsmpHost, IsmpProvider, Query, Signature, StateMachineUpdated,
	StateProofQueryType, TxResult, WithdrawFundsResult,
};
use anyhow::{anyhow, Error};
use ismp::{
	consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
	events::{Event, StateCommitmentVetoed},
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use pallet_ismp_host_executive::HostParam;
use pallet_ismp_relayer::withdrawal::WithdrawalProof;
use pallet_state_coprocessor::impls::GetRequestsWithProof;
use parity_scale_codec::Codec;
use primitive_types::{H160, H256, U256};
use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
	time::Duration,
};

pub struct MockHost<C> {
	pub consensus_state: Arc<Mutex<C>>,
	pub latest_height: Arc<Mutex<u64>>,
	pub state_machine: StateMachine,
	/// Records batches passed to [`IsmpProvider::submit`]. Callers can clone and
	/// inspect to assert on what the code under test sent.
	pub submitted: Arc<Mutex<Vec<Vec<Message>>>>,
	/// When false, [`IsmpProvider::query_requests_proof`] returns an error.
	pub request_proof_ok: Arc<Mutex<bool>>,
	/// When false, [`IsmpProvider::query_state_proof`] returns an error.
	pub state_proof_ok: Arc<Mutex<bool>>,
	/// The proof bytes returned by [`IsmpProvider::query_state_proof`].
	pub state_proof: Arc<Mutex<Vec<u8>>>,
	/// Records the `(height, keys)` passed to [`IsmpProvider::query_state_proof`].
	pub state_proof_queries: Arc<Mutex<Vec<(u64, StateProofQueryType)>>>,
	/// Records messages passed to [`HandleGetResponse::submit_get_response`].
	pub get_responses: Arc<Mutex<Vec<GetRequestsWithProof>>>,
	/// When false, [`HandleGetResponse::dry_run_submission`] returns an error.
	pub dry_run_ok: Arc<Mutex<bool>>,
	/// The address returned by [`IsmpProvider::address`].
	pub address: Arc<Mutex<Vec<u8>>>,
	/// The name returned by [`IsmpProvider::name`].
	pub name: Arc<Mutex<String>>,
	/// The consensus state id embedded in [`IsmpProvider::state_machine_id`].
	pub consensus_state_id: Arc<Mutex<ConsensusStateId>>,
	/// Commitments this host has a delivery receipt for, keyed to the relayer
	/// that delivered them.
	pub request_receipts: Arc<Mutex<BTreeMap<H256, Vec<u8>>>>,
	/// Response receipts keyed by the GET request commitment.
	pub response_receipts: Arc<Mutex<BTreeMap<H256, Vec<u8>>>>,
	/// The timestamp returned by [`IsmpProvider::query_timestamp`].
	pub timestamp: Arc<Mutex<Duration>>,
	/// When false, [`IsmpProvider::query_timestamp`] returns an error.
	pub timestamp_ok: Arc<Mutex<bool>>,
	/// The events returned by [`IsmpProvider::query_ismp_events`].
	pub events: Arc<Mutex<Vec<Event>>>,
	/// When false, [`IsmpProvider::query_ismp_events`] returns an error.
	pub query_events_ok: Arc<Mutex<bool>>,
}

impl<C> MockHost<C> {
	pub fn new(consensus_state: C, latest_height: u64, state_machine: StateMachine) -> Self {
		Self {
			consensus_state: Arc::new(Mutex::new(consensus_state)),
			latest_height: Arc::new(Mutex::new(latest_height)),
			state_machine,
			submitted: Arc::new(Mutex::new(Vec::new())),
			request_proof_ok: Arc::new(Mutex::new(true)),
			state_proof_ok: Arc::new(Mutex::new(true)),
			state_proof: Arc::new(Mutex::new(vec![1])),
			state_proof_queries: Arc::new(Mutex::new(Vec::new())),
			get_responses: Arc::new(Mutex::new(Vec::new())),
			dry_run_ok: Arc::new(Mutex::new(true)),
			address: Arc::new(Mutex::new(Vec::new())),
			name: Arc::new(Mutex::new("Mock".to_string())),
			consensus_state_id: Arc::new(Mutex::new(*b"Mock")),
			request_receipts: Arc::new(Mutex::new(BTreeMap::new())),
			response_receipts: Arc::new(Mutex::new(BTreeMap::new())),
			timestamp: Arc::new(Mutex::new(Duration::from_secs(0))),
			timestamp_ok: Arc::new(Mutex::new(true)),
			events: Arc::new(Mutex::new(Vec::new())),
			query_events_ok: Arc::new(Mutex::new(true)),
		}
	}

	/// Clone of the submission log since the mock was created.
	pub fn submissions(&self) -> Vec<Vec<Message>> {
		self.submitted.lock().unwrap().clone()
	}

	pub fn with_request_proof_fail(self) -> Self {
		*self.request_proof_ok.lock().unwrap() = false;
		self
	}

	pub fn with_state_proof_fail(self) -> Self {
		*self.state_proof_ok.lock().unwrap() = false;
		self
	}

	pub fn with_state_proof(self, proof: Vec<u8>) -> Self {
		*self.state_proof.lock().unwrap() = proof;
		self
	}

	pub fn with_timestamp_fail(self) -> Self {
		*self.timestamp_ok.lock().unwrap() = false;
		self
	}

	pub fn with_query_events_fail(self) -> Self {
		*self.query_events_ok.lock().unwrap() = false;
		self
	}

	pub fn with_address(self, address: Vec<u8>) -> Self {
		*self.address.lock().unwrap() = address;
		self
	}

	pub fn with_name(self, name: impl Into<String>) -> Self {
		*self.name.lock().unwrap() = name.into();
		self
	}

	pub fn with_consensus_state_id(self, id: ConsensusStateId) -> Self {
		*self.consensus_state_id.lock().unwrap() = id;
		self
	}

	/// Record `commitment` as already delivered by `relayer`.
	pub fn with_request_receipt(self, commitment: H256, relayer: Vec<u8>) -> Self {
		self.request_receipts.lock().unwrap().insert(commitment, relayer);
		self
	}

	/// Record a response receipt for a GET request commitment.
	pub fn with_response_receipt(self, commitment: H256, relayer: Vec<u8>) -> Self {
		self.response_receipts.lock().unwrap().insert(commitment, relayer);
		self
	}

	pub fn with_dry_run_fail(self) -> Self {
		*self.dry_run_ok.lock().unwrap() = false;
		self
	}

	pub fn with_timestamp(self, timestamp: Duration) -> Self {
		*self.timestamp.lock().unwrap() = timestamp;
		self
	}

	pub fn with_events(self, events: Vec<Event>) -> Self {
		*self.events.lock().unwrap() = events;
		self
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
	) -> anyhow::Result<Vec<WithdrawFundsResult>> {
		Err(anyhow!("Unimplemented"))
	}

	async fn check_claimed(&self, _commitment: H256) -> anyhow::Result<bool> {
		Ok(false)
	}

	async fn relayer_nonce(&self, _address: Vec<u8>, _chain: StateMachine) -> anyhow::Result<u64> {
		Ok(0)
	}
}

#[async_trait::async_trait]
impl<C: Codec + Send + Sync> HandleGetResponse for MockHost<C> {
	async fn submit_get_response(&self, msg: GetRequestsWithProof) -> anyhow::Result<()> {
		self.get_responses.lock().unwrap().push(msg);
		Ok(())
	}

	async fn dry_run_submission(&self, _msg: GetRequestsWithProof) -> anyhow::Result<()> {
		if *self.dry_run_ok.lock().unwrap() {
			Ok(())
		} else {
			Err(anyhow!("mock: dry run failure"))
		}
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
		if *self.timestamp_ok.lock().unwrap() {
			Ok(*self.timestamp.lock().unwrap())
		} else {
			Err(anyhow!("mock: timestamp query failure"))
		}
	}

	async fn query_requests_proof(
		&self,
		_at: u64,
		_keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		if *self.request_proof_ok.lock().unwrap() {
			Ok(Default::default())
		} else {
			Err(anyhow!("mock: request proof failure"))
		}
	}

	async fn query_state_proof(
		&self,
		at: u64,
		keys: StateProofQueryType,
	) -> Result<Vec<u8>, Error> {
		self.state_proof_queries.lock().unwrap().push((at, keys));
		if *self.state_proof_ok.lock().unwrap() {
			Ok(self.state_proof.lock().unwrap().clone())
		} else {
			Err(anyhow!("mock: state proof failure"))
		}
	}

	async fn query_ismp_events(
		&self,
		_previous_height: u64,
		_event: StateMachineUpdated,
	) -> Result<Vec<Event>, Error> {
		if *self.query_events_ok.lock().unwrap() {
			Ok(self.events.lock().unwrap().clone())
		} else {
			Err(anyhow!("mock: query events failure"))
		}
	}

	fn name(&self) -> String {
		self.name.lock().unwrap().clone()
	}

	fn state_machine_id(&self) -> StateMachineId {
		StateMachineId {
			state_id: self.state_machine,
			consensus_state_id: *self.consensus_state_id.lock().unwrap(),
		}
	}

	fn ismp_host_contract(&self) -> Option<sp_core::H160> {
		None
	}

	fn block_max_gas(&self) -> u64 {
		todo!()
	}

	fn initial_height(&self) -> u64 {
		0
	}

	async fn estimate_gas(
		&self,
		msg: Vec<Message>,
	) -> Result<Vec<EstimateGasReturnParams>, anyhow::Error> {
		// Default: every message executes successfully at zero cost. Tests that
		// need to exercise the unprofitable path should override behaviour via
		// a dedicated flag rather than relying on this stub.
		Ok(msg
			.into_iter()
			.map(|_| EstimateGasReturnParams {
				execution_cost: Cost::default(),
				successful_execution: true,
				failure: None,
			})
			.collect())
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
		messages: Vec<Message>,
		_coprocessor: StateMachine,
	) -> Result<TxResult, Error> {
		self.submitted.lock().unwrap().push(messages);
		Ok(TxResult::default())
	}

	fn request_commitment_full_key(&self, _commitment: H256) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn request_receipt_full_key(&self, _commitment: H256) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn address(&self) -> Vec<u8> {
		self.address.lock().unwrap().clone()
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

	async fn veto_state_commitment(&self, _height: StateMachineHeight) -> Result<(), Error> {
		todo!()
	}

	async fn query_host_params(
		&self,
		_state_machine: StateMachine,
	) -> Result<HostParam, anyhow::Error> {
		todo!()
	}

	async fn query_request_receipt(&self, hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		let receipts = self.request_receipts.lock().unwrap();
		Ok(receipts.get(&hash).cloned().unwrap_or_else(|| H160::zero().0.to_vec()))
	}

	async fn query_response_receipt(&self, hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		let receipts = self.response_receipts.lock().unwrap();
		Ok(receipts.get(&hash).cloned().unwrap_or_else(|| H160::zero().0.to_vec()))
	}

	async fn fee_token_decimals(&self) -> Result<u8, anyhow::Error> {
		Ok(18)
	}
}

impl<C: Send + Sync> Clone for MockHost<C> {
	fn clone(&self) -> Self {
		Self {
			consensus_state: self.consensus_state.clone(),
			latest_height: self.latest_height.clone(),
			state_machine: self.state_machine.clone(),
			submitted: self.submitted.clone(),
			request_proof_ok: self.request_proof_ok.clone(),
			state_proof_ok: self.state_proof_ok.clone(),
			state_proof: self.state_proof.clone(),
			state_proof_queries: self.state_proof_queries.clone(),
			get_responses: self.get_responses.clone(),
			dry_run_ok: self.dry_run_ok.clone(),
			address: self.address.clone(),
			name: self.name.clone(),
			consensus_state_id: self.consensus_state_id.clone(),
			request_receipts: self.request_receipts.clone(),
			response_receipts: self.response_receipts.clone(),
			timestamp: self.timestamp.clone(),
			timestamp_ok: self.timestamp_ok.clone(),
			events: self.events.clone(),
			query_events_ok: self.query_events_ok.clone(),
		}
	}
}
