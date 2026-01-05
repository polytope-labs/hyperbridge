// Copyright (C) Polytope Labs Ltd.
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

//! Tests for pallet-intents

#![cfg(test)]

use crate::{self as pallet_intents, *};
use alloc::vec;
use frame_support::{
	assert_noop, assert_ok, parameter_types,
	traits::{ConstU32, Everything},
};
use frame_system::EnsureRoot;
use ismp::host::StateMachine;
use ismp_testsuite::mocks::MockRouter;

use polkadot_sdk::*;
use primitive_types::{H160, H256, U256};
use sp_core::H256 as SpH256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u64;
type AccountId = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Timestamp: pallet_timestamp,
		Ismp: pallet_ismp,
		Intents: pallet_intents,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = SpH256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type RuntimeTask = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
	type ExtensionsWeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const HostStateMachine: StateMachine = StateMachine::Polkadot(2000);
}

impl pallet_ismp::Config for Test {
	type AdminOrigin = EnsureRoot<AccountId>;
	type HostStateMachine = HostStateMachine;
	type Coprocessor = ();
	type TimestampProvider = Timestamp;
	type Router = MockRouter;
	type Balance = Balance;
	type Currency = Balances;
	type ConsensusClients = ();
	type OffchainDB = ();
	type FeeHandler = ();
}

parameter_types! {
	pub const StorageDepositFee: Balance = 100;
}

impl pallet_intents::Config for Test {
	type Dispatcher = Ismp;
	type Currency = Balances;
	type StorageDepositFee = StorageDepositFee;
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(1, 10000), (2, 10000), (3, 10000)],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

#[test]
fn place_bid_works() {
	new_test_ext().execute_with(|| {
		let filler = 1u64;
		let commitment = H256::random();
		let user_op = vec![1u8, 2u8, 3u8];

		// Place a bid
		assert_ok!(Intents::place_bid(RuntimeOrigin::signed(filler), commitment, user_op.clone()));

		// Verify bid was stored
		assert!(Bids::<Test>::contains_key(&commitment, &filler));

		// Verify deposit was reserved
		let bid = Bids::<Test>::get(&commitment, &filler).unwrap();
		assert_eq!(bid.filler, filler);
		assert_eq!(bid.commitment, commitment);
		assert_eq!(bid.user_op, user_op);
		assert_eq!(bid.deposit, StorageDepositFee::get());
	});
}

#[test]
fn place_bid_fails_with_empty_user_op() {
	new_test_ext().execute_with(|| {
		let filler = 1u64;
		let commitment = H256::random();
		let user_op = vec![];

		assert_noop!(
			Intents::place_bid(RuntimeOrigin::signed(filler), commitment, user_op),
			Error::<Test>::InvalidUserOp
		);
	});
}

#[test]
fn place_bid_fails_when_already_exists() {
	new_test_ext().execute_with(|| {
		let filler = 1u64;
		let commitment = H256::random();
		let user_op = vec![1u8, 2u8, 3u8];

		// Place first bid
		assert_ok!(Intents::place_bid(RuntimeOrigin::signed(filler), commitment, user_op.clone()));

		// Try to place same bid again
		assert_noop!(
			Intents::place_bid(RuntimeOrigin::signed(filler), commitment, user_op),
			Error::<Test>::BidAlreadyExists
		);
	});
}

