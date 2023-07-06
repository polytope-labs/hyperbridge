use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
        StateMachineHeight, StateMachineId,
    },
    error::Error,
    host::{IsmpHost, StateMachine},
    messaging::{Proof, StateCommitmentHeight},
    module::IsmpModule,
    router::{
        DispatchRequest, Get, IsmpDispatcher, IsmpRouter, Post, PostResponse, Request,
        RequestResponse, Response,
    },
    util::{hash_request, hash_response},
};
use primitive_types::H256;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
    rc::Rc,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Default)]
pub struct MockClient;

pub const MOCK_CONSENSUS_CLIENT_ID: [u8; 4] = [1u8; 4];

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
    ) -> Result<(Vec<u8>, BTreeMap<StateMachine, StateCommitmentHeight>), Error> {
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

    fn state_machine(&self, _id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
        Ok(Box::new(MockStateMachineClient))
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

    fn state_trie_key(&self, _request: Vec<Request>) -> Vec<Vec<u8>> {
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

#[derive(Default, Clone)]
pub struct Host {
    requests: Rc<RefCell<BTreeSet<H256>>>,
    receipts: Rc<RefCell<HashMap<H256, ()>>>,
    responses: Rc<RefCell<BTreeSet<H256>>>,
    consensus_clients: Rc<RefCell<HashMap<ConsensusStateId, ConsensusClientId>>>,
    consensus_states: Rc<RefCell<HashMap<ConsensusStateId, Vec<u8>>>>,
    state_commitments: Rc<RefCell<HashMap<StateMachineHeight, StateCommitment>>>,
    consensus_update_time: Rc<RefCell<HashMap<ConsensusStateId, Duration>>>,
    frozen_state_machines: Rc<RefCell<HashMap<StateMachineId, StateMachineHeight>>>,
    latest_state_height: Rc<RefCell<HashMap<StateMachineId, u64>>>,
    nonce: Rc<RefCell<u64>>,
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
            .ok_or_else(|| Error::ImplementationSpecific("latest height not found".into()))
    }

    fn state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<StateCommitment, Error> {
        self.state_commitments
            .borrow()
            .get(&height)
            .cloned()
            .ok_or_else(|| Error::ImplementationSpecific("state commitment not found".into()))
    }

    fn consensus_update_time(&self, id: ConsensusStateId) -> Result<Duration, Error> {
        self.consensus_update_time
            .borrow()
            .get(&id)
            .copied()
            .ok_or_else(|| Error::ImplementationSpecific("Consensus update time not found".into()))
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
            .ok_or_else(|| Error::ImplementationSpecific("consensus state not found".into()))
    }

    fn timestamp(&self) -> Duration {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
    }

    fn is_state_machine_frozen(&self, machine: StateMachineHeight) -> Result<(), Error> {
        let val = self
            .frozen_state_machines
            .borrow()
            .get(&machine.id)
            .map(|frozen_height| machine.height >= frozen_height.height)
            .unwrap_or(false);
        if val {
            Err(Error::FrozenStateMachine { height: machine })?;
        }

        Ok(())
    }

    fn is_consensus_client_frozen(&self, _client: ConsensusStateId) -> Result<(), Error> {
        Ok(())
    }

    fn request_commitment(&self, hash: H256) -> Result<(), Error> {
        self.requests
            .borrow()
            .contains(&hash)
            .then_some(())
            .ok_or_else(|| Error::ImplementationSpecific("Request commitment not found".into()))
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

    fn response_receipt(&self, res: &Request) -> Option<()> {
        let hash = hash_request::<Self>(res);
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

    fn store_consensus_update_time(
        &self,
        id: ConsensusStateId,
        timestamp: Duration,
    ) -> Result<(), Error> {
        self.consensus_update_time.borrow_mut().insert(id, timestamp);
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

    fn freeze_state_machine(&self, height: StateMachineHeight) -> Result<(), Error> {
        self.frozen_state_machines.borrow_mut().insert(height.id, height);
        Ok(())
    }

    fn freeze_consensus_client(&self, _client: ConsensusStateId) -> Result<(), Error> {
        Ok(())
    }

    fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error> {
        self.latest_state_height.borrow_mut().insert(height.id, height.height);
        Ok(())
    }

    fn delete_request_commitment(&self, req: &Request) -> Result<(), Error> {
        let hash = hash_request::<Self>(req);
        self.requests.borrow_mut().remove(&hash);
        Ok(())
    }

    fn store_request_receipt(&self, req: &Request) -> Result<(), Error> {
        let hash = hash_request::<Self>(req);
        self.receipts.borrow_mut().insert(hash, ());
        Ok(())
    }

    fn store_response_receipt(&self, res: &Request) -> Result<(), Error> {
        let hash = hash_request::<Self>(res);
        self.receipts.borrow_mut().insert(hash, ());
        Ok(())
    }

    fn consensus_client(&self, id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
        match id {
            MOCK_CONSENSUS_CLIENT_ID => Ok(Box::new(MockClient)),
            _ => Err(Error::ImplementationSpecific("Client not found".to_string())),
        }
    }

    fn keccak256(bytes: &[u8]) -> H256
    where
        Self: Sized,
    {
        sp_core::keccak_256(bytes).into()
    }

    fn challenge_period(&self, _consensus_state_id: ConsensusStateId) -> Option<Duration> {
        Some(Duration::from_secs(60 * 60))
    }

    fn unbonding_period(&self, _consensus_state_id: ConsensusStateId) -> Option<Duration> {
        Some(Duration::from_secs(60 * 60 * 60))
    }

    fn ismp_router(&self) -> Box<dyn IsmpRouter> {
        Box::new(MockRouter(self.clone()))
    }
}

#[derive(Default)]
pub struct MockModule;

impl IsmpModule for MockModule {
    fn on_accept(&self, _request: Post) -> Result<(), Error> {
        Ok(())
    }

    fn on_response(&self, _response: Response) -> Result<(), Error> {
        Ok(())
    }

    fn on_timeout(&self, _request: Request) -> Result<(), Error> {
        Ok(())
    }
}

pub struct MockRouter(pub Host);

impl IsmpRouter for MockRouter {
    fn module_for_id(&self, _bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error> {
        Ok(Box::new(MockModule))
    }
}

pub struct MockDispatcher(pub Arc<Host>);

impl IsmpDispatcher for MockDispatcher {
    fn dispatch_request(&self, request: DispatchRequest) -> Result<(), Error> {
        let host = self.0.clone();
        let request = match request {
            DispatchRequest::Get(dispatch_get) => {
                let get = Get {
                    source_chain: host.host_state_machine(),
                    dest_chain: dispatch_get.dest_chain,
                    nonce: host.next_nonce(),
                    from: dispatch_get.from,
                    keys: dispatch_get.keys,
                    height: dispatch_get.height,
                    timeout_timestamp: dispatch_get.timeout_timestamp,
                };
                Request::Get(get)
            }
            DispatchRequest::Post(dispatch_post) => {
                let post = Post {
                    source_chain: host.host_state_machine(),
                    dest_chain: dispatch_post.dest_chain,
                    nonce: host.next_nonce(),
                    from: dispatch_post.from,
                    to: dispatch_post.to,
                    timeout_timestamp: dispatch_post.timeout_timestamp,
                    data: dispatch_post.data,
                };
                Request::Post(post)
            }
        };
        let hash = hash_request::<Host>(&request);
        host.requests.borrow_mut().insert(hash);
        Ok(())
    }

    fn dispatch_response(&self, response: PostResponse) -> Result<(), Error> {
        let host = self.0.clone();
        let response = Response::Post(response);
        let hash = hash_response::<Host>(&response);
        if host.responses.borrow().contains(&hash) {
            return Err(Error::ImplementationSpecific("Duplicate response".to_string()))
        }
        host.responses.borrow_mut().insert(hash);
        Ok(())
    }
}
