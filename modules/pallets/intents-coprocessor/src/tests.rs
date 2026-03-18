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
use alloc::{collections::BTreeSet, vec};
use codec::Decode;
use frame_support::{
	assert_noop, assert_ok, parameter_types,
	traits::{ConstU32, Everything, Hooks},
	BoundedVec,
};
use frame_system::EnsureRoot;
use ismp::host::StateMachine;
use ismp_testsuite::mocks::MockRouter;

use polkadot_sdk::*;
use primitive_types::{H160, H256, U256};
use sp_core::H256 as SpH256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u64;
type AccountId = AccountId32;

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
	type MaxPriceEntries = ConstU32<100>;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(AccountId32::new([1; 32]), 10000),
			(AccountId32::new([2; 32]), 10000),
			(AccountId32::new([3; 32]), 10000),
		],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext: sp_io::TestExternalities = t.into();
	ext.execute_with(|| {
		pallet_intents::StorageDepositFee::<Test>::put(200u64);
		// 24 hours in milliseconds
		pallet_intents::PriceWindowDurationValue::<Test>::put(86_400_000u64);
		// Price deposit: 500 tokens
		pallet_intents::PriceDepositAmount::<Test>::put(500u64);
		// Lock duration: 10 blocks
		pallet_intents::PriceDepositLockDuration::<Test>::put(10u64);
	});
	ext
}

#[test]
fn place_bid_works() {
	new_test_ext().execute_with(|| {
		let filler = AccountId32::new([1; 32]);
		let commitment = H256::random();
		let user_op = BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap();

		// Place a bid
		assert_ok!(Intents::place_bid(
			RuntimeOrigin::signed(filler.clone()),
			commitment,
			user_op.clone()
		));

		// Verify bid was stored (deposit amount for discoverability and refunds)
		assert!(Bids::<Test>::contains_key(&commitment, &filler));
		assert_eq!(Bids::<Test>::get(&commitment, &filler), Some(Intents::storage_deposit_fee()));

		// Verify deposit was reserved
		assert_eq!(Balances::reserved_balance(&filler), Intents::storage_deposit_fee());
	});
}

#[test]
fn place_bid_fails_with_empty_user_op() {
	new_test_ext().execute_with(|| {
		let filler = AccountId32::new([1; 32]);
		let commitment = H256::random();
		let user_op = BoundedVec::try_from(vec![]).unwrap();

		assert_noop!(
			Intents::place_bid(RuntimeOrigin::signed(filler.clone()), commitment, user_op),
			Error::<Test>::InvalidUserOp
		);
	});
}

#[test]
fn filler_can_update_own_bid() {
	new_test_ext().execute_with(|| {
		let filler = AccountId32::new([1; 32]);
		let commitment = H256::random();
		let user_op_1 = BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap();
		let user_op_2 = BoundedVec::try_from(vec![4u8, 5u8, 6u8]).unwrap();

		// Place first bid
		assert_ok!(Intents::place_bid(
			RuntimeOrigin::signed(filler.clone()),
			commitment,
			user_op_1.clone()
		));

		// Verify bid exists
		assert!(Bids::<Test>::contains_key(&commitment, &filler));
		assert_eq!(Balances::reserved_balance(&filler), Intents::storage_deposit_fee());

		// Update the bid with new user_op
		assert_ok!(Intents::place_bid(
			RuntimeOrigin::signed(filler.clone()),
			commitment,
			user_op_2.clone()
		));

		// Verify bid still exists and deposit is still reserved (only once)
		assert!(Bids::<Test>::contains_key(&commitment, &filler));
		assert_eq!(Balances::reserved_balance(&filler), Intents::storage_deposit_fee());
	});
}

