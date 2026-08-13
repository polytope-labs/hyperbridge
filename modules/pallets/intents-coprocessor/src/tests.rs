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
	traits::{ConstU32, Everything, Hooks},
	BoundedBTreeMap, BoundedVec,
};
use frame_system::EnsureRoot;
use ismp::{consensus::StateMachineId, host::StateMachine};
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
	pub const PhantomOrderBidWindowBlocks: u32 = 100;
}

impl pallet_intents::Config for Test {
	type Dispatcher = Ismp;
	type Currency = Balances;
	type StorageDepositFee = StorageDepositFee;
	type PhantomOrderBidWindowBlocks = PhantomOrderBidWindowBlocks;
	type GovernanceOrigin = EnsureRoot<AccountId>;
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
fn upgrade_gateway_works() {
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

		assert_ok!(Intents::add_deployment(RuntimeOrigin::root(), state_machine, gateway, params));

		assert_ok!(Intents::upgrade_gateway(
			RuntimeOrigin::root(),
			state_machine,
			H160::repeat_byte(0x42),
			vec![0xDE, 0xAD],
		));
	});
}

#[test]
fn upgrade_gateway_fails_when_gateway_not_found() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Intents::upgrade_gateway(
				RuntimeOrigin::root(),
				StateMachine::Evm(1),
				H160::repeat_byte(0x42),
				vec![]
			),
			Error::<Test>::GatewayNotFound
		);
	});
}

#[test]
fn upgrade_gateway_requires_governance_origin() {
	new_test_ext().execute_with(|| {
		// A signed (non-governance) origin must be rejected before any gateway lookup.
		assert!(Intents::upgrade_gateway(
			RuntimeOrigin::signed(AccountId32::new([1; 32])),
			StateMachine::Evm(1),
			H160::repeat_byte(0x42),
			vec![]
		)
		.is_err());
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

fn phantom_pair(token_a: u8, token_b: u8) -> types::PhantomTokenPair {
	types::PhantomTokenPair {
		token_a: H160::repeat_byte(token_a),
		token_b: H160::repeat_byte(token_b),
		standard_amount: 1_000_000,
		standard_amount_b: 2_000_000,
	}
}

fn chain_id(evm_id: u32) -> StateMachineId {
	StateMachineId { state_id: StateMachine::Evm(evm_id), consensus_state_id: *b"ETH0" }
}

/// A configuration carrying one chain's pairs.
fn phantom_config(
	chain: StateMachineId,
	pairs: alloc::vec::Vec<types::PhantomTokenPair>,
	interval_blocks: u32,
) -> types::PhantomOrderConfiguration {
	phantom_config_chains(vec![(chain, pairs)], interval_blocks)
}

fn phantom_config_chains(
	chains: alloc::vec::Vec<(StateMachineId, alloc::vec::Vec<types::PhantomTokenPair>)>,
	interval_blocks: u32,
) -> types::PhantomOrderConfiguration {
	let chains = chains
		.into_iter()
		.map(|(chain, pairs)| (chain, BoundedVec::try_from(pairs).unwrap()))
		.collect::<alloc::collections::BTreeMap<_, _>>();
	types::PhantomOrderConfiguration {
		chains: BoundedBTreeMap::try_from(chains).unwrap(),
		interval_blocks,
	}
}

fn one_pair(chain: StateMachineId, interval_blocks: u32) -> types::PhantomOrderConfiguration {
	phantom_config(chain, vec![phantom_pair(1, 2)], interval_blocks)
}

#[test]
fn set_phantom_order_config_rejects_window_not_shorter_than_interval() {
	new_test_ext().execute_with(|| {
		// Default PhantomBidWindow is 0, so the effective window is the fallback constant (100).
		// An interval equal to or below the window must be rejected.
		assert_noop!(
			Intents::set_phantom_order_config(RuntimeOrigin::root(), one_pair(chain_id(1), 100)),
			Error::<Test>::PhantomBidWindowNotShorterThanInterval
		);
		assert_noop!(
			Intents::set_phantom_order_config(RuntimeOrigin::root(), one_pair(chain_id(1), 50)),
			Error::<Test>::PhantomBidWindowNotShorterThanInterval
		);

		// An interval strictly greater than the window is accepted.
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			one_pair(chain_id(1), 200)
		));

		// interval_blocks == 0 means generate-once (no regeneration), so the invariant is vacuous.
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			one_pair(chain_id(1), 0)
		));
	});
}

