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

use core::str::FromStr;
use fp_evm::{
    FeeCalculator, IsPrecompileResult, Precompile, PrecompileHandle, PrecompileResult,
    PrecompileSet,
};
use frame_support::{dispatch::Weight, parameter_types};

use frame_support::traits::{ConstU32, ConstU64, FindAuthor, Get};
use frame_system::EnsureRoot;
use ismp_rs::{
    consensus::{ConsensusClient, ConsensusClientId},
    error::Error,
    host::StateMachine,
    module::IsmpModule,
    router::IsmpRouter,
};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot, IdentityAddressMapping};

use crate::{
    module::EvmIsmpModule,
    precompiles::{
        IsmpGetDispatcher, IsmpPostDispatcher, IsmpResponseDispatcher, GET_REQUEST_DISPATCHER,
        POST_REQUEST_DISPATCHER, POST_RESPONSE_DISPATCHER,
    },
};
use pallet_ismp::{
    mocks::ismp::MockConsensusClient,
    primitives::{ConsensusClientProvider, ModuleId},
};
use sp_core::{H160, H256, U256};
use sp_runtime::{
    testing::Header,
    traits::{IdentityLookup, Keccak256},
    ConsensusEngineId,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
            Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
            Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
            Ismp: pallet_ismp::{Pallet, Storage, Call, Event<T>},
            EVM: pallet_evm::{Pallet, Call, Storage, Config, Event<T>},
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
    fn consensus_client(_id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
        Ok(Box::new(MockConsensusClient))
    }
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = Keccak256;
    type AccountId = H160;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1000;
}
impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Balance = u64;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type ReserveIdentifier = ();
    type HoldIdentifier = ();
    type FreezeIdentifier = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type MaxHolds = ();
    type MaxFreezes = ();
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<1>;
    type WeightInfo = ();
}

impl pallet_ismp::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    const INDEXING_PREFIX: &'static [u8] = b"ISMP";
    type AdminOrigin = EnsureRoot<H160>;
    type StateMachine = StateMachineProvider;
    type TimeProvider = Timestamp;
    type IsmpRouter = ModuleRouter;
    type ConsensusClientProvider = ConsensusProvider;
    type WeightInfo = ();
    type WeightProvider = ();
}

#[derive(Default)]
pub struct ModuleRouter;

impl IsmpRouter for ModuleRouter {
    fn module_for_id(&self, bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error> {
        let module_id = ModuleId::from_bytes(&bytes).unwrap();
        match module_id {
            ModuleId::Evm(_) => Ok(Box::new(EvmIsmpModule::<Test>::default())),
            _ => Err(Error::ImplementationSpecific("Module handler not found".to_string())),
        }
    }
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
    fn min_gas_price() -> (U256, Weight) {
        // Return some meaningful gas price and weight
        (10u128.into(), Weight::from_parts(7u64, 0))
    }
}

pub struct FindAuthorTruncated;
impl FindAuthor<H160> for FindAuthorTruncated {
    fn find_author<'a, I>(_digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        Some(H160::from_str("1234500000000000000000000000000000000000").unwrap())
    }
}
const BLOCK_GAS_LIMIT: u64 = 1_500_000_000;
const MAX_POV_SIZE: u64 = 5 * 1024 * 1024;

parameter_types! {
    pub BlockGasLimit: U256 = U256::from(BLOCK_GAS_LIMIT);
    pub const GasLimitPovSizeRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(MAX_POV_SIZE);
    pub WeightPerGas: Weight = Weight::from_parts(20_000, 0);
    pub MockPrecompiles: MockPrecompileSet = MockPrecompileSet;
}
impl pallet_evm::Config for Test {
    type FeeCalculator = FixedGasPrice;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;

    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type CallOrigin = EnsureAddressRoot<Self::AccountId>;

    type WithdrawOrigin = EnsureAddressNever<Self::AccountId>;
    type AddressMapping = IdentityAddressMapping;
    type Currency = Balances;

    type RuntimeEvent = RuntimeEvent;
    type PrecompilesType = MockPrecompileSet;
    type PrecompilesValue = MockPrecompiles;
    type ChainId = ();
    type BlockGasLimit = BlockGasLimit;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type OnChargeTransaction = ();
    type OnCreate = ();
    type FindAuthor = FindAuthorTruncated;
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type Timestamp = Timestamp;
    type WeightInfo = ();
}
/// Example PrecompileSet with only Identity precompile.
pub struct MockPrecompileSet;

impl PrecompileSet for MockPrecompileSet {
    /// Tries to execute a precompile in the precompile set.
    /// If the provided address is not a precompile, returns None.
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        let address = handle.code_address();
        if address == POST_REQUEST_DISPATCHER {
            return Some(IsmpPostDispatcher::<Test>::execute(handle))
        } else if address == GET_REQUEST_DISPATCHER {
            return Some(IsmpGetDispatcher::<Test>::execute(handle))
        } else if address == POST_RESPONSE_DISPATCHER {
            return Some(IsmpResponseDispatcher::<Test>::execute(handle))
        }

        None
    }

    /// Check if the given address is a precompile. Should only be called to
    /// perform the check while not executing the precompile afterward, since
    /// `execute` already performs a check internally.
    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: address == POST_REQUEST_DISPATCHER ||
                address == GET_REQUEST_DISPATCHER ||
                address == POST_RESPONSE_DISPATCHER,
            extra_cost: 0,
        }
    }
}