#[test]
fn place_bid_fails_with_insufficient_balance() {
	new_test_ext().execute_with(|| {
		let filler = AccountId32::new([4; 32]); // Account with 0 balance
		let commitment = H256::random();
		let user_op = BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap();

		assert_noop!(
			Intents::place_bid(RuntimeOrigin::signed(filler.clone()), commitment, user_op),
			Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn retract_bid_works() {
	new_test_ext().execute_with(|| {
		let filler = AccountId32::new([1; 32]);
		let commitment = H256::random();
		let user_op = BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap();

		// Place a bid first
		assert_ok!(Intents::place_bid(RuntimeOrigin::signed(filler.clone()), commitment, user_op));

		assert!(Bids::<Test>::contains_key(&commitment, &filler));

		// Retract the bid
		assert_ok!(Intents::retract_bid(RuntimeOrigin::signed(filler.clone()), commitment));

		// Verify bid was removed
		assert!(!Bids::<Test>::contains_key(&commitment, &filler));
	});
}

#[test]
fn retract_bid_fails_when_not_found() {
	new_test_ext().execute_with(|| {
		let filler = AccountId32::new([1; 32]);
		let commitment = H256::random();

		assert_noop!(
			Intents::retract_bid(RuntimeOrigin::signed(filler.clone()), commitment),
			Error::<Test>::BidNotFound
		);
	});
}

#[test]
fn retract_bid_fails_when_not_owner() {
	new_test_ext().execute_with(|| {
		let filler = AccountId32::new([1; 32]);
		let other = AccountId32::new([2; 32]);
		let commitment = H256::random();
		let user_op = BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap();

		// Place a bid
		assert_ok!(Intents::place_bid(RuntimeOrigin::signed(filler.clone()), commitment, user_op));

		// Try to retract with different account
		assert_noop!(
			Intents::retract_bid(RuntimeOrigin::signed(other.clone()), commitment),
			Error::<Test>::BidNotFound
		);
	});
}

#[test]
fn add_deployment_works() {
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

		assert_ok!(Intents::add_deployment(
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
fn add_deployment_requires_root() {
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
			Intents::add_deployment(
				RuntimeOrigin::signed(AccountId32::new([1; 32])),
				state_machine,
				gateway,
				params
			),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn add_deployment_notifies_existing_gateways() {
	new_test_ext().execute_with(|| {
		let state_machine_1 = StateMachine::Evm(1);
		let state_machine_2 = StateMachine::Evm(2);
		let gateway_1 = H160::from_low_u64_be(1);
		let gateway_2 = H160::from_low_u64_be(2);
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};

		// Add first gateway deployment
		assert_ok!(Intents::add_deployment(
			RuntimeOrigin::root(),
			state_machine_1,
			gateway_1,
			params.clone()
		));

		// Add second gateway deployment on different state machine
		// This should notify the first gateway about the new deployment
		assert_ok!(Intents::add_deployment(
			RuntimeOrigin::root(),
			state_machine_2,
			gateway_2,
			params.clone()
		));

		// Verify both gateways exist
		assert!(Gateways::<Test>::contains_key(state_machine_1));
		assert!(Gateways::<Test>::contains_key(state_machine_2));

		// Add third gateway with same address as first
		// The logic should skip notifying state_machine_1 (same address)
		// and notify state_machine_2 (different address)
		let state_machine_3 = StateMachine::Evm(3);
		assert_ok!(Intents::add_deployment(
			RuntimeOrigin::root(),
			state_machine_3,
			gateway_1, // Same address as gateway on state_machine_1
			params
		));

		// Verify the third gateway exists
		assert!(Gateways::<Test>::contains_key(state_machine_3));
	});
}

#[test]
fn update_params_works() {
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
		assert_ok!(Intents::add_deployment(
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

		assert_ok!(Intents::update_params(
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
fn update_params_fails_when_gateway_not_found() {
	new_test_ext().execute_with(|| {
		let state_machine = StateMachine::Evm(1);

		let params_update =
			types::ParamsUpdate { protocol_fee_bps: Some(U256::from(200)), ..Default::default() };

		assert_noop!(
			Intents::update_params(RuntimeOrigin::root(), state_machine, params_update),
			Error::<Test>::GatewayNotFound
		);
	});
}

#[test]
fn update_token_decimals_works() {
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
		assert_ok!(Intents::add_deployment(RuntimeOrigin::root(), state_machine, gateway, params));

		let updates = vec![types::TokenDecimalsUpdate {
			source_chain: vec![1u8; 10],
			tokens: vec![types::TokenDecimal { token: H160::default(), decimals: 18 }],
		}];

		assert_ok!(Intents::update_token_decimals(RuntimeOrigin::root(), state_machine, updates));
	});
}

#[test]
fn update_token_decimals_fails_when_gateway_not_found() {
	new_test_ext().execute_with(|| {
		let state_machine = StateMachine::Evm(1);

		let updates = vec![types::TokenDecimalsUpdate {
			source_chain: vec![1u8; 10],
			tokens: vec![types::TokenDecimal { token: H160::default(), decimals: 18 }],
		}];

		assert_noop!(
			Intents::update_token_decimals(RuntimeOrigin::root(), state_machine, updates),
			Error::<Test>::GatewayNotFound
		);
	});
}

#[test]
fn multiple_fillers_can_bid_on_same_order() {
	new_test_ext().execute_with(|| {
		let filler1 = AccountId32::new([1; 32]);
		let filler2 = AccountId32::new([2; 32]);
		let commitment = H256::random();
		let user_op = BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap();

		// Both fillers place bids
		assert_ok!(Intents::place_bid(
			RuntimeOrigin::signed(filler1.clone()),
			commitment,
			user_op.clone()
		));

		assert_ok!(Intents::place_bid(RuntimeOrigin::signed(filler2.clone()), commitment, user_op));

		// Verify both bids exist
		assert!(Bids::<Test>::contains_key(&commitment, &filler1));
		assert!(Bids::<Test>::contains_key(&commitment, &filler2));
	});
}

#[test]
fn remove_recognized_pair_works() {
	new_test_ext().execute_with(|| {
		let pair_id =
			types::TokenPair { base: b"TOKEN_A".to_vec(), quote: b"TOKEN_B".to_vec() }.pair_id();

		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id));

		Prices::<Test>::insert(
			&pair_id,
			BTreeSet::from([types::PriceEntry {
				range_start: U256::zero(),
				range_end: U256::from(999),
				price: U256::from(1000),
				filler: H256::from_low_u64_be(1),
			}]),
		);

		assert_ok!(Intents::remove_recognized_pair(RuntimeOrigin::root(), pair_id));

		assert!(!RecognizedPairs::<Test>::get(&pair_id));
		assert!(Prices::<Test>::get(&pair_id).is_empty());
	});
}

#[test]
fn submit_pair_price_reserves_deposit_on_first_submission() {
	new_test_ext().execute_with(|| {
		let submitter = AccountId32::new([1; 32]);

		let pair_id =
			types::TokenPair { base: b"TOKEN_A".to_vec(), quote: b"TOKEN_B".to_vec() }.pair_id();
		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id));

		pallet_timestamp::Now::<Test>::put(2_000_000u64);

		let balance_before = Balances::free_balance(&submitter);
		let deposit_amount = PriceDepositAmount::<Test>::get();

		let entries = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::zero(),
			range_end: U256::from(999),
			price: U256::from(2000),
		}])
		.unwrap();

		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
			entries,
		));

		// Deposit was reserved
		assert_eq!(Balances::free_balance(&submitter), balance_before - deposit_amount);
		assert_eq!(Balances::reserved_balance(&submitter), deposit_amount);

		// Deposit record stored (no unlock block yet)
		let (stored_amount, unlock_block) =
			PriceDeposits::<Test>::get(&submitter, &pair_id).unwrap();
		assert_eq!(stored_amount, deposit_amount);
		assert_eq!(unlock_block, None);

		// Price entry stored
		let prices = Prices::<Test>::get(&pair_id);
		assert_eq!(prices.len(), 1);
		assert_eq!(prices.iter().next().unwrap().price, U256::from(2000));
	});
}

#[test]
fn submit_pair_price_second_submission_is_free() {
	new_test_ext().execute_with(|| {
		let submitter = AccountId32::new([1; 32]);

		let pair_id =
			types::TokenPair { base: b"TOKEN_A".to_vec(), quote: b"TOKEN_B".to_vec() }.pair_id();
		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id));

		pallet_timestamp::Now::<Test>::put(2_000_000u64);

		let entries1 = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::zero(),
			range_end: U256::from(999),
			price: U256::from(2000),
		}])
		.unwrap();

		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
			entries1,
		));

		let deposit_amount = PriceDepositAmount::<Test>::get();
		let balance_after_first = Balances::free_balance(&submitter);

		// Second submission — no additional deposit
		let entries2 = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::from(1000),
			range_end: U256::from(5000),
			price: U256::from(3000),
		}])
		.unwrap();

		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
			entries2,
		));

		// Balance unchanged (no extra deposit)
		assert_eq!(Balances::free_balance(&submitter), balance_after_first);
		// Still only one deposit reserved
		assert_eq!(Balances::reserved_balance(&submitter), deposit_amount);

		// Two entries now stored
		let prices = Prices::<Test>::get(&pair_id);
		assert_eq!(prices.len(), 2);
	});
}