#[test]
fn set_phantom_bid_window_rejects_window_not_shorter_than_interval() {
	new_test_ext().execute_with(|| {
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			one_pair(chain_id(1), 200)
		));
		// Every chain generates on the one interval, so the check has a single value to make.
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			one_pair(chain_id(10), 200)
		));

		// A window equal to or above the shared interval must be rejected.
		assert_noop!(
			Intents::set_phantom_bid_window(RuntimeOrigin::root(), 200),
			Error::<Test>::PhantomBidWindowNotShorterThanInterval
		);
		assert_noop!(
			Intents::set_phantom_bid_window(RuntimeOrigin::root(), 250),
			Error::<Test>::PhantomBidWindowNotShorterThanInterval
		);

		// A window of 0 resolves to the fallback constant (100), which is < 200.
		assert_ok!(Intents::set_phantom_bid_window(RuntimeOrigin::root(), 0));
		// An explicit window below every interval is accepted.
		assert_ok!(Intents::set_phantom_bid_window(RuntimeOrigin::root(), 50));
	});
}

#[test]
fn set_phantom_bid_window_allows_any_window_when_no_config() {
	new_test_ext().execute_with(|| {
		// With no active config there is no interval to violate.
		assert_ok!(Intents::set_phantom_bid_window(RuntimeOrigin::root(), 1_000_000));
	});
}

#[test]
fn set_phantom_order_config_rejects_empty_and_duplicate_pairs() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Intents::set_phantom_order_config(
				RuntimeOrigin::root(),
				phantom_config(chain_id(1), vec![], 200)
			),
			Error::<Test>::EmptyPhantomTokenPairs
		);

		assert_noop!(
			Intents::set_phantom_order_config(
				RuntimeOrigin::root(),
				phantom_config(
					chain_id(1),
					vec![phantom_pair(1, 2), phantom_pair(3, 4), phantom_pair(1, 2)],
					200
				)
			),
			Error::<Test>::DuplicatePhantomTokenPair
		);

		// The generator expands every pair into both directions, so registering the reverse
		// explicitly would duplicate legs already in the order.
		assert_noop!(
			Intents::set_phantom_order_config(
				RuntimeOrigin::root(),
				phantom_config(chain_id(1), vec![phantom_pair(1, 2), phantom_pair(2, 1)], 200)
			),
			Error::<Test>::DuplicatePhantomTokenPair
		);

		let mut zero_amount = phantom_pair(1, 2);
		zero_amount.standard_amount = 0;
		assert_noop!(
			Intents::set_phantom_order_config(
				RuntimeOrigin::root(),
				phantom_config(chain_id(1), vec![zero_amount], 200)
			),
			Error::<Test>::ZeroPhantomStandardAmount
		);

		let mut zero_reverse = phantom_pair(1, 2);
		zero_reverse.standard_amount_b = 0;
		assert_noop!(
			Intents::set_phantom_order_config(
				RuntimeOrigin::root(),
				phantom_config(chain_id(1), vec![zero_reverse], 200)
			),
			Error::<Test>::ZeroPhantomStandardAmount
		);
	});
}

