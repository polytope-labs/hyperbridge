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

//! Mock implementations for tests & benchmarks
#![allow(missing_docs, dead_code, unused_imports)]
pub mod mocks;
use frame_support::derive_impl;

use crate as pallet_ismp;
use crate::*;
use frame_support::parameter_types;

use crate::primitives::ConsensusClientProvider;
use frame_support::traits::{ConstU32, ConstU64, Get};
use frame_system::EnsureRoot;
use ismp::{consensus::ConsensusClient, module::IsmpModule, router::IsmpRouter};
use sp_core::crypto::AccountId32;

use ismp::host::StateMachine;
use mocks::{MockConsensusClient, MockModule};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{IdentityLookup, Keccak256},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Ismp: pallet_ismp::{Pallet, Storage, Call, Event<T>},
        Balances: pallet_balances,
    }
);

pub struct StateMachineProvider;

impl Get<StateMachine> for StateMachineProvider {
    fn get() -> StateMachine {
        StateMachine::Kusama(100)
    }
}

pub struct ConsensusProvider;

impl ConsensusClientProvider for ConsensusProvider {
    fn consensus_client(
        _id: ConsensusClientId,
    ) -> Result<Box<dyn ConsensusClient>, ismp::error::Error> {
        Ok(Box::new(MockConsensusClient))
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

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    const INDEXING_PREFIX: &'static [u8] = b"ISMP";
    type AdminOrigin = EnsureRoot<AccountId32>;
    type HostStateMachine = StateMachineProvider;
    type TimeProvider = Timestamp;
    type IsmpRouter = ModuleRouter;
    type ConsensusClientProvider = ConsensusProvider;
    type WeightInfo = ();
    type WeightProvider = ();
}

#[derive(Default)]
pub struct ModuleRouter;

impl IsmpRouter for ModuleRouter {
    fn module_for_id(&self, _bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, ismp::error::Error> {
        Ok(Box::new(MockModule))
    }
}