#[test]
fn submit_pair_price_fails_with_insufficient_balance() {
	new_test_ext().execute_with(|| {
		let submitter = AccountId32::new([4; 32]); // no balance

		let pair_id =
			types::TokenPair { base: b"TOKEN_A".to_vec(), quote: b"TOKEN_B".to_vec() }.pair_id();
		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id));

		pallet_timestamp::Now::<Test>::put(2_000_000u64);

		let entries = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::zero(),
			range_end: U256::from(999),
			price: U256::from(2000),
		}])
		.unwrap();

		assert_noop!(
			Intents::submit_pair_price(RuntimeOrigin::signed(submitter), pair_id, entries,),
			Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn withdraw_price_deposit_two_phase() {
	new_test_ext().execute_with(|| {
		let submitter = AccountId32::new([1; 32]);

		let pair_id =
			types::TokenPair { base: b"TOKEN_A".to_vec(), quote: b"TOKEN_B".to_vec() }.pair_id();
		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id));

		pallet_timestamp::Now::<Test>::put(2_000_000u64);

		let entries = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::zero(),
			range_end: U256::from(999),
			price: U256::from(2000),
		}])
		.unwrap();

		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
			entries,
		));

		let deposit_amount = PriceDepositAmount::<Test>::get();
		let balance_after_submit = Balances::free_balance(&submitter);

		// Phase 1: Initiate withdrawal at block 1
		System::set_block_number(1);
		assert_ok!(Intents::withdraw_price_deposit(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
		));

		// Deposit is NOT yet unreserved
		assert_eq!(Balances::free_balance(&submitter), balance_after_submit);
		assert_eq!(Balances::reserved_balance(&submitter), deposit_amount);

		// Unlock block is set (1 + 10 = 11)
		let (_, unlock_block) = PriceDeposits::<Test>::get(&submitter, &pair_id).unwrap();
		assert_eq!(unlock_block, Some(11u64));

		// Phase 2 too early: still locked at block 5
		System::set_block_number(5);
		assert_noop!(
			Intents::withdraw_price_deposit(RuntimeOrigin::signed(submitter.clone()), pair_id,),
			Error::<Test>::DepositStillLocked
		);

		// Phase 2: Complete withdrawal at block 11
		System::set_block_number(11);
		assert_ok!(Intents::withdraw_price_deposit(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
		));

		// Deposit unreserved
		assert_eq!(Balances::free_balance(&submitter), balance_after_submit + deposit_amount);
		assert_eq!(Balances::reserved_balance(&submitter), 0);

		// Deposit record removed
		assert!(PriceDeposits::<Test>::get(&submitter, &pair_id).is_none());
	});
}