#[test]
fn phantom_generation_bundles_every_pair_into_one_order() {
	new_test_ext().execute_with(|| {
		let pairs = vec![phantom_pair(1, 2), phantom_pair(3, 4)];
		let chain = chain_id(1);
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(chain, pairs.clone(), 200)
		));

		// The hook reads the latest confirmed height and uses it as the order's deadline.
		pallet_ismp::LatestStateMachineHeight::<Test>::insert(chain, 42u64);

		System::set_block_number(1);
		System::reset_events();
		Intents::on_initialize(1);

		let active = CurrentPhantomOrder::<Test>::get().unwrap();
		assert_eq!(active.len(), 1);
		let (commitment, info) = active[0].clone();
		assert_eq!(info.created_at_block, 1);
		assert_eq!(info.chain, b"EVM-1".to_vec());

		// One commitment covers both directions of every configured pair.
		let expanded = types::phantom_order_legs(&pairs);
		let (expected, _) = types::phantom_order_commitment(1, b"EVM-1", &expanded, 42);
		assert_eq!(commitment, expected);

		let registered = System::events()
			.into_iter()
			.filter_map(|record| match record.event {
				RuntimeEvent::Intents(Event::PhantomOrderRegistered { legs, .. }) => Some(legs),
				_ => None,
			})
			.collect::<alloc::vec::Vec<_>>();
		assert_eq!(registered.len(), 1);
		assert_eq!(registered[0].to_vec(), expanded);
	});
}

#[test]
fn phantom_generation_emits_one_order_per_configured_chain() {
	new_test_ext().execute_with(|| {
		let (chain_a, pairs_a) = (chain_id(1), vec![phantom_pair(1, 2), phantom_pair(3, 4)]);
		let (chain_b, pairs_b) = (chain_id(10), vec![phantom_pair(5, 6)]);
		// The config carries a map, so one call can configure both chains at once.
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config_chains(
				vec![(chain_a, pairs_a.clone()), (chain_b, pairs_b.clone())],
				200
			)
		));

		pallet_ismp::LatestStateMachineHeight::<Test>::insert(chain_a, 42u64);
		pallet_ismp::LatestStateMachineHeight::<Test>::insert(chain_b, 77u64);

		System::set_block_number(1);
		System::reset_events();
		Intents::on_initialize(1);

		// One bundled order per chain, each committing to its own chain's legs and deadline.
		let active = CurrentPhantomOrder::<Test>::get().unwrap();
		assert_eq!(active.len(), 2);
		let legs_a = types::phantom_order_legs(&pairs_a);
		let legs_b = types::phantom_order_legs(&pairs_b);
		let (expected_a, _) = types::phantom_order_commitment(1, b"EVM-1", &legs_a, 42);
		let (expected_b, _) = types::phantom_order_commitment(1, b"EVM-10", &legs_b, 77);
		assert_eq!(active[0].0, expected_a);
		assert_eq!(active[0].1.chain, b"EVM-1".to_vec());
		assert_eq!(active[1].0, expected_b);
		assert_eq!(active[1].1.chain, b"EVM-10".to_vec());

		let registered = System::events()
			.into_iter()
			.filter_map(|record| match record.event {
				RuntimeEvent::Intents(Event::PhantomOrderRegistered {
					commitment, legs, ..
				}) => Some((commitment, legs.to_vec())),
				_ => None,
			})
			.collect::<alloc::vec::Vec<_>>();
		assert_eq!(registered, vec![(expected_a, legs_a), (expected_b, legs_b)]);

		// Both chains generated on the same block here, so each order signals its own exhaustion
		// when the shared window closes.
		let closing = 1 + 100;
		System::set_block_number(closing);
		System::reset_events();
		Intents::on_finalize(closing);
		let exhausted = System::events()
			.into_iter()
			.filter_map(|record| match record.event {
				RuntimeEvent::Intents(Event::PhantomBidWindowExhausted { commitment, .. }) =>
					Some(commitment),
				_ => None,
			})
			.collect::<alloc::vec::Vec<_>>();
		assert_eq!(exhausted, vec![expected_a, expected_b]);
	});
}

