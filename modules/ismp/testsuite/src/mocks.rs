use codec::Encode;
use polkadot_sdk::{sp_runtime::Weight, *};
use primitive_types::H256;
use std::{
	cell::RefCell,
	collections::{BTreeMap, BTreeSet, HashMap},
	rc::Rc,
	sync::Arc,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineHeight, StateMachineId, VerifiedCommitments,
	},
	dispatcher::{DispatchRequest, FeeMetadata, IsmpDispatcher},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::{hash_post_response, hash_request, hash_response, Keccak256, Proof},
	module::IsmpModule,
	router::{
		GetRequest, IsmpRouter, PostRequest, PostResponse, Request, RequestResponse, Response,
		Timeout,
	},
};

#[derive(Default)]
pub struct MockClient;
#[derive(Default)]
pub struct MockProxyClient;

pub const MOCK_CONSENSUS_CLIENT_ID: [u8; 4] = [1u8; 4];
pub const MOCK_PROXY_CONSENSUS_CLIENT_ID: [u8; 4] = [2u8; 4];

#[derive(codec::Encode, codec::Decode)]
pub struct MockConsensusState {
	frozen_height: Option<u64>,
}

impl ConsensusClient for MockClient {
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		_consensus_state_id: ConsensusStateId,
		_trusted_consensus_state: Vec<u8>,
		_proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		Ok(Default::default())
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		_trusted_consensus_state: Vec<u8>,
		_proof_1: Vec<u8>,
		_proof_2: Vec<u8>,
	) -> Result<(), Error> {
		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		MOCK_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, _id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		match _id {
			StateMachine::Evm(11155111) => Ok(Box::new(MockStateMachineClient)),
			_ => Err(Error::Custom("Invalid state machine".to_string())),
		}
	}
}

impl ConsensusClient for MockProxyClient {
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		_consensus_state_id: ConsensusStateId,
		_trusted_consensus_state: Vec<u8>,
		_proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		Ok(Default::default())
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		_trusted_consensus_state: Vec<u8>,
		_proof_1: Vec<u8>,
		_proof_2: Vec<u8>,
	) -> Result<(), Error> {
		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		MOCK_PROXY_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, _id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		match _id {
			StateMachine::Kusama(2000) => Ok(Box::new(MockStateMachineClient)),
			_ => Err(Error::Custom("Invalid state machine".to_string())),
		}
	}
}

pub struct MockStateMachineClient;

impl StateMachineClient for MockStateMachineClient {
	fn verify_membership(
		&self,
		_host: &dyn IsmpHost,
		_item: RequestResponse,
		_root: StateCommitment,
		_proof: &Proof,
	) -> Result<(), Error> {
		Ok(())
	}

	fn receipts_state_trie_key(&self, _request: RequestResponse) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		_keys: Vec<Vec<u8>>,
		_root: StateCommitment,
		_proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		Ok(Default::default())
	}
}

#[derive(Default, Clone, Debug)]
pub struct Host {
	requests: Rc<RefCell<BTreeSet<H256>>>,
	receipts: Rc<RefCell<HashMap<H256, ()>>>,
	responses: Rc<RefCell<BTreeSet<H256>>>,
	consensus_clients: Rc<RefCell<HashMap<ConsensusStateId, ConsensusClientId>>>,
	consensus_states: Rc<RefCell<HashMap<ConsensusStateId, Vec<u8>>>>,
	state_commitments: Rc<RefCell<HashMap<StateMachineHeight, StateCommitment>>>,
	consensus_update_time: Rc<RefCell<HashMap<ConsensusStateId, Duration>>>,
	frozen_consensus_clients: Rc<RefCell<HashMap<ConsensusStateId, bool>>>,
	latest_state_height: Rc<RefCell<HashMap<StateMachineId, u64>>>,
	previous_state_height: Rc<RefCell<HashMap<StateMachineId, u64>>>,
	nonce: Rc<RefCell<u64>>,
	pub proxy: Option<StateMachine>,
}

impl IsmpHost for Host {
	fn host_state_machine(&self) -> StateMachine {
		StateMachine::Polkadot(1000)
	}

	fn latest_commitment_height(&self, id: StateMachineId) -> Result<u64, Error> {
		self.latest_state_height
			.borrow()
			.get(&id)
			.copied()
			.ok_or_else(|| Error::Custom("latest height not found".into()))
	}