#[test]
fn withdraw_price_deposit_phase2_fails_when_still_locked() {
	new_test_ext().execute_with(|| {
		let submitter = AccountId32::new([1; 32]);

		let pair_id =
			types::TokenPair { base: b"TOKEN_A".to_vec(), quote: b"TOKEN_B".to_vec() }.pair_id();
		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id));

		pallet_timestamp::Now::<Test>::put(2_000_000u64);

		let entries = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::zero(),
			range_end: U256::from(999),
			price: U256::from(2000),
		}])
		.unwrap();

		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
			entries,
		));

		// Phase 1: Initiate withdrawal at block 1
		System::set_block_number(1);
		assert_ok!(Intents::withdraw_price_deposit(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
		));

		// Phase 2: Try to complete at block 5 (unlock is at 11)
		System::set_block_number(5);
		assert_noop!(
			Intents::withdraw_price_deposit(RuntimeOrigin::signed(submitter), pair_id,),
			Error::<Test>::DepositStillLocked
		);
	});
}

#[test]
fn withdraw_price_deposit_fails_when_no_deposit() {
	new_test_ext().execute_with(|| {
		let submitter = AccountId32::new([1; 32]);
		let pair_id = H256::random();

		assert_noop!(
			Intents::withdraw_price_deposit(RuntimeOrigin::signed(submitter), pair_id,),
			Error::<Test>::DepositNotFound
		);
	});
}