#[test]
fn every_chain_shares_one_generation_interval() {
	new_test_ext().execute_with(|| {
		let (chain_a, chain_b) = (chain_id(1), chain_id(10));
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(chain_a, vec![phantom_pair(1, 2)], 200)
		));
		// The interval rides on the call but is stored once, so the last value set is the one
		// every chain generates on.
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(chain_b, vec![phantom_pair(5, 6)], 300)
		));
		assert_eq!(PhantomOrderInterval::<Test>::get(), 300);
		pallet_ismp::LatestStateMachineHeight::<Test>::insert(chain_a, 42u64);
		pallet_ismp::LatestStateMachineHeight::<Test>::insert(chain_b, 77u64);

		System::set_block_number(1);
		Intents::on_initialize(1);
		let first = CurrentPhantomOrder::<Test>::get().unwrap();
		assert_eq!(first.len(), 2);
		assert_eq!(LastPhantomGeneration::<Test>::get(), Some(1));

		// Nothing regenerates before the shared interval elapses...
		System::set_block_number(300);
		Intents::on_initialize(300);
		assert_eq!(CurrentPhantomOrder::<Test>::get().unwrap(), first);

		// ...and when it does, every chain regenerates on the same block, so their bid windows
		// close together.
		System::set_block_number(301);
		Intents::on_initialize(301);
		let second = CurrentPhantomOrder::<Test>::get().unwrap();
		assert_eq!(second.len(), 2);
		assert_ne!(second[0].0, first[0].0);
		assert_ne!(second[1].0, first[1].0);
		assert_eq!(second[0].1.created_at_block, 301);
		assert_eq!(second[1].1.created_at_block, 301);
		assert_eq!(LastPhantomGeneration::<Test>::get(), Some(301));
	});
}

#[test]
fn set_phantom_order_config_leaves_other_chains_configured() {
	new_test_ext().execute_with(|| {
		let (reconfigured, untouched) = (chain_id(1), chain_id(10));
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(reconfigured, vec![phantom_pair(1, 2)], 200)
		));
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(untouched, vec![phantom_pair(5, 6)], 200)
		));
		pallet_ismp::LatestStateMachineHeight::<Test>::insert(reconfigured, 42u64);
		pallet_ismp::LatestStateMachineHeight::<Test>::insert(untouched, 77u64);

		System::set_block_number(1);
		Intents::on_initialize(1);
		assert_eq!(CurrentPhantomOrder::<Test>::get().unwrap().len(), 2);

		// Reconfiguring one chain rewrites only its pairs; the shared cycle restarts so no bid
		// can be placed against a commitment the new pairs no longer describe.
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(reconfigured, vec![phantom_pair(3, 4)], 200)
		));
		assert_eq!(
			PhantomOrderConfig::<Test>::get(reconfigured).unwrap().to_vec(),
			vec![phantom_pair(3, 4)]
		);
		assert_eq!(
			PhantomOrderConfig::<Test>::get(untouched).unwrap().to_vec(),
			vec![phantom_pair(5, 6)]
		);
		assert!(CurrentPhantomOrder::<Test>::get().is_none());
		assert!(LastPhantomGeneration::<Test>::get().is_none());

		// Both chains are back on the next block, the reconfigured one with its new pairs.
		System::set_block_number(2);
		Intents::on_initialize(2);
		let active = CurrentPhantomOrder::<Test>::get().unwrap();
		assert_eq!(active.len(), 2);
		let legs = types::phantom_order_legs(&[phantom_pair(3, 4)]);
		let (expected, _) = types::phantom_order_commitment(2, b"EVM-1", &legs, 42);
		assert_eq!(active[0].0, expected);
	});
}

#[test]
fn remove_phantom_order_config_stops_only_that_chain() {
	new_test_ext().execute_with(|| {
		let (removed, kept) = (chain_id(1), chain_id(10));
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(removed, vec![phantom_pair(1, 2)], 200)
		));
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(kept, vec![phantom_pair(5, 6)], 200)
		));
		pallet_ismp::LatestStateMachineHeight::<Test>::insert(removed, 42u64);
		pallet_ismp::LatestStateMachineHeight::<Test>::insert(kept, 77u64);

		System::set_block_number(1);
		Intents::on_initialize(1);
		let before = CurrentPhantomOrder::<Test>::get().unwrap();
		assert_eq!(before.len(), 2);

		assert_ok!(Intents::remove_phantom_order_config(RuntimeOrigin::root(), removed));
		assert!(PhantomOrderConfig::<Test>::get(removed).is_none());
		assert!(!PhantomChains::<Test>::get().contains(&removed));
		// Its active order goes with it, so no further bids can be placed against it, and the
		// chain that stays keeps the order it is still taking bids on.
		assert_eq!(CurrentPhantomOrder::<Test>::get().unwrap().to_vec(), vec![before[1].clone()]);
		assert_eq!(LastPhantomGeneration::<Test>::get(), Some(1));

		// The removed chain never generates again; the other stays on the shared cycle.
		System::set_block_number(201);
		Intents::on_initialize(201);
		let active = CurrentPhantomOrder::<Test>::get().unwrap();
		assert_eq!(active.len(), 1);
		assert_eq!(active[0].1.chain, b"EVM-10".to_vec());
		assert_eq!(active[0].1.created_at_block, 201);

		// Removing the last chain empties the batch rather than leaving an empty vec behind.
		assert_ok!(Intents::remove_phantom_order_config(RuntimeOrigin::root(), kept));
		assert!(CurrentPhantomOrder::<Test>::get().is_none());
		assert!(PhantomChains::<Test>::get().is_empty());
	});
}

