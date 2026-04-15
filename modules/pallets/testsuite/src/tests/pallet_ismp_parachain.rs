#![cfg(test)]

use crate::runtime::{new_test_ext, Test};
use ismp_parachain::{
	pallet::{CurrentRelayChainStateRoots, OldestRetainedRelayBlock, RelayChainStateCommitments},
	Pallet as IsmpParachain, RelayChainOracle, LEGACY_DRAIN_BATCH_SIZE,
	MAX_RELAY_STATE_COMMITMENTS,
};
use pallet_ismp::{
	BoundedStateCommitments, BoundedStateMachineUpdateTime, OldestRetainedStateMachineHeight,
	StateCommitments, StateCommitmentsCount, StateMachineUpdateTime,
};
use polkadot_sdk::*;
use primitive_types::H256;

use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	host::StateMachine,
};

fn dummy_sm_id() -> StateMachineId {
	StateMachineId { state_id: StateMachine::Evm(1), consensus_state_id: *b"ETH0" }
}

fn dummy_height(h: u64) -> StateMachineHeight {
	StateMachineHeight { id: dummy_sm_id(), height: h }
}

fn dummy_commitment() -> StateCommitment {
	StateCommitment {
		timestamp: 1000,
		overlay_root: Some(H256::repeat_byte(0xAA)),
		state_root: H256::repeat_byte(0xBB),
	}
}

// ── RelayChainStateCommitments legacy drain ─────────────────────────────

#[test]
fn relay_drain_removes_up_to_batch_size() {
	new_test_ext().execute_with(|| {
		let total = LEGACY_DRAIN_BATCH_SIZE + 500;
		for i in 0..total {
			RelayChainStateCommitments::<Test>::insert(i + 1_000_000, H256::repeat_byte(i as u8));
		}

		IsmpParachain::<Test>::drain_legacy_commitments();

		let remaining = RelayChainStateCommitments::<Test>::iter_keys().count() as u32;
		assert_eq!(remaining, 500);
	});
}

#[test]
fn relay_drain_stops_when_empty() {
	new_test_ext().execute_with(|| {
		for i in 0..10u32 {
			RelayChainStateCommitments::<Test>::insert(i + 1_000_000, H256::repeat_byte(i as u8));
		}

		IsmpParachain::<Test>::drain_legacy_commitments();

		assert_eq!(RelayChainStateCommitments::<Test>::iter_keys().count(), 0);

		// calling again on empty is a no-op
		IsmpParachain::<Test>::drain_legacy_commitments();
	});
}

#[test]
fn relay_drain_does_not_touch_new_map() {
	new_test_ext().execute_with(|| {
		for i in 0..100u32 {
			RelayChainStateCommitments::<Test>::insert(i + 1_000_000, H256::repeat_byte(i as u8));
		}
		for i in 0..50u32 {
			CurrentRelayChainStateRoots::<Test>::insert(i + 5_000_000, H256::repeat_byte(i as u8));
		}
		OldestRetainedRelayBlock::<Test>::put(5_000_000);

		IsmpParachain::<Test>::drain_legacy_commitments();

		assert_eq!(CurrentRelayChainStateRoots::<Test>::count(), 50);
	});
}

// ── RelayChainStateRoots bounded eviction ───────────────────────────────

#[test]
fn relay_eviction_removes_oldest_and_advances_cursor() {
	new_test_ext().execute_with(|| {
		let start = 100u32;
		for i in 0..10u32 {
			CurrentRelayChainStateRoots::<Test>::insert(start + i, H256::repeat_byte(i as u8));
		}
		OldestRetainedRelayBlock::<Test>::put(start);

		IsmpParachain::<Test>::evict_oldest_relay_commitment(start + 10);

		assert_eq!(CurrentRelayChainStateRoots::<Test>::count(), 9);
		assert!(!CurrentRelayChainStateRoots::<Test>::contains_key(start));
		assert_eq!(OldestRetainedRelayBlock::<Test>::get(), Some(start + 1));
	});
}

#[test]
fn relay_eviction_skips_gaps() {
	new_test_ext().execute_with(|| {
		CurrentRelayChainStateRoots::<Test>::insert(100, H256::repeat_byte(1));
		CurrentRelayChainStateRoots::<Test>::insert(105, H256::repeat_byte(2));
		CurrentRelayChainStateRoots::<Test>::insert(110, H256::repeat_byte(3));
		OldestRetainedRelayBlock::<Test>::put(100);

		IsmpParachain::<Test>::evict_oldest_relay_commitment(200);
		assert!(!CurrentRelayChainStateRoots::<Test>::contains_key(100));
		assert_eq!(OldestRetainedRelayBlock::<Test>::get(), Some(101));

		IsmpParachain::<Test>::evict_oldest_relay_commitment(200);
		assert!(!CurrentRelayChainStateRoots::<Test>::contains_key(105));
		assert_eq!(OldestRetainedRelayBlock::<Test>::get(), Some(106));

		IsmpParachain::<Test>::evict_oldest_relay_commitment(200);
		assert!(!CurrentRelayChainStateRoots::<Test>::contains_key(110));
		assert_eq!(OldestRetainedRelayBlock::<Test>::get(), Some(111));
	});
}