	fn state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		self.state_commitments
			.borrow()
			.get(&height)
			.cloned()
			.ok_or_else(|| Error::StateCommitmentNotFound { height })
	}

	fn consensus_update_time(&self, id: ConsensusStateId) -> Result<Duration, Error> {
		self.consensus_update_time
			.borrow()
			.get(&id)
			.copied()
			.ok_or_else(|| Error::Custom("Consensus update time not found".into()))
	}

	fn state_machine_update_time(
		&self,
		state_machine_height: StateMachineHeight,
	) -> Result<Duration, Error> {
		self.consensus_update_time
			.borrow()
			.get(&state_machine_height.id.consensus_state_id)
			.copied()
			.ok_or_else(|| Error::Custom("Consensus update time not found".into()))
	}

	fn consensus_client_id(
		&self,
		consensus_state_id: ConsensusStateId,
	) -> Option<ConsensusClientId> {
		self.consensus_clients.borrow().get(&consensus_state_id).copied()
	}

	fn consensus_state(&self, id: ConsensusStateId) -> Result<Vec<u8>, Error> {
		self.consensus_states
			.borrow()
			.get(&id)
			.cloned()
			.ok_or_else(|| Error::Custom("consensus state not found".into()))
	}

	fn timestamp(&self) -> Duration {
		SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
	}

	fn is_consensus_client_frozen(&self, _client: ConsensusStateId) -> Result<(), Error> {
		let binding = self.frozen_consensus_clients.borrow();
		let val = binding.get(&_client).unwrap_or(&false);
		if *val {
			Err(Error::FrozenConsensusClient { consensus_state_id: _client })?;
		}

		Ok(())
	}

	fn request_commitment(&self, hash: H256) -> Result<(), Error> {
		self.requests
			.borrow()
			.contains(&hash)
			.then_some(())
			.ok_or_else(|| Error::Custom("Request commitment not found".into()))
	}

	fn response_commitment(&self, hash: H256) -> Result<(), Error> {
		self.responses
			.borrow()
			.contains(&hash)
			.then_some(())
			.ok_or_else(|| Error::Custom("Request commitment not found".into()))
	}

	fn next_nonce(&self) -> u64 {
		let nonce = *self.nonce.borrow();
		*self.nonce.borrow_mut() = nonce + 1;
		nonce
	}

	fn request_receipt(&self, req: &Request) -> Option<()> {
		let hash = hash_request::<Self>(req);
		self.receipts.borrow().get(&hash).map(|_| ())
	}

	fn response_receipt(&self, res: &Response) -> Option<()> {
		let hash = hash_request::<Self>(&res.request());
		self.receipts.borrow().get(&hash).map(|_| ())
	}

	fn store_consensus_state_id(
		&self,
		consensus_state_id: ConsensusStateId,
		client_id: ConsensusClientId,
	) -> Result<(), Error> {
		self.consensus_clients.borrow_mut().insert(consensus_state_id, client_id);
		Ok(())
	}

	fn store_consensus_state(&self, id: ConsensusStateId, state: Vec<u8>) -> Result<(), Error> {
		self.consensus_states.borrow_mut().insert(id, state);
		Ok(())
	}

	fn store_unbonding_period(
		&self,
		_consensus_state_id: ConsensusStateId,
		_period: u64,
	) -> Result<(), Error> {
		Ok(())
	}

	fn store_consensus_update_time(
		&self,
		id: ConsensusStateId,
		timestamp: Duration,
	) -> Result<(), Error> {
		self.consensus_update_time.borrow_mut().insert(id, timestamp);
		Ok(())
	}

	fn store_state_machine_update_time(
		&self,
		_state_machine_height: StateMachineHeight,
		_timestamp: Duration,
	) -> Result<(), Error> {
		Ok(())
	}

	fn store_state_machine_commitment(
		&self,
		height: StateMachineHeight,
		state: StateCommitment,
	) -> Result<(), Error> {
		self.state_commitments.borrow_mut().insert(height, state);
		Ok(())
	}

	fn delete_state_commitment(&self, height: StateMachineHeight) -> Result<(), Error> {
		self.state_commitments.borrow_mut().remove(&height);
		Ok(())
	}

	fn freeze_consensus_client(&self, _client: ConsensusStateId) -> Result<(), Error> {
		self.frozen_consensus_clients.borrow_mut().insert(_client, true);
		Ok(())
	}

	fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error> {
		let previous_height = self.latest_state_height.borrow().get(&height.id).copied();

		self.previous_state_height
			.borrow_mut()
			.insert(height.id, previous_height.unwrap_or_default());

		self.latest_state_height.borrow_mut().insert(height.id, height.height);
		Ok(())
	}

	fn delete_request_commitment(&self, req: &Request) -> Result<Vec<u8>, Error> {
		let hash = hash_request::<Self>(req);
		let val = self.requests.borrow_mut().remove(&hash);
		Ok(val.encode())
	}

	fn delete_response_commitment(&self, res: &PostResponse) -> Result<Vec<u8>, Error> {
		let hash = hash_post_response::<Self>(res);
		let val = self.responses.borrow_mut().remove(&hash);
		Ok(val.encode())
	}

	fn delete_request_receipt(&self, req: &Request) -> Result<Vec<u8>, Error> {
		let hash = hash_request::<Self>(req);
		let val = self.receipts.borrow_mut().remove(&hash);
		Ok(val.encode())
	}

	fn delete_response_receipt(&self, res: &Response) -> Result<Vec<u8>, Error> {
		let hash = hash_request::<Self>(&res.request());
		let val = self.receipts.borrow_mut().remove(&hash);
		Ok(val.encode())
	}

	fn store_request_receipt(&self, req: &Request, _signer: &Vec<u8>) -> Result<Vec<u8>, Error> {
		let hash = hash_request::<Self>(req);
		self.receipts.borrow_mut().insert(hash, ());
		Ok(vec![])
	}

	fn store_response_receipt(&self, res: &Response, _signer: &Vec<u8>) -> Result<Vec<u8>, Error> {
		let hash = hash_response::<Self>(res);
		self.receipts.borrow_mut().insert(hash, ());
		Ok(vec![])
	}

	fn store_request_commitment(&self, req: &Request, _meta: Vec<u8>) -> Result<(), Error> {
		let hash = hash_request::<Self>(req);
		self.requests.borrow_mut().insert(hash);
		Ok(())
	}

	fn store_response_commitment(&self, res: &PostResponse, _meta: Vec<u8>) -> Result<(), Error> {
		let hash = hash_request::<Self>(&Request::Post(res.post.clone()));
		self.responses.borrow_mut().insert(hash);
		Ok(())
	}

	fn consensus_clients(&self) -> Vec<Box<dyn ConsensusClient>> {
		vec![Box::new(MockClient), Box::new(MockProxyClient)]
	}

	fn challenge_period(&self, _state_machine: StateMachineId) -> Option<Duration> {
		Some(Duration::from_secs(60 * 60))
	}

	fn store_challenge_period(
		&self,
		_state_machine: StateMachineId,
		_period: u64,
	) -> Result<(), Error> {
		Ok(())
	}

	fn allowed_proxy(&self) -> Option<StateMachine> {
		self.proxy.clone()
	}

	fn unbonding_period(&self, _consensus_state_id: ConsensusStateId) -> Option<Duration> {
		Some(Duration::from_secs(60 * 60 * 60))
	}

	fn ismp_router(&self) -> Box<dyn IsmpRouter> {
		Box::new(MockRouter(self.clone()))
	}

	fn previous_commitment_height(&self, id: StateMachineId) -> Option<u64> {
		self.previous_state_height.borrow().get(&id).copied()
	}
}

