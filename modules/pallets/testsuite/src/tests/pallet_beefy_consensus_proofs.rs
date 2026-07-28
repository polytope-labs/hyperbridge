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

//! Settlement tests for `pallet-beefy-consensus-proofs`.
//!
//! These drive `settle_first_proof` with a synthetic `VerifyOutcome` rather than a real BEEFY
//! proof. Minting a proof that rotates the authority set without advancing the parachain head
//! isn't something a test can do against live data, so the outcome is constructed directly.
//! That's the whole point: the collision these guard against needs a mandatory proof landing
//! on a height a messaging proof already used, which on a live chain only happens when the
//! parachain stalls across a session boundary.

use crate::runtime::{new_test_ext, Test};
use pallet_beefy_consensus_proofs::{
	pallet::VerifyOutcome,
	types::{messaging_offchain_key, rotation_offchain_key, PROOF_TYPE_NAIVE, PROOF_TYPE_SP1},
	AcceptedProvers, MessagingProofs, ProverCount, RotationProofs,
};
use polkadot_sdk::*;
use primitive_types::H256;
use sp_core::offchain::StorageKind;
use sp_runtime::AccountId32;

fn submitter(seed: u8) -> AccountId32 {
	AccountId32::new([seed; 32])
}

fn messaging_outcome(height: u64, set_id: u64) -> VerifyOutcome {
	VerifyOutcome {
		latest_height: height,
		current_set_id: set_id,
		rotated: false,
		has_new_messages: true,
		child_trie_root: H256::repeat_byte(height as u8),
	}
}

/// A mandatory proof that finalizes the same head an earlier messaging proof already covered,
/// which is exactly what #1067 made possible.
fn rotation_outcome(height: u64, set_id: u64) -> VerifyOutcome {
	VerifyOutcome {
		latest_height: height,
		current_set_id: set_id,
		rotated: true,
		has_new_messages: false,
		child_trie_root: H256::repeat_byte(height as u8),
	}
}

fn read_offchain(key: &[u8]) -> Option<Vec<u8>> {
	sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, key)
}

/// The regression this PR exists to prevent: a mandatory proof landing on a height a messaging
/// proof already wrote must not replace it. Under the old shared namespace the second write
/// silently clobbered the first.
#[test]
fn rotation_at_a_used_height_does_not_overwrite_the_messaging_blob() {
	let mut ext = new_test_ext();
	let height = 500u64;
	let messaging_blob = vec![PROOF_TYPE_NAIVE, 0xaa, 0xaa];
	let rotation_blob = vec![PROOF_TYPE_NAIVE, 0xbb, 0xbb];

	ext.execute_with(|| {
		pallet_beefy_consensus_proofs::Pallet::<Test>::settle_first_proof(
			submitter(1),
			messaging_blob.clone(),
			None,
			PROOF_TYPE_NAIVE,
			Vec::new(),
			messaging_outcome(height, 7),
		)
		.expect("messaging proof settles");

		pallet_beefy_consensus_proofs::Pallet::<Test>::settle_first_proof(
			submitter(2),
			rotation_blob.clone(),
			None,
			PROOF_TYPE_NAIVE,
			Vec::new(),
			rotation_outcome(height, 8),
		)
		.expect("rotation at the same height settles");
	});
	ext.persist_offchain_overlay();

	ext.execute_with(|| {
		assert_eq!(
			read_offchain(&messaging_offchain_key(height)),
			Some(messaging_blob),
			"the messaging blob must survive a rotation landing on its height",
		);
		assert_eq!(
			read_offchain(&rotation_offchain_key(8)),
			Some(rotation_blob),
			"the rotation blob must be readable under its set id",
		);
	});
}