#[test]
fn set_price_deposit_amount_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Intents::set_price_deposit_amount(RuntimeOrigin::root(), 1000u64));
		assert_eq!(PriceDepositAmount::<Test>::get(), 1000u64);
	});
}

#[test]
fn set_price_deposit_lock_duration_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Intents::set_price_deposit_lock_duration(RuntimeOrigin::root(), 100u64));
		assert_eq!(PriceDepositLockDuration::<Test>::get(), 100u64);
	});
}

#[test]
fn prices_persist_across_window_and_clear_on_first_submission() {
	new_test_ext().execute_with(|| {
		let submitter = AccountId32::new([1; 32]);
		let pair_id =
			types::TokenPair { base: b"TOKEN_A".to_vec(), quote: b"TOKEN_B".to_vec() }.pair_id();
		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id));

		// Simulate day 1: store some prices
		Prices::<Test>::insert(
			&pair_id,
			BTreeSet::from([types::PriceEntry {
				range_start: U256::zero(),
				range_end: U256::from(999),
				price: U256::from(1666),
				filler: H256::from_low_u64_be(1),
			}]),
		);

		// Window started at second 1000
		PriceWindowStart::<Test>::put(1000u64);

		// Before window expires: on_initialize does nothing to prices
		pallet_timestamp::Now::<Test>::put(50_000_000u64); // 50_000 seconds in ms
		Intents::on_initialize(1u64);

		assert_eq!(Prices::<Test>::get(&pair_id).len(), 1);

		// Advance past the window (1000 + 86_400 = 87_400 seconds)
		pallet_timestamp::Now::<Test>::put(90_000_000u64); // 90_000 seconds in ms
		Intents::on_initialize(2u64);

		// Prices still persist (readable until first new submission)
		assert_eq!(Prices::<Test>::get(&pair_id).len(), 1);
		assert_eq!(PriceWindowStart::<Test>::get(), 90_000);

		// Submit a new price — this is the first submission in the new window.
		// It should clear stale entries before adding the new one.
		let new_entries = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::zero(),
			range_end: U256::from(999),
			price: U256::from(2000),
		}])
		.unwrap();
		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
			new_entries,
		));

		// Old entries gone, only new entry remains
		let prices = Prices::<Test>::get(&pair_id);
		assert_eq!(prices.len(), 1);
		assert_eq!(prices.iter().next().unwrap().price, U256::from(2000));
	});
}