#[test]
fn place_bid_fails_with_insufficient_balance() {
	new_test_ext().execute_with(|| {
		let filler = 4u64; // Account with 0 balance
		let commitment = H256::random();
		let user_op = vec![1u8, 2u8, 3u8];

		assert_noop!(
			Intents::place_bid(RuntimeOrigin::signed(filler), commitment, user_op),
			Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn retract_bid_works() {
	new_test_ext().execute_with(|| {
		let filler = 1u64;
		let commitment = H256::random();
		let user_op = vec![1u8, 2u8, 3u8];

		// Place a bid first
		assert_ok!(Intents::place_bid(RuntimeOrigin::signed(filler), commitment, user_op));

		assert!(Bids::<Test>::contains_key(&commitment, &filler));

		// Retract the bid
		assert_ok!(Intents::retract_bid(RuntimeOrigin::signed(filler), commitment));

		// Verify bid was removed
		assert!(!Bids::<Test>::contains_key(&commitment, &filler));
	});
}

#[test]
fn retract_bid_fails_when_not_found() {
	new_test_ext().execute_with(|| {
		let filler = 1u64;
		let commitment = H256::random();

		assert_noop!(
			Intents::retract_bid(RuntimeOrigin::signed(filler), commitment),
			Error::<Test>::BidNotFound
		);
	});
}

#[test]
fn retract_bid_fails_when_not_owner() {
	new_test_ext().execute_with(|| {
		let filler = 1u64;
		let other = 2u64;
		let commitment = H256::random();
		let user_op = vec![1u8, 2u8, 3u8];

		// Place a bid
		assert_ok!(Intents::place_bid(RuntimeOrigin::signed(filler), commitment, user_op));

		// Try to retract with different account
		assert_noop!(
			Intents::retract_bid(RuntimeOrigin::signed(other), commitment),
			Error::<Test>::BidNotFound
		);
	});
}

#[test]
fn add_gateway_deployment_works() {
	new_test_ext().execute_with(|| {
		let state_machine = StateMachine::Evm(1);
		let gateway = H160::default();
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};

		assert_ok!(Intents::add_gateway_deployment(
			RuntimeOrigin::root(),
			state_machine,
			gateway,
			params.clone()
		));

		// Verify gateway was stored
		let stored = Gateways::<Test>::get(state_machine).unwrap();
		assert_eq!(stored.gateway, gateway);
		assert_eq!(stored.params, params);
	});
}

#[test]
fn add_gateway_deployment_requires_root() {
	new_test_ext().execute_with(|| {
		let state_machine = StateMachine::Evm(1);
		let gateway = H160::default();
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};

		assert_noop!(
			Intents::add_gateway_deployment(
				RuntimeOrigin::signed(1),
				state_machine,
				gateway,
				params
			),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn update_gateway_params_works() {
	new_test_ext().execute_with(|| {
		let state_machine = StateMachine::Evm(1);
		let gateway = H160::default();
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};

		// Add gateway first
		assert_ok!(Intents::add_gateway_deployment(
			RuntimeOrigin::root(),
			state_machine,
			gateway,
			params.clone()
		));

		// Update params (only changing specific fields)
		let new_params = types::ParamsUpdate {
			solver_selection: Some(false),
			surplus_share_bps: Some(U256::from(3000)),
			protocol_fee_bps: Some(U256::from(50)),
			..Default::default()
		};

		assert_ok!(Intents::update_gateway_params(
			RuntimeOrigin::root(),
			state_machine,
			new_params.clone()
		));

		// Verify params were updated
		let stored = Gateways::<Test>::get(state_machine).unwrap();
		assert_eq!(stored.params.solver_selection, false);
		assert_eq!(stored.params.surplus_share_bps, U256::from(3000));
		assert_eq!(stored.params.protocol_fee_bps, U256::from(50));
	});
}

#[test]
fn update_gateway_params_fails_when_gateway_not_found() {
	new_test_ext().execute_with(|| {
		let state_machine = StateMachine::Evm(1);

		let params_update =
			types::ParamsUpdate { protocol_fee_bps: Some(U256::from(200)), ..Default::default() };

		assert_noop!(
			Intents::update_gateway_params(RuntimeOrigin::root(), state_machine, params_update),
			Error::<Test>::GatewayNotFound
		);
	});
}

#[test]
fn update_oracle_token_decimals_works() {
	new_test_ext().execute_with(|| {
		let state_machine = StateMachine::Evm(1);
		let gateway = H160::default();
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};

		// Add gateway first (which includes oracle address in params)
		assert_ok!(Intents::add_gateway_deployment(
			RuntimeOrigin::root(),
			state_machine,
			gateway,
			params
		));

		let updates = vec![types::TokenDecimalsUpdate {
			source_chain: vec![1u8; 10],
			tokens: vec![types::TokenDecimal { token: H160::default(), decimals: 18 }],
		}];

		assert_ok!(Intents::update_oracle_token_decimals(
			RuntimeOrigin::root(),
			state_machine,
			updates
		));
	});
}

#[test]
fn update_oracle_token_decimals_fails_when_gateway_not_found() {
	new_test_ext().execute_with(|| {
		let state_machine = StateMachine::Evm(1);

		let updates = vec![types::TokenDecimalsUpdate {
			source_chain: vec![1u8; 10],
			tokens: vec![types::TokenDecimal { token: H160::default(), decimals: 18 }],
		}];

		assert_noop!(
			Intents::update_oracle_token_decimals(RuntimeOrigin::root(), state_machine, updates),
			Error::<Test>::GatewayNotFound
		);
	});
}

#[test]
fn multiple_fillers_can_bid_on_same_order() {
	new_test_ext().execute_with(|| {
		let filler1 = 1u64;
		let filler2 = 2u64;
		let commitment = H256::random();
		let user_op = vec![1u8, 2u8, 3u8];

		// Both fillers place bids
		assert_ok!(Intents::place_bid(RuntimeOrigin::signed(filler1), commitment, user_op.clone()));

		assert_ok!(Intents::place_bid(RuntimeOrigin::signed(filler2), commitment, user_op));

		// Verify both bids exist
		assert!(Bids::<Test>::contains_key(&commitment, &filler1));
		assert!(Bids::<Test>::contains_key(&commitment, &filler2));
	});
}