// ── RelayChainOracle fallback ───────────────────────────────────────────

#[test]
fn oracle_prefers_new_map() {
	new_test_ext().execute_with(|| {
		let height = 500u32;
		let new_root = H256::repeat_byte(0xAA);
		let old_root = H256::repeat_byte(0xBB);

		CurrentRelayChainStateRoots::<Test>::insert(height, new_root);
		RelayChainStateCommitments::<Test>::insert(height, old_root);

		assert_eq!(IsmpParachain::<Test>::state_root(height), Some(new_root));
	});
}

#[test]
fn oracle_falls_back_to_legacy() {
	new_test_ext().execute_with(|| {
		let height = 500u32;
		let old_root = H256::repeat_byte(0xBB);

		RelayChainStateCommitments::<Test>::insert(height, old_root);

		assert_eq!(IsmpParachain::<Test>::state_root(height), Some(old_root));
	});
}

#[test]
fn oracle_returns_none_when_missing() {
	new_test_ext().execute_with(|| {
		assert_eq!(IsmpParachain::<Test>::state_root(999), None);
	});
}

// ── pallet-ismp StateCommitments legacy drain ───────────────────────────

#[test]
fn sm_drain_removes_entries_independently() {
	new_test_ext().execute_with(|| {
		for i in 0..100u64 {
			StateCommitments::<Test>::insert(dummy_height(i), dummy_commitment());
			StateMachineUpdateTime::<Test>::insert(dummy_height(i), 1000 + i);
		}

		pallet_ismp::Pallet::<Test>::drain_legacy_state_commitments();

		assert_eq!(StateCommitments::<Test>::iter_keys().count(), 0);
		assert_eq!(StateMachineUpdateTime::<Test>::iter_keys().count(), 0);
	});
}

#[test]
fn sm_drain_handles_unequal_counts() {
	new_test_ext().execute_with(|| {
		for i in 0..50u64 {
			StateCommitments::<Test>::insert(dummy_height(i), dummy_commitment());
		}
		for i in 0..200u64 {
			StateMachineUpdateTime::<Test>::insert(dummy_height(i), 1000 + i);
		}

		pallet_ismp::Pallet::<Test>::drain_legacy_state_commitments();

		assert_eq!(StateCommitments::<Test>::iter_keys().count(), 0);
		assert_eq!(StateMachineUpdateTime::<Test>::iter_keys().count(), 0);
	});
}

// ── pallet-ismp bounded state commitments eviction ──────────────────────

#[test]
fn bounded_sm_insert_and_evict() {
	new_test_ext().execute_with(|| {
		let sm_id = dummy_sm_id();

		for i in 0..300u64 {
			let height = StateMachineHeight { id: sm_id, height: i };
			pallet_ismp::Pallet::<Test>::insert_bounded_state_commitment(
				height,
				dummy_commitment(),
			);
			pallet_ismp::Pallet::<Test>::insert_bounded_update_time(height, 1000 + i);
		}

		// count should be capped at MAX
		assert!(
			StateCommitmentsCount::<Test>::get(sm_id) <= pallet_ismp::MAX_STATE_MACHINE_COMMITMENTS
		);

		// oldest entries should have been evicted
		assert!(!BoundedStateCommitments::<Test>::contains_key(sm_id, 0));
		assert!(!BoundedStateMachineUpdateTime::<Test>::contains_key(sm_id, 0));

		// recent entries should exist
		assert!(BoundedStateCommitments::<Test>::contains_key(sm_id, 299));
		assert!(BoundedStateMachineUpdateTime::<Test>::contains_key(sm_id, 299));
	});
}

#[test]
fn bounded_sm_eviction_is_per_chain() {
	new_test_ext().execute_with(|| {
		let chain_a =
			StateMachineId { state_id: StateMachine::Evm(1), consensus_state_id: *b"ETH0" };
		let chain_b =
			StateMachineId { state_id: StateMachine::Evm(2), consensus_state_id: *b"BSC0" };

		for i in 0..300u64 {
			pallet_ismp::Pallet::<Test>::insert_bounded_state_commitment(
				StateMachineHeight { id: chain_a, height: i },
				dummy_commitment(),
			);
		}

		for i in 0..10u64 {
			pallet_ismp::Pallet::<Test>::insert_bounded_state_commitment(
				StateMachineHeight { id: chain_b, height: i },
				dummy_commitment(),
			);
		}

		// chain_a capped, chain_b untouched
		assert!(
			StateCommitmentsCount::<Test>::get(chain_a) <=
				pallet_ismp::MAX_STATE_MACHINE_COMMITMENTS
		);
		assert_eq!(StateCommitmentsCount::<Test>::get(chain_b), 10);
	});
}
