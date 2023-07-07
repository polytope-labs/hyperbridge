//! Mocks used by both tests and benchmarks
use crate::primitives::ModuleId;
use alloc::collections::BTreeMap;
use frame_support::PalletId;
use ismp_rs::{
    consensus::{
        ConsensusClient, StateCommitment, StateMachineClient, StateMachineHeight, StateMachineId,
    },
    error::Error as IsmpError,
    handlers,
    host::{Ethereum, IsmpHost, StateMachine},
    messaging::{CreateConsensusState, Proof, StateCommitmentHeight},
    module::IsmpModule,
    router::{Post, Request, RequestResponse, Response},
};

pub const MOCK_CONSENSUS_STATE_ID: [u8; 4] = *b"mock";

/// module id for the mock benchmarking module
pub const MODULE_ID: ModuleId = ModuleId::Pallet(PalletId(*b"___mock_"));

fn set_timestamp<T: pallet_timestamp::Config>(value: u64)
where
    <T as pallet_timestamp::Config>::Moment: From<u64>,
{
    pallet_timestamp::Pallet::<T>::set_timestamp(value.into());
}

#[derive(Default)]
pub struct MockModule;

impl IsmpModule for MockModule {
    fn on_accept(&self, _request: Post) -> Result<(), ismp_rs::error::Error> {
        Ok(())
    }

    fn on_response(&self, _response: Response) -> Result<(), ismp_rs::error::Error> {
        Ok(())
    }

    fn on_timeout(&self, _request: Request) -> Result<(), ismp_rs::error::Error> {
        Ok(())
    }
}

/// A mock consensus client for benchmarking
#[derive(Default)]
pub struct MockConsensusClient;

impl ConsensusClient for MockConsensusClient {
    fn verify_consensus(
        &self,
        _host: &dyn IsmpHost,
        _cs_id: ismp_rs::consensus::ConsensusStateId,
        _trusted_consensus_state: Vec<u8>,
        _proof: Vec<u8>,
    ) -> Result<(Vec<u8>, BTreeMap<StateMachine, StateCommitmentHeight>), IsmpError> {
        Ok(Default::default())
    }

    fn verify_fraud_proof(
        &self,
        _host: &dyn IsmpHost,
        _trusted_consensus_state: Vec<u8>,
        _proof_1: Vec<u8>,
        _proof_2: Vec<u8>,
    ) -> Result<(), IsmpError> {
        Ok(())
    }

    fn state_machine(&self, _id: StateMachine) -> Result<Box<dyn StateMachineClient>, IsmpError> {
        Ok(Box::new(MockStateMachine))
    }
}

/// Mock State Machine
pub struct MockStateMachine;

impl StateMachineClient for MockStateMachine {
    fn verify_membership(
        &self,
        _host: &dyn IsmpHost,
        _item: RequestResponse,
        _root: StateCommitment,
        _proof: &Proof,
    ) -> Result<(), IsmpError> {
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
    ) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, IsmpError> {
        Ok(Default::default())
    }
}

pub fn setup_mock_client<H: IsmpHost, T: pallet_timestamp::Config>(host: &H) -> StateMachineHeight
where
    <T as pallet_timestamp::Config>::Moment: From<u64>,
{
    set_timestamp::<T>(1000_000);
    handlers::create_client(
        host,
        CreateConsensusState {
            consensus_state: vec![],
            consensus_client_id: MOCK_CONSENSUS_STATE_ID,
            consensus_state_id: MOCK_CONSENSUS_STATE_ID,
            unbonding_period: 1_000_000,
            state_machine_commitments: vec![(
                StateMachineId {
                    state_id: StateMachine::Ethereum(Ethereum::ExecutionLayer),
                    consensus_state_id: MOCK_CONSENSUS_STATE_ID,
                },
                StateCommitmentHeight {
                    commitment: StateCommitment {
                        timestamp: 1000,
                        overlay_root: None,
                        state_root: Default::default(),
                    },
                    height: 3,
                },
            )],
        },
    )
    .unwrap();

    set_timestamp::<T>(1000_000_000);
    StateMachineHeight {
        id: StateMachineId {
            state_id: StateMachine::Ethereum(Ethereum::ExecutionLayer),
            consensus_state_id: MOCK_CONSENSUS_STATE_ID,
        },
        height: 3,
    }
}
