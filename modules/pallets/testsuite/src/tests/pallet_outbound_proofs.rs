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

#![cfg(test)]
use polkadot_sdk::*;

use crate::runtime::{new_test_ext, set_mock_verified_proof, RuntimeOrigin, Test};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use pallet_outbound_proofs::{
	pallet::{Error, LatestMessageBlock, RecentProofs, BEEFY_CONSENSUS_ID},
	VerifiedProof,
};
use sp_runtime::AccountId32;

fn proof(data: &[u8]) -> BoundedVec<u8, <Test as pallet_outbound_proofs::Config>::MaxProofSize> {
	BoundedVec::try_from(data.to_vec()).unwrap()
}

fn setup_consensus_state() {
	pallet_ismp::ConsensusStates::<Test>::insert(BEEFY_CONSENSUS_ID, vec![0u8; 32]);
}

fn setup_mandatory_proof(relay_height: u32, para_height: u32, old_set_id: u64, new_set_id: u64) {
	setup_consensus_state();
	set_mock_verified_proof(VerifiedProof {
		relay_chain_height: relay_height,
		parachain_height: para_height,
		new_validator_set_id: new_set_id,
		old_validator_set_id: old_set_id,
		new_consensus_state: vec![0u8; 32],
	});
}

fn setup_non_mandatory_proof(relay_height: u32, para_height: u32, set_id: u64) {
	setup_consensus_state();
	set_mock_verified_proof(VerifiedProof {
		relay_chain_height: relay_height,
		parachain_height: para_height,
		new_validator_set_id: set_id,
		old_validator_set_id: set_id,
		new_consensus_state: vec![0u8; 32],
	});
}

fn submit(prover: AccountId32) -> frame_support::dispatch::DispatchResult {
	pallet_outbound_proofs::Pallet::<Test>::submit_proof(
		RuntimeOrigin::signed(prover),
		proof(&[0x01]),
	)
}

fn test_account() -> AccountId32 {
	AccountId32::new([1u8; 32])
}

fn other_account() -> AccountId32 {
	AccountId32::new([2u8; 32])
}

#[test]
fn mandatory_proof_accepted_without_messages() {
	new_test_ext().execute_with(|| {
		// relay=1000, para=500, epoch 0→1
		setup_mandatory_proof(1000, 500, 0, 1);
		assert_ok!(submit(test_account()));

		let recent = RecentProofs::<Test>::get();
		assert_eq!(recent.len(), 1);
		assert_eq!(recent[0].finalized_height, 1000);
		assert_eq!(recent[0].validator_set_id, 1);
	});
}

#[test]
fn mandatory_proof_advances_epoch() {
	new_test_ext().execute_with(|| {
		setup_mandatory_proof(1000, 500, 0, 1);
		assert_ok!(submit(test_account()));

		setup_mandatory_proof(2000, 1000, 1, 2);
		assert_ok!(submit(test_account()));

		setup_mandatory_proof(3000, 1500, 2, 3);
		assert_ok!(submit(test_account()));

		let recent = RecentProofs::<Test>::get();
		assert_eq!(recent.len(), 3);
		assert_eq!(recent[2].validator_set_id, 3);
	});
}

#[test]
fn mandatory_proof_rejected_if_already_proven_relay_height() {
	new_test_ext().execute_with(|| {
		setup_mandatory_proof(1000, 500, 0, 1);
		assert_ok!(submit(test_account()));

		// Same relay_chain_height
		setup_mandatory_proof(1000, 600, 1, 2);
		assert_noop!(submit(other_account()), Error::<Test>::AlreadyProven);
	});
}

#[test]
fn non_mandatory_rejected_when_no_messages() {
	new_test_ext().execute_with(|| {
		// LatestMessageBlock = 0, no messages dispatched
		setup_non_mandatory_proof(1000, 500, 5);
		assert_noop!(submit(test_account()), Error::<Test>::ProofNotNeeded);
	});
}

#[test]
fn non_mandatory_rejected_when_parachain_height_doesnt_cover_message() {
	new_test_ext().execute_with(|| {
		// Message at parachain block 2000
		LatestMessageBlock::<Test>::put(2000);

		// Proof covers parachain up to 1500, which is < last_message (2000)
		setup_non_mandatory_proof(3000, 1500, 5);
		assert_noop!(submit(test_account()), Error::<Test>::ProofNotNeeded);
	});
}

#[test]
fn non_mandatory_accepted_when_proof_covers_message() {
	new_test_ext().execute_with(|| {
		// Message at parachain block 1500
		LatestMessageBlock::<Test>::put(1500);

		// Proof covers parachain up to 2000 >= last_message (1500)
		setup_non_mandatory_proof(3000, 2000, 5);
		assert_ok!(submit(test_account()));

		let recent = RecentProofs::<Test>::get();
		assert_eq!(recent.len(), 1);
		assert_eq!(recent[0].finalized_height, 3000);
	});
}

#[test]
fn non_mandatory_accepted_when_proof_exactly_covers_message() {
	new_test_ext().execute_with(|| {
		LatestMessageBlock::<Test>::put(1000);

		// parachain_height == last_message
		setup_non_mandatory_proof(2000, 1000, 5);
		assert_ok!(submit(test_account()));
	});
}

#[test]
fn on_dispatch_updates_latest_message_block() {
	new_test_ext().execute_with(|| {
		use pallet_ismp::OnDispatch;

		assert_eq!(LatestMessageBlock::<Test>::get(), 0);

		frame_system::Pallet::<Test>::set_block_number(42);
		pallet_outbound_proofs::Pallet::<Test>::on_dispatch();
		assert_eq!(LatestMessageBlock::<Test>::get(), 42);

		frame_system::Pallet::<Test>::set_block_number(100);
		pallet_outbound_proofs::Pallet::<Test>::on_dispatch();
		assert_eq!(LatestMessageBlock::<Test>::get(), 100);
	});
}

#[test]
fn ring_buffer_evicts_oldest() {
	new_test_ext().execute_with(|| {
		// MaxStoredProofs = 3, insert 4 mandatory proofs
		for i in 1..=4u64 {
			setup_mandatory_proof(i as u32 * 1000, i as u32 * 500, i - 1, i);
			assert_ok!(submit(test_account()));
		}

		let recent = RecentProofs::<Test>::get();
		assert_eq!(recent.len(), 3);
		assert_eq!(recent[0].finalized_height, 2000);
		assert_eq!(recent[1].finalized_height, 3000);
		assert_eq!(recent[2].finalized_height, 4000);
	});
}

#[test]
fn first_prover_wins() {
	new_test_ext().execute_with(|| {
		setup_mandatory_proof(1000, 500, 0, 1);
		assert_ok!(submit(test_account()));

		let recent = RecentProofs::<Test>::get();
		assert_eq!(recent[0].prover, test_account());

		// Same relay height rejected
		setup_mandatory_proof(1000, 600, 1, 2);
		assert_noop!(submit(other_account()), Error::<Test>::AlreadyProven);
	});
}