#[test]
fn price_entry_encoding_matches_rpc_tuple_decoding() {
	// The RPC decodes PriceEntry as Vec<(U256, U256, U256, H256)>.
	// Verify that PriceEntry's SCALE encoding is identical to the tuple encoding.
	use codec::Encode;

	let range_start = U256::zero();
	let range_end = U256::from(999);
	let price = U256::from(42_000);
	let filler = H256::from_low_u64_be(1);

	let entry = PriceEntry { range_start, range_end, price, filler };

	let entry_bytes = entry.encode();
	let tuple_bytes = (range_start, range_end, price, filler).encode();
	assert_eq!(entry_bytes, tuple_bytes, "PriceEntry SCALE encoding must match tuple encoding");

	// Also verify round-trip: encode as PriceEntry, decode as tuple
	type RpcTuple = (U256, U256, U256, H256);
	let entries = vec![entry];
	let encoded = entries.encode();
	let decoded: Vec<RpcTuple> = Decode::decode(&mut &encoded[..]).unwrap();
	assert_eq!(decoded.len(), 1);
	assert_eq!(decoded[0].0, range_start);
	assert_eq!(decoded[0].1, range_end);
	assert_eq!(decoded[0].2, price);
	assert_eq!(decoded[0].3, filler);
}

#[test]
fn price_entry_storage_roundtrip_via_raw_key() {
	new_test_ext().execute_with(|| {
		let pair_id =
			types::TokenPair { base: b"TOKEN_A".to_vec(), quote: b"TOKEN_B".to_vec() }.pair_id();

		let entry1 = types::PriceEntry {
			range_start: U256::zero(),
			range_end: U256::from(999),
			price: U256::from(2000),
			filler: H256::from_low_u64_be(1),
		};
		let entry2 = types::PriceEntry {
			range_start: U256::from(1000),
			range_end: U256::from(5000),
			price: U256::from(3000),
			filler: H256::from_low_u64_be(2),
		};

		Prices::<Test>::insert(&pair_id, BTreeSet::from([entry1.clone(), entry2.clone()]));

		// Build the storage key the same way the RPC does.
		let pallet_prefix = b"Intents";

		let mut key = Vec::new();
		key.extend_from_slice(&sp_io::hashing::twox_128(pallet_prefix));
		key.extend_from_slice(&sp_io::hashing::twox_128(b"Prices"));
		let pair_id_bytes = pair_id.as_bytes();
		key.extend_from_slice(&sp_io::hashing::blake2_128(pair_id_bytes));
		key.extend_from_slice(pair_id_bytes);

		let raw = sp_io::storage::get(&key).expect("Prices storage should exist");

		type RpcTuple = (U256, U256, U256, H256);
		let decoded: Vec<RpcTuple> = Decode::decode(&mut &raw[..]).unwrap();
		assert_eq!(decoded.len(), 2);
		assert_eq!(decoded[0].0, U256::zero());
		assert_eq!(decoded[0].1, U256::from(999));
		assert_eq!(decoded[0].2, U256::from(2000));
		assert_eq!(decoded[0].3, H256::from_low_u64_be(1));
		assert_eq!(decoded[1].0, U256::from(1000));
		assert_eq!(decoded[1].1, U256::from(5000));
		assert_eq!(decoded[1].2, U256::from(3000));
		assert_eq!(decoded[1].3, H256::from_low_u64_be(2));
	});
}