#[test]
fn remove_phantom_order_config_rejects_unconfigured_chains() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Intents::remove_phantom_order_config(RuntimeOrigin::root(), chain_id(1)),
			Error::<Test>::PhantomChainNotConfigured
		);
	});
}

#[test]
fn phantom_generation_skips_chains_without_a_confirmed_height() {
	new_test_ext().execute_with(|| {
		let (chain_a, chain_b) = (chain_id(1), chain_id(10));
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(chain_a, vec![phantom_pair(1, 2)], 200)
		));
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(chain_b, vec![phantom_pair(5, 6)], 200)
		));

		// Only chain A has a confirmed height; chain B must be skipped, not sink the batch.
		pallet_ismp::LatestStateMachineHeight::<Test>::insert(chain_a, 42u64);

		System::set_block_number(1);
		Intents::on_initialize(1);

		let active = CurrentPhantomOrder::<Test>::get().unwrap();
		assert_eq!(active.len(), 1);
		assert_eq!(active[0].1.chain, b"EVM-1".to_vec());
		// The shared marker still advances: A generated, so the batch is not retried until the
		// next interval and B simply misses this cycle.
		assert_eq!(LastPhantomGeneration::<Test>::get(), Some(1));
	});
}

#[test]
fn phantom_generation_retries_when_no_chain_has_a_confirmed_height() {
	new_test_ext().execute_with(|| {
		let chain = chain_id(1);
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(chain, vec![phantom_pair(1, 2)], 200)
		));

		// No confirmed height anywhere: nothing is generated and the generation marker stays
		// clear, so the hook retries on the very next block instead of waiting an interval.
		System::set_block_number(1);
		Intents::on_initialize(1);
		assert!(CurrentPhantomOrder::<Test>::get().is_none());
		assert!(LastPhantomGeneration::<Test>::get().is_none());

		pallet_ismp::LatestStateMachineHeight::<Test>::insert(chain, 42u64);
		System::set_block_number(2);
		Intents::on_initialize(2);
		assert_eq!(CurrentPhantomOrder::<Test>::get().unwrap().len(), 1);
		assert_eq!(LastPhantomGeneration::<Test>::get(), Some(2));
	});
}

#[test]
fn set_phantom_order_config_rejects_a_second_consensus_state_id_per_chain() {
	new_test_ext().execute_with(|| {
		let chain = chain_id(1);
		assert_ok!(Intents::set_phantom_order_config(RuntimeOrigin::root(), one_pair(chain, 200)));

		// An order commits to the state machine, not the consensus state id, so a second entry
		// for the same chain would generate a colliding order.
		assert_noop!(
			Intents::set_phantom_order_config(
				RuntimeOrigin::root(),
				one_pair(
					StateMachineId { state_id: chain.state_id, consensus_state_id: *b"ETH1" },
					200
				)
			),
			Error::<Test>::DuplicatePhantomChain
		);

		// Two entries for it in the same call are rejected the same way.
		assert_noop!(
			Intents::set_phantom_order_config(
				RuntimeOrigin::root(),
				phantom_config_chains(
					vec![
						(chain_id(10), vec![phantom_pair(1, 2)]),
						(
							StateMachineId {
								state_id: StateMachine::Evm(10),
								consensus_state_id: *b"ETH1"
							},
							vec![phantom_pair(3, 4)]
						),
					],
					200
				)
			),
			Error::<Test>::DuplicatePhantomChain
		);

		// Re-setting the same chain under the same consensus state id is an update, not a
		// duplicate.
		assert_ok!(Intents::set_phantom_order_config(RuntimeOrigin::root(), one_pair(chain, 200)));
	});
}