impl Keccak256 for Host {
	fn keccak256(bytes: &[u8]) -> H256
	where
		Self: Sized,
	{
		sp_core::keccak_256(bytes).into()
	}
}

#[derive(Default)]
pub struct MockModule;

impl IsmpModule for MockModule {
	fn on_accept(&self, _request: PostRequest) -> Result<Weight, anyhow::Error> {
		Ok(weight())
	}

	fn on_response(&self, _response: Response) -> Result<Weight, anyhow::Error> {
		Ok(weight())
	}

	fn on_timeout(&self, _request: Timeout) -> Result<Weight, anyhow::Error> {
		Ok(weight())
	}
}

fn weight() -> Weight {
	Weight::from_parts(0, 0)
}

pub struct MockRouter(pub Host);

impl IsmpRouter for MockRouter {
	fn module_for_id(&self, _bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
		Ok(Box::new(MockModule))
	}
}

pub struct MockDispatcher(pub Arc<Host>);

impl IsmpDispatcher for Host {
	type Account = Vec<u8>;
	type Balance = u32;

	fn dispatch_request(
		&self,
		request: DispatchRequest,
		_fee: FeeMetadata<Self::Account, Self::Balance>,
	) -> Result<H256, anyhow::Error> {
		let host = self.clone();
		let request = match request {
			DispatchRequest::Get(dispatch_get) => {
				let get = GetRequest {
					source: host.host_state_machine(),
					dest: dispatch_get.dest,
					nonce: host.next_nonce(),
					from: dispatch_get.from,
					keys: dispatch_get.keys,
					context: dispatch_get.context,

					height: dispatch_get.height,
					timeout_timestamp: dispatch_get.timeout,
				};
				Request::Get(get)
			},
			DispatchRequest::Post(dispatch_post) => {
				let post = PostRequest {
					source: host.host_state_machine(),
					dest: dispatch_post.dest,
					nonce: host.next_nonce(),
					from: dispatch_post.from,
					to: dispatch_post.to,
					timeout_timestamp: dispatch_post.timeout,
					body: dispatch_post.body,
				};
				Request::Post(post)
			},
		};
		let hash = hash_request::<Host>(&request);
		host.requests.borrow_mut().insert(hash);
		Ok(hash)
	}

	fn dispatch_response(
		&self,
		response: PostResponse,
		_fee: FeeMetadata<Self::Account, Self::Balance>,
	) -> Result<H256, anyhow::Error> {
		let host = self.clone();
		let response = Response::Post(response);
		let hash = hash_response::<Host>(&response);
		if host.responses.borrow().contains(&hash) {
			return Err(Error::Custom("Duplicate response".to_string()).into());
		}
		host.responses.borrow_mut().insert(hash);
		Ok(hash)
	}
}

pub struct Keccak256Hasher;

impl ismp::messaging::Keccak256 for Keccak256Hasher {
	fn keccak256(bytes: &[u8]) -> H256
	where
		Self: Sized,
	{
		sp_core::keccak_256(bytes).into()
	}
}
