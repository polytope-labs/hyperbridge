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
use alloc::{boxed::Box, collections::BTreeMap, vec};
use frame_support::{
	assert_noop, assert_ok, parameter_types,
	traits::{ConstU32, Everything, Hooks},
	BoundedVec,
};
use frame_system::EnsureRoot;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineHeight, StateMachineId, VerifiedCommitments,
	},
	error::Error as IsmpError,
	host::{IsmpHost, StateMachine},
	messaging::Proof,
	router::RequestResponse,
};
use ismp_testsuite::mocks::MockRouter;

use polkadot_sdk::*;
use primitive_types::{H160, H256, U256};
use sp_core::H256 as SpH256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, BuildStorage,
};

/// Mock consensus client ID
const MOCK_CONSENSUS_CLIENT_ID: ConsensusClientId = [1u8; 4];
/// Mock consensus state ID
const MOCK_CONSENSUS_STATE_ID: ConsensusStateId = *b"ETH0";

/// Height used for the non-membership proof (order not yet filled)
const H1_HEIGHT: u64 = 100;
/// Height used for the membership proof (order was filled)
const H2_HEIGHT: u64 = 200;

/// A mock consensus client for testing `submit_pair_price`.
///
/// Returns `MockPriceStateMachineClient` which encodes test behavior:
/// - At H1 (non-membership): returns `{key: None}` for storage queries
/// - At H2 (membership): returns `{key: Some(filler_address)}` from proof bytes
#[derive(Default)]
pub struct MockPriceConsensusClient;

impl ConsensusClient for MockPriceConsensusClient {
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		_consensus_state_id: ConsensusStateId,
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
		Ok(Box::new(MockPriceStateMachineClient))
	}
}

/// Mock state machine client that returns different results based on proof height.
///
/// - Height == H1_HEIGHT: returns `{key: None}` (non-membership)
/// - Height == H2_HEIGHT: returns `{key: Some(proof_bytes)}` (membership, proof bytes = filler
///   address)
pub struct MockPriceStateMachineClient;

impl StateMachineClient for MockPriceStateMachineClient {
	fn verify_membership(
		&self,
		_host: &dyn IsmpHost,
		_item: RequestResponse,
		_root: StateCommitment,
		_proof: &Proof,
	) -> Result<(), IsmpError> {
		Ok(())
	}

	fn receipts_state_trie_key(&self, _request: RequestResponse) -> Vec<Vec<u8>> {
		Default::default()
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		_root: StateCommitment,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, IsmpError> {
		let mut result = BTreeMap::new();
		let is_non_membership = proof.height.height == H1_HEIGHT;

		for key in keys {
			if is_non_membership {
				// Non-membership: value not present
				result.insert(key, None);
			} else {
				// Membership: value is the proof bytes (filler address padded to 32 bytes)
				result.insert(key, Some(proof.proof.clone()));
			}
		}

		Ok(result)
	}
}

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
	type ConsensusClients = (MockPriceConsensusClient,);
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
		// 1 hour in seconds
		pallet_intents::ProofFreshnessThresholdValue::<Test>::put(3600u64);
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
		let pair =
			types::TokenPair { base: H160::from_low_u64_be(1), quote: H160::from_low_u64_be(2) };
		let pair_id = pair.pair_id();

		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair));

		PriceAccumulators::<Test>::insert(
			&pair_id,
			types::PriceAccumulator { sum: U256::from(1000), count: 5 },
		);
		AveragePrice::<Test>::insert(&pair_id, U256::from(200));

		assert_ok!(Intents::remove_recognized_pair(RuntimeOrigin::root(), pair_id));

		// Verify clean up
		assert!(RecognizedPairs::<Test>::get(&pair_id).is_none());
		assert_eq!(AveragePrice::<Test>::get(&pair_id), U256::zero());
		assert_eq!(PriceAccumulators::<Test>::get(&pair_id).count, 0);
	});
}