#[test]
fn set_phantom_order_config_rejects_an_empty_chain_map() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Intents::set_phantom_order_config(
				RuntimeOrigin::root(),
				phantom_config_chains(vec![], 200)
			),
			Error::<Test>::EmptyPhantomChains
		);
	});
}

#[test]
fn set_phantom_order_config_rejects_more_than_max_chains() {
	new_test_ext().execute_with(|| {
		for evm_id in 0..types::MAX_PHANTOM_CHAINS {
			assert_ok!(Intents::set_phantom_order_config(
				RuntimeOrigin::root(),
				one_pair(chain_id(evm_id), 200)
			));
		}

		assert_noop!(
			Intents::set_phantom_order_config(
				RuntimeOrigin::root(),
				one_pair(chain_id(types::MAX_PHANTOM_CHAINS), 200)
			),
			Error::<Test>::TooManyPhantomChains
		);

		// Freeing a slot lets the next chain in.
		assert_ok!(Intents::remove_phantom_order_config(RuntimeOrigin::root(), chain_id(0)));
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			one_pair(chain_id(types::MAX_PHANTOM_CHAINS), 200)
		));
	});
}

#[test]
fn phantom_order_legs_expand_each_pair_into_both_directions() {
	let pairs = vec![phantom_pair(1, 2), phantom_pair(3, 4)];
	let legs = types::phantom_order_legs(&pairs);

	// Pair order is preserved and each pair's forward leg precedes its reverse, with the
	// reverse denominated in standard_amount_b.
	assert_eq!(legs.len(), 4);
	for (pair, chunk) in pairs.iter().zip(legs.chunks(2)) {
		assert_eq!((chunk[0].token_a, chunk[0].token_b), (pair.token_a, pair.token_b));
		assert_eq!(chunk[0].standard_amount, pair.standard_amount);
		assert_eq!((chunk[1].token_a, chunk[1].token_b), (pair.token_b, pair.token_a));
		assert_eq!(chunk[1].standard_amount, pair.standard_amount_b);
	}

	// A same-token pair has no distinct reverse, so it contributes a single leg.
	let same = types::phantom_order_legs(&[phantom_pair(5, 5)]);
	assert_eq!(same.len(), 1);
	assert_eq!((same[0].token_a, same[0].token_b), (H160::repeat_byte(5), H160::repeat_byte(5)));
	assert_eq!(same[0].standard_amount, 1_000_000);
}

#[test]
fn phantom_order_body_pairs_legs_positionally() {
	use alloy_sol_types::SolValue;

	// D1: leg i is inputs[i] against output.assets[i], with the input carrying that leg's
	// standard amount and the matching output left at zero. Fillers walk the legs this way and
	// the indexer keys snapshots by leg index, so both break silently if the encoder reorders
	// or misaligns them. The commitment assertion elsewhere cannot catch that: any consistent
	// encoding hashes consistently.
	let legs = types::phantom_order_legs(&vec![phantom_pair(1, 2), phantom_pair(3, 4)]);
	let (commitment, encoded) = types::phantom_order_commitment(7, b"EVM-8453", &legs, 42);

	// Round-trips through abi.decode(data, (Order)), which is what the SDK's fetchPhantomOrder
	// does with placeOrder's tuple input.
	let order = types::sol_types::Order::abi_decode(&encoded).expect("order round-trips");

	assert_eq!(commitment, H256(sp_io::hashing::keccak_256(&encoded)));
	assert_eq!(order.deadline, alloy_primitives::U256::from(42));
	assert_eq!(order.nonce, alloy_primitives::U256::from(7));
	assert_eq!(order.inputs.len(), legs.len());
	assert_eq!(order.output.assets.len(), legs.len());

	for (i, leg) in legs.iter().enumerate() {
		let mut token_a = [0u8; 32];
		token_a[12..].copy_from_slice(leg.token_a.as_bytes());
		let mut token_b = [0u8; 32];
		token_b[12..].copy_from_slice(leg.token_b.as_bytes());

		assert_eq!(order.inputs[i].token.as_slice(), &token_a, "input token at leg {i}");
		assert_eq!(
			order.inputs[i].amount,
			alloy_primitives::U256::from(leg.standard_amount),
			"input amount at leg {i}"
		);
		assert_eq!(order.output.assets[i].token.as_slice(), &token_b, "output token at leg {i}");
		// The output amount is what fillers quote, so the request leaves it at zero.
		assert_eq!(order.output.assets[i].amount, alloy_primitives::U256::ZERO);
	}
}

