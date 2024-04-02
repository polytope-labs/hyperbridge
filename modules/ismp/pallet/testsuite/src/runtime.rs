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
#![allow(missing_docs, dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU32, ConstU64, Get},
    PalletId,
};
use frame_system::{EnsureRoot, EventRecord};
use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, StateCommitment, StateMachineClient,
        StateMachineHeight, StateMachineId, VerifiedCommitments,
    },
    error::Error as IsmpError,
    handlers,
    host::{Ethereum, IsmpHost, StateMachine},
    messaging::{CreateConsensusState, Proof, StateCommitmentHeight},
    module::IsmpModule,
    router::{IsmpRouter, Post, RequestResponse, Response, Timeout},
};
use ismp_sync_committee::constants::sepolia::Sepolia;
use pallet_ismp::{host::Host, primitives::ModuleId};
use sp_core::{
    crypto::AccountId32,
    offchain::{testing::TestOffchainExt, OffchainDbExt, OffchainWorkerExt},
    H256,
};
use sp_runtime::{
    traits::{IdentityLookup, Keccak256},
    BuildStorage,
};
use substrate_state_machine::SubstrateStateMachine;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Ismp: pallet_ismp,
        Balances: pallet_balances,
        Relayer: pallet_ismp_relayer,
        Fishermen: pallet_fishermen,
    }
);

/// Verify the the last event emitted
pub fn assert_last_event<T: frame_system::Config>(generic_event: T::RuntimeEvent) {
    let events = frame_system::Pallet::<T>::events();
    dbg!(&events);
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &generic_event);
}

pub struct StateMachineProvider;

impl Get<StateMachine> for StateMachineProvider {
    fn get() -> StateMachine {
        StateMachine::Kusama(100)
    }
}

/// Balance of an account.
pub type Balance = u128;
// Unit = the base number of indivisible units for balances
pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLIUNIT: Balance = 1_000_000_000;
pub const MICROUNIT: Balance = 1_000_000;

/// The existential deposit. Set to 1/10 of the Connected Relay Chain.
pub const EXISTENTIAL_DEPOSIT: Balance = MILLIUNIT;

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Test {
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
    /// The type for recording an account's balance.
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = ();
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type MaxHolds = ConstU32<1>;
    type MaxFreezes = ();
}

impl pallet_fishermen::Config for Test {
    type RuntimeEvent = RuntimeEvent;
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = Keccak256;
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = ();
    type BlockWeights = ();
    type RuntimeTask = ();
    type BlockLength = ();
    type Version = ();
    type Nonce = u64;
    type Block = Block;
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<1>;
    type WeightInfo = ();
}

parameter_types! {
    pub const Coprocessor: Option<StateMachine> = None;
}

impl pallet_ismp::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    const INDEXING_PREFIX: &'static [u8] = b"ISMP";
    type AdminOrigin = EnsureRoot<AccountId32>;
    type HostStateMachine = StateMachineProvider;
    type Coprocessor = Coprocessor;
    type TimeProvider = Timestamp;
    type Router = ModuleRouter;
    type ConsensusClients = (
        MockConsensusClient,
        ismp_sync_committee::SyncCommitteeConsensusClient<Host<Test>, Sepolia>,
        ismp_bsc::BscClient<Host<Test>>,
    );
    type WeightProvider = ();
}

impl pallet_ismp_relayer::Config for Test {
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_ismp_host_executive::Config for Test {}

#[derive(Default)]
pub struct ModuleRouter;

impl IsmpRouter for ModuleRouter {
    fn module_for_id(&self, _bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, ismp::error::Error> {
        Ok(Box::new(MockModule))
    }
}

/// Mock consensus state id
pub const MOCK_CONSENSUS_STATE_ID: [u8; 4] = *b"mock";

pub const MOCK_CONSENSUS_CLIENT_ID: [u8; 4] = [1u8; 4];

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

    fn consensus_client_id(&self) -> ConsensusClientId {
        MOCK_CONSENSUS_CLIENT_ID
    }

    fn state_machine(&self, _id: StateMachine) -> Result<Box<dyn StateMachineClient>, IsmpError> {
        let state_machine: Box<dyn StateMachineClient> = match _id {
            StateMachine::Kusama(2000) | StateMachine::Kusama(2001) =>
                Box::new(SubstrateStateMachine::<Test>::default()),
            _ => Box::new(MockStateMachine),
        };
        Ok(state_machine)
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
            consensus_client_id: MOCK_CONSENSUS_CLIENT_ID,
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

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();
    register_offchain_ext(&mut ext);
    ext
}

fn register_offchain_ext(ext: &mut sp_io::TestExternalities) {
    let (offchain, _offchain_state) = TestOffchainExt::with_offchain_db(ext.offchain_db());
    ext.register_extension(OffchainDbExt::new(offchain.clone()));
    ext.register_extension(OffchainWorkerExt::new(offchain));
}