#[test]
fn submit_pair_price() {
	new_test_ext().execute_with(|| {
		let filler = AccountId32::new([1; 32]);
		let state_machine = StateMachine::Evm(1);
		let commitment = H256::repeat_byte(0xaa);
		let price = U256::from(2000);

		//  Add a recognized token pair
		let pair =
			types::TokenPair { base: H160::from_low_u64_be(1), quote: H160::from_low_u64_be(2) };
		let pair_id = pair.pair_id();
		assert_ok!(Intents::add_recognized_pair(RuntimeOrigin::root(), pair));

		//  Add a gateway deployment
		let gateway = H160::from_low_u64_be(42);
		let params = types::IntentGatewayParams {
			host: H160::default(),
			dispatcher: H160::default(),
			solver_selection: true,
			surplus_share_bps: U256::from(5000),
			protocol_fee_bps: U256::from(100),
			price_oracle: H160::default(),
		};
		assert_ok!(Intents::add_deployment(RuntimeOrigin::root(), state_machine, gateway, params,));

		// Set up ISMP consensus state
		let sm_id =
			StateMachineId { state_id: state_machine, consensus_state_id: MOCK_CONSENSUS_STATE_ID };
		let h1 = StateMachineHeight { id: sm_id, height: H1_HEIGHT };
		let h2 = StateMachineHeight { id: sm_id, height: H2_HEIGHT };

		// Map consensus_state_id -> consensus_client_id
		pallet_ismp::ConsensusStateClient::<Test>::insert(
			MOCK_CONSENSUS_STATE_ID,
			MOCK_CONSENSUS_CLIENT_ID,
		);

		// Store consensus state
		pallet_ismp::ConsensusStates::<Test>::insert(MOCK_CONSENSUS_CLIENT_ID, vec![0u8]);

		// Store state commitments at both heights
		// H1 timestamp=1000, H2 timestamp=1050
		pallet_ismp::child_trie::StateCommitments::<Test>::insert(
			h1,
			StateCommitment {
				timestamp: 1000,
				overlay_root: None,
				state_root: H256::repeat_byte(0x11).into(),
			},
		);
		pallet_ismp::child_trie::StateCommitments::<Test>::insert(
			h2,
			StateCommitment {
				timestamp: 1050,
				overlay_root: None,
				state_root: H256::repeat_byte(0x22).into(),
			},
		);

		// Store challenge period
		pallet_ismp::ChallengePeriod::<Test>::insert(sm_id, 0u64);

		// Store state machine update times
		pallet_ismp::StateMachineUpdateTime::<Test>::insert(h1, 1000u64);
		pallet_ismp::StateMachineUpdateTime::<Test>::insert(h2, 1050u64);

		// Set the pallet timestamp so host.timestamp()
		pallet_timestamp::Now::<Test>::put(2_000_000u64); // 2000 seconds in ms

		// Build proofs
		// Non-membership proof: all zeros
		let non_membership_proof = Proof { height: h1, proof: vec![0u8; 32] };

		// Membership proof: proof bytes contain the filler address padded to 32 bytes
		// The code reads filler_bytes[12..32] as an H160 address
		let mut filler_bytes = vec![0u8; 32];
		// Put a non-zero address in bytes 12..32
		filler_bytes[12..32].copy_from_slice(&H160::from_low_u64_be(0xdeadbeef).0);
		let membership_proof = Proof { height: h2, proof: filler_bytes };

		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(filler.clone()),
			state_machine,
			commitment,
			pair_id,
			price,
			membership_proof,
			non_membership_proof,
		));

		// Verify the price accumulator was updated
		let acc = PriceAccumulators::<Test>::get(&pair_id);
		assert_eq!(acc.sum, price);
		assert_eq!(acc.count, 1);

		// Verify the average price was stored
		let avg = AveragePrice::<Test>::get(&pair_id);
		assert_eq!(avg, price); // With 1 submission, average = price

		// Submit a second price and verify the average updates
		let price2 = U256::from(4000);
		let commitment2 = H256::repeat_byte(0xbb);

		let non_membership_proof_2 = Proof { height: h1, proof: vec![0u8; 32] };
		let mut filler_bytes_2 = vec![0u8; 32];
		filler_bytes_2[12..32].copy_from_slice(&H160::from_low_u64_be(0xcafebabe).0);
		let membership_proof_2 = Proof { height: h2, proof: filler_bytes_2 };

		assert_ok!(Intents::submit_pair_price(
			RuntimeOrigin::signed(filler.clone()),
			state_machine,
			commitment2,
			pair_id,
			price2,
			membership_proof_2,
			non_membership_proof_2,
		));

		let acc2 = PriceAccumulators::<Test>::get(&pair_id);
		assert_eq!(acc2.sum, price.saturating_add(price2));
		assert_eq!(acc2.count, 2);

		let avg2 = AveragePrice::<Test>::get(&pair_id);
		assert_eq!(avg2, U256::from(3000));

		// 9. Reusing the same commitment should fail
		let non_membership_proof_dup = Proof { height: h1, proof: vec![0u8; 32] };
		let mut filler_bytes_dup = vec![0u8; 32];
		filler_bytes_dup[12..32].copy_from_slice(&H160::from_low_u64_be(0xdeadbeef).0);
		let membership_proof_dup = Proof { height: h2, proof: filler_bytes_dup };

		assert_noop!(
			Intents::submit_pair_price(
				RuntimeOrigin::signed(filler.clone()),
				state_machine,
				commitment, // same commitment as step 5
				pair_id,
				U256::from(9999),
				membership_proof_dup,
				non_membership_proof_dup,
			),
			Error::<Test>::CommitmentAlreadyUsed
		);
	});
}

#[test]
fn on_initialize_resets_accumulators_on_new_day() {
	new_test_ext().execute_with(|| {
		let pair =
			types::TokenPair { base: H160::from_low_u64_be(1), quote: H160::from_low_u64_be(2) };
		let pair_id = pair.pair_id();

		PriceAccumulators::<Test>::insert(
			&pair_id,
			types::PriceAccumulator { sum: U256::from(5000), count: 3 },
		);
		AveragePrice::<Test>::insert(&pair_id, U256::from(1666));

		// Window started at second 1000, duration is 86_400_000 ms = 86_400 s
		PriceWindowStart::<Test>::put(1000u64);

		pallet_timestamp::Now::<Test>::put(50_000_000u64); // 50_000 seconds in ms
		Intents::on_initialize(1u64);

		// Window has NOT expired, accumulators and average should be untouched
		let acc = PriceAccumulators::<Test>::get(&pair_id);
		assert_eq!(acc.count, 3);
		assert_eq!(acc.sum, U256::from(5000));
		assert_eq!(AveragePrice::<Test>::get(&pair_id), U256::from(1666));

		// Advance past the window (1000 + 86_400 = 87_400 seconds)
		pallet_timestamp::Now::<Test>::put(90_000_000u64); // 90_000 seconds in ms
		Intents::on_initialize(2u64);

		// Window expired, accumulators should be cleared but average price
		// from yesterday is preserved (readable until overwritten by new submissions)
		let acc = PriceAccumulators::<Test>::get(&pair_id);
		assert_eq!(acc.count, 0);
		assert_eq!(acc.sum, U256::zero());
		assert_eq!(AveragePrice::<Test>::get(&pair_id), U256::from(1666));

		// Window start should be updated
		assert_eq!(PriceWindowStart::<Test>::get(), 90_000);
	});
}
