//! Mocks used by both tests and benchmarks
use crate::primitives::ModuleId;
use alloc::collections::BTreeMap;
use frame_support::PalletId;
use ismp::{
    consensus::{
        ConsensusClient, StateCommitment, StateMachineClient, StateMachineHeight, StateMachineId,
        VerifiedCommitments,
    },
    error::Error as IsmpError,
    handlers,
    host::{Ethereum, IsmpHost, StateMachine},
    messaging::{CreateConsensusState, Proof, StateCommitmentHeight},
    module::IsmpModule,
    router::{Post, Request, RequestResponse, Response, Timeout},
};
use sp_core::H256;

/// Mock consensus state id
pub const MOCK_CONSENSUS_STATE_ID: [u8; 4] = *b"mock";

/// module id for the mock benchmarking module
pub const MODULE_ID: ModuleId = ModuleId::Pallet(PalletId(*b"__mock__"));

pub fn set_timestamp<T: pallet_timestamp::Config>(value: u64)
where
    <T as pallet_timestamp::Config>::Moment: From<u64>,
{
    pallet_timestamp::Pallet::<T>::set_timestamp(value.into());
}

/// Mock module
#[derive(Default)]
pub struct MockModule;

impl IsmpModule for MockModule {
    fn on_accept(&self, _request: Post) -> Result<(), ismp::error::Error> {
        Ok(())
    }

    fn on_response(&self, _response: Response) -> Result<(), ismp::error::Error> {
        Ok(())
    }

    fn on_timeout(&self, _request: Timeout) -> Result<(), ismp::error::Error> {
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
        _cs_id: ismp::consensus::ConsensusStateId,
        _trusted_consensus_state: Vec<u8>,
        _proof: Vec<u8>,
    ) -> Result<(Vec<u8>, VerifiedCommitments), IsmpError> {
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

    fn state_trie_key(&self, _request: RequestResponse) -> Vec<Vec<u8>> {
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

/// Mock client setup
pub fn setup_mock_client<H: IsmpHost, T: pallet_timestamp::Config>(host: &H) -> StateMachineHeight
where
    <T as pallet_timestamp::Config>::Moment: From<u64>,
{
    let number = frame_system::Pallet::<T>::block_number() + 1u32.into();

    frame_system::Pallet::<T>::reset_events();
    frame_system::Pallet::<T>::initialize(&number, &Default::default(), &Default::default());
    frame_system::Pallet::<T>::finalize();
    set_timestamp::<T>(1000_000);
    handlers::create_client(
        host,
        CreateConsensusState {
            consensus_state: vec![],
            consensus_client_id: MOCK_CONSENSUS_STATE_ID,
            consensus_state_id: MOCK_CONSENSUS_STATE_ID,
            unbonding_period: 1_000_000,
            challenge_period: 0,
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
    let height = StateMachineHeight {
        id: StateMachineId {
            state_id: StateMachine::Ethereum(Ethereum::ExecutionLayer),
            consensus_state_id: MOCK_CONSENSUS_STATE_ID,
        },
        height: 3,
    };
    host.store_state_machine_update_time(height, core::time::Duration::from_millis(1000_000))
        .unwrap();

    set_timestamp::<T>(1000_000_000);
    height
}