/// A rotation writes nothing to the messaging namespace even though it advances the proven
/// height, which is the hole `OffchainProofSource::fetch` resolves through `RotationProofs`.
/// Pinned here so the index a reader needs for that lookup stays populated.
#[test]
fn a_rotation_is_only_discoverable_through_the_rotation_index() {
	let mut ext = new_test_ext();
	let height = 900u64;

	ext.execute_with(|| {
		pallet_beefy_consensus_proofs::Pallet::<Test>::settle_first_proof(
			submitter(1),
			vec![PROOF_TYPE_NAIVE, 0xcc],
			None,
			PROOF_TYPE_NAIVE,
			Vec::new(),
			rotation_outcome(height, 12),
		)
		.expect("rotation settles");

		assert!(
			!MessagingProofs::<Test>::get().contains(&height),
			"a rotation must not be recorded as a messaging proof",
		);
		assert_eq!(
			RotationProofs::<Test>::get().get(&12).copied(),
			Some(height),
			"the rotation index must map the set id to the height a reader would hold",
		);
	});
	ext.persist_offchain_overlay();

	ext.execute_with(|| {
		assert!(
			read_offchain(&messaging_offchain_key(height)).is_none(),
			"nothing is written to the messaging namespace for a rotation",
		);
		assert!(read_offchain(&rotation_offchain_key(12)).is_some());
	});
}

/// A rotation must still settle when the height it lands on has already handed out every uncle
/// slot. `record_uncle_metadata` fails there, and treating that as a dispatch error would roll
/// back the authority-set rotation, pinning consensus on the old set forever.
#[test]
fn a_full_uncle_slate_cannot_block_a_rotation() {
	let mut ext = new_test_ext();
	let height = 700u64;

	ext.execute_with(|| {
		// Fill every slot: one first prover plus MaxUncleProvers.
		let full = (0..=5u8).map(|i| H256::repeat_byte(i)).collect::<Vec<_>>();
		AcceptedProvers::<Test>::insert(
			height,
			sp_runtime::BoundedVec::truncate_from(full.clone()),
		);
		ProverCount::<Test>::insert(height, full.len() as u32);

		pallet_beefy_consensus_proofs::Pallet::<Test>::settle_first_proof(
			submitter(9),
			vec![PROOF_TYPE_SP1, 0xdd],
			Some(H256::repeat_byte(9)),
			PROOF_TYPE_SP1,
			Vec::new(),
			rotation_outcome(height, 20),
		)
		.expect("a full uncle slate must not reject the rotation");

		assert_eq!(
			RotationProofs::<Test>::get().get(&20).copied(),
			Some(height),
			"the rotation must be recorded despite the uncle bookkeeping being full",
		);
	});
}

/// Eviction clears the namespace the departing entry actually lived in. Previously an expiring
/// rotation computed a height key, which could delete a live messaging blob.
#[test]
fn eviction_clears_only_the_departing_namespace() {
	let mut ext = new_test_ext();
	// MaxStoredProofs is 4 in the test runtime, so the fifth rotation evicts the first.
	let heights = [10u64, 20, 30, 40, 50];

	ext.execute_with(|| {
		pallet_beefy_consensus_proofs::Pallet::<Test>::settle_first_proof(
			submitter(1),
			vec![PROOF_TYPE_NAIVE, 0xee],
			None,
			PROOF_TYPE_NAIVE,
			Vec::new(),
			messaging_outcome(heights[0], 1),
		)
		.expect("messaging proof settles");

		for (i, height) in heights.iter().enumerate() {
			pallet_beefy_consensus_proofs::Pallet::<Test>::settle_first_proof(
				submitter(2),
				vec![PROOF_TYPE_NAIVE, *height as u8],
				None,
				PROOF_TYPE_NAIVE,
				Vec::new(),
				rotation_outcome(*height, 100 + i as u64),
			)
			.expect("rotation settles");
		}
	});
	ext.persist_offchain_overlay();

	ext.execute_with(|| {
		assert!(
			read_offchain(&rotation_offchain_key(100)).is_none(),
			"the oldest rotation should have been evicted",
		);
		assert!(
			read_offchain(&rotation_offchain_key(104)).is_some(),
			"the newest rotation should still be present",
		);
		assert!(
			read_offchain(&messaging_offchain_key(heights[0])).is_some(),
			"evicting a rotation must not delete the messaging blob at the same height",
		);
	});
}