#[test]
fn multiple_submitters_independent_deposits() {
	new_test_ext().execute_with(|| {
		let submitter1 = AccountId32::new([1; 32]);
		let submitter2 = AccountId32::new([2; 32]);

		let pair_id =
			types::TokenPair { base: b"TOKEN_A".to_vec(), quote: b"TOKEN_B".to_vec() }.pair_id();
		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id));

		pallet_timestamp::Now::<Test>::put(2_000_000u64);

		let deposit_amount = PriceDepositAmount::<Test>::get();

		let entries1 = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::zero(),
			range_end: U256::from(999),
			price: U256::from(2000),
		}])
		.unwrap();

		let entries2 = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::from(1000),
			range_end: U256::from(4999),
			price: U256::from(2100),
		}])
		.unwrap();

		// Both submitters submit prices
		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter1.clone()),
			pair_id,
			entries1,
		));
		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter2.clone()),
			pair_id,
			entries2,
		));

		// Each has their own deposit
		assert_eq!(Balances::reserved_balance(&submitter1), deposit_amount);
		assert_eq!(Balances::reserved_balance(&submitter2), deposit_amount);

		// Two entries in prices
		assert_eq!(Prices::<Test>::get(&pair_id).len(), 2);
	});
}

#[test]
fn separate_deposits_per_pair() {
	new_test_ext().execute_with(|| {
		let submitter = AccountId32::new([1; 32]);

		let pair_id1 =
			types::TokenPair { base: b"TOKEN_A".to_vec(), quote: b"TOKEN_B".to_vec() }.pair_id();
		let pair_id2 =
			types::TokenPair { base: b"TOKEN_C".to_vec(), quote: b"TOKEN_D".to_vec() }.pair_id();
		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id1));
		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id2));

		pallet_timestamp::Now::<Test>::put(2_000_000u64);

		let deposit_amount = PriceDepositAmount::<Test>::get();

		let entries = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::zero(),
			range_end: U256::from(999),
			price: U256::from(2000),
		}])
		.unwrap();

		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id1,
			entries.clone(),
		));
		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id2,
			entries,
		));

		// Two deposits reserved (one per pair)
		assert_eq!(Balances::reserved_balance(&submitter), deposit_amount * 2);

		// Phase 1: Initiate both withdrawals at block 1
		System::set_block_number(1);
		assert_ok!(Intents::withdraw_price_deposit(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id1,
		));
		assert_ok!(Intents::withdraw_price_deposit(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id2,
		));

		// Phase 2: Complete both after lock duration (block 11)
		System::set_block_number(11);
		assert_ok!(Intents::withdraw_price_deposit(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id1,
		));
		assert_eq!(Balances::reserved_balance(&submitter), deposit_amount);

		assert_ok!(Intents::withdraw_price_deposit(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id2,
		));
		assert_eq!(Balances::reserved_balance(&submitter), 0);
	});
}

#[test]
fn submit_pair_price_blocked_after_withdrawal_initiated() {
	new_test_ext().execute_with(|| {
		let submitter: AccountId = AccountId::from([1u8; 32]);
		let deposit_amount = 100u64;

		let pair = types::TokenPair { base: b"TOKEN_X".to_vec(), quote: b"TOKEN_Y".to_vec() };
		let pair_id = pair.pair_id();

		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair_id));
		PriceDepositAmount::<Test>::put(deposit_amount);
		PriceDepositLockDuration::<Test>::put(10u64);

		let entries = BoundedVec::try_from(vec![PriceInput {
			range_start: U256::zero(),
			range_end: U256::from(1000),
			price: U256::from(42),
		}])
		.unwrap();

		// Submit prices(reserves deposit)
		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
			entries.clone(),
		));

		// Initiate withdrawal
		System::set_block_number(1);
		assert_ok!(Intents::withdraw_price_deposit(
			RuntimeOrigin::signed(submitter.clone()),
			pair_id,
		));

		// Submitting prices should now fail
		assert_noop!(
			Intents::submit_pair_price(RuntimeOrigin::signed(submitter.clone()), pair_id, entries,),
			Error::<Test>::WithdrawalInProgress
		);
	});
}