#[test]
fn phantom_order_at_max_pairs_stays_within_budget() {
	use codec::Encode;

	let pairs = (0..types::MAX_PHANTOM_TOKEN_PAIRS)
		.map(|i| types::PhantomTokenPair {
			token_a: H160::from_low_u64_be(i.into()),
			token_b: H160::from_low_u64_be((i + types::MAX_PHANTOM_TOKEN_PAIRS).into()),
			standard_amount: 1_000_000_000_000_000_000u128,
			standard_amount_b: 1_000_000_000_000_000_000u128,
		})
		.collect::<alloc::vec::Vec<_>>();

	let legs = types::phantom_order_legs(&pairs);
	assert_eq!(legs.len() as u32, types::MAX_PHANTOM_ORDER_LEGS);
	let (_, encoded) = types::phantom_order_commitment(1, b"EVM-8453", &legs, 42);
	let config = phantom_config(chain_id(8453), pairs, 200);
	let event_payload = (H256::repeat_byte(1), b"EVM-8453".to_vec(), 1u64, legs).encode();

	// The whole batch now rides in one order body, one config value and one event, so each has
	// to be sized against its own budget rather than against a single pair. Every pair expands
	// into both directions, so the order body and event carry twice the configured pair count.
	//
	// The order body only ever lives in offchain storage, which is local and unmetered, and is
	// fetched over RPC by fillers. The config is read by on_initialize on every block, so its
	// size is the recurring cost of the feature and the one worth watching. The event lands in
	// block storage and counts against the block's proof size.
	//
	// These numbers are what 64 pairs (128 legs) actually costs. They are asserted rather than
	// merely printed so raising MAX_PHANTOM_TOKEN_PAIRS or widening PhantomTokenPair has to come
	// back through here.
	assert!(encoded.len() < 32 * 1024, "encoded order body: {} bytes", encoded.len());
	assert!(config.encode().len() < 8 * 1024, "config value: {} bytes", config.encode().len());
	assert!(event_payload.len() < 8 * 1024, "event payload: {} bytes", event_payload.len());

	println!(
		"MAX_PHANTOM_TOKEN_PAIRS = {}: order body {} bytes, config {} bytes, event {} bytes",
		types::MAX_PHANTOM_TOKEN_PAIRS,
		encoded.len(),
		config.encode().len(),
		event_payload.len(),
	);
}

#[test]
fn phantom_bid_window_exhausted_fires_once_for_the_active_order() {
	new_test_ext().execute_with(|| {
		let chain = chain_id(1);
		assert_ok!(Intents::set_phantom_order_config(
			RuntimeOrigin::root(),
			phantom_config(chain, vec![phantom_pair(1, 2), phantom_pair(3, 4)], 200)
		));
		pallet_ismp::LatestStateMachineHeight::<Test>::insert(chain, 42u64);

		System::set_block_number(1);
		Intents::on_initialize(1);
		let (commitment, _) = CurrentPhantomOrder::<Test>::get().unwrap()[0].clone();

		// The default window storage is zero, so the fallback constant (100) applies.
		let closing = 1 + 100;
		System::set_block_number(closing);
		System::reset_events();
		Intents::on_finalize(closing);

		let exhausted = System::events()
			.into_iter()
			.filter_map(|record| match record.event {
				RuntimeEvent::Intents(Event::PhantomBidWindowExhausted { commitment, .. }) =>
					Some(commitment),
				_ => None,
			})
			.collect::<alloc::vec::Vec<_>>();
		assert_eq!(exhausted, vec![commitment]);
	});
}

// ── SimplexPaymaster governance ──────────────────────────────────────

fn sample_paymaster_params() -> types::PaymasterParams {
	types::PaymasterParams {
		native_oracle: H160::repeat_byte(0x01),
		markup_bps: U256::from(200),
		treasury: H160::repeat_byte(0x02),
		max_oracle_age: U256::from(90_000),
		swap_slippage_bps: U256::from(200),
	}
}

#[test]
fn add_paymaster_deployment_works() {
	new_test_ext().execute_with(|| {
		let state_machine = StateMachine::Evm(1);
		let paymaster = H160::repeat_byte(0xAA);
		assert_ok!(Intents::add_paymaster_deployment(
			RuntimeOrigin::root(),
			state_machine,
			paymaster
		));
		assert_eq!(Paymasters::<Test>::get(state_machine), Some(paymaster));
	});
}

#[test]
fn add_paymaster_deployment_requires_governance_origin() {
	new_test_ext().execute_with(|| {
		assert!(Intents::add_paymaster_deployment(
			RuntimeOrigin::signed(AccountId32::new([1; 32])),
			StateMachine::Evm(1),
			H160::repeat_byte(0xAA),
		)
		.is_err());
	});
}

#[test]
fn paymaster_actions_fail_when_not_registered() {
	new_test_ext().execute_with(|| {
		let sm = StateMachine::Evm(1);
		assert_noop!(
			Intents::upgrade_paymaster(RuntimeOrigin::root(), sm, H160::repeat_byte(0x42), vec![]),
			Error::<Test>::PaymasterNotFound
		);
		assert_noop!(
			Intents::update_paymaster_params(RuntimeOrigin::root(), sm, sample_paymaster_params()),
			Error::<Test>::PaymasterNotFound
		);
		assert_noop!(
			Intents::register_paymaster_token(
				RuntimeOrigin::root(),
				sm,
				H160::repeat_byte(0x11),
				H160::repeat_byte(0x22)
			),
			Error::<Test>::PaymasterNotFound
		);
		assert_noop!(
			Intents::deactivate_paymaster_token(RuntimeOrigin::root(), sm, H160::repeat_byte(0x11)),
			Error::<Test>::PaymasterNotFound
		);
		assert_noop!(
			Intents::withdraw_paymaster_assets(
				RuntimeOrigin::root(),
				sm,
				H160::zero(),
				U256::from(1)
			),
			Error::<Test>::PaymasterNotFound
		);
	});
}

#[test]
fn paymaster_governance_actions_dispatch() {
	new_test_ext().execute_with(|| {
		let sm = StateMachine::Evm(1);
		let paymaster = H160::repeat_byte(0xAA);
		assert_ok!(Intents::add_paymaster_deployment(RuntimeOrigin::root(), sm, paymaster));

		assert_ok!(Intents::upgrade_paymaster(
			RuntimeOrigin::root(),
			sm,
			H160::repeat_byte(0x42),
			vec![0xDE, 0xAD],
		));
		assert_ok!(Intents::update_paymaster_params(
			RuntimeOrigin::root(),
			sm,
			sample_paymaster_params()
		));
		assert_ok!(Intents::register_paymaster_token(
			RuntimeOrigin::root(),
			sm,
			H160::repeat_byte(0x11),
			H160::repeat_byte(0x22)
		));
		assert_ok!(Intents::deactivate_paymaster_token(
			RuntimeOrigin::root(),
			sm,
			H160::repeat_byte(0x11)
		));
		assert_ok!(Intents::withdraw_paymaster_assets(
			RuntimeOrigin::root(),
			sm,
			H160::zero(),
			U256::from(1_000_000)
		));
	});
}

#[test]
fn paymaster_actions_require_governance_origin() {
	new_test_ext().execute_with(|| {
		let sm = StateMachine::Evm(1);
		assert!(Intents::upgrade_paymaster(
			RuntimeOrigin::signed(AccountId32::new([1; 32])),
			sm,
			H160::repeat_byte(0x42),
			vec![]
		)
		.is_err());
	});
}
