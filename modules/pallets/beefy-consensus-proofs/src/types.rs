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

//! Types for `pallet-beefy-consensus-proofs`.

use alloc::vec::Vec;
use codec::{Decode, Encode};

/// Offchain-storage prefix for messaging proof bytes, combined with the proven parachain
/// height.
pub const MESSAGING_OFFCHAIN_PREFIX: &[u8] = b"beefy_consensus_proofs::";

/// Offchain-storage prefix for mandatory proof bytes, combined with the authority set id
/// the proof rotated to.
///
/// Mandatory proofs need their own namespace because parachain height doesn't identify them
/// uniquely. A rotation proof carries whatever head the relay chain held at the session
/// boundary, which can be a head an earlier messaging proof already finalized, so under one
/// shared namespace the two blobs land on the same key and the later write wins.
pub const ROTATION_OFFCHAIN_PREFIX: &[u8] = b"beefy_consensus_proofs::rotation::";

/// Proof type byte: naive BEEFY proof.
pub const PROOF_TYPE_NAIVE: u8 = 0x00;
/// Proof type byte: SP1 ZK BEEFY proof.
pub const PROOF_TYPE_SP1: u8 = 0x01;
/// Proof type byte: aggregate public key BEEFY proof.
pub const PROOF_TYPE_APK: u8 = 0x02;

fn offchain_key(prefix: &[u8], id: u64) -> Vec<u8> {
	let mut key = Vec::with_capacity(prefix.len() + 8);
	key.extend_from_slice(prefix);
	key.extend_from_slice(&id.to_be_bytes());
	key
}

/// Offchain key for a messaging proof, keyed by the parachain height it advanced to.
/// Relayers rebuild this off a [`MessagingProofs`](crate::pallet::MessagingProofs) entry.
/// Messaging proofs must advance the proven height, so the height is unique here.
pub fn messaging_offchain_key(proven_height: u64) -> Vec<u8> {
	offchain_key(MESSAGING_OFFCHAIN_PREFIX, proven_height)
}

/// Offchain key for a mandatory proof, keyed by the authority set id it rotated to. This is
/// the same key [`RotationProofs`](crate::pallet::RotationProofs) is indexed by, so a relayer
/// walking that map across epochs builds the key from the map key rather than the height.
pub fn rotation_offchain_key(set_id: u64) -> Vec<u8> {
	offchain_key(ROTATION_OFFCHAIN_PREFIX, set_id)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// No height and set id may ever produce the same key, whatever the two ids are. Compare
	/// the built keys across a spread of pairs rather than asserting anything about the
	/// prefixes, so that making the two namespaces identical fails this outright.
	#[test]
	fn messaging_and_rotation_keys_cannot_alias() {
		let ids = [0u64, 1, 2, 512, 9_812_059, u64::MAX];
		for height in ids {
			for set_id in ids {
				assert_ne!(
					messaging_offchain_key(height),
					rotation_offchain_key(set_id),
					"messaging height {height} aliased rotation set {set_id}",
				);
			}
		}
	}

	/// The SDK rebuilds both keys from hardcoded strings and big-endian ids, so pin the
	/// encoding here to catch a change on this side that isn't mirrored there.
	#[test]
	fn keys_match_the_sdk_encoding() {
		assert_eq!(
			messaging_offchain_key(1),
			[b"beefy_consensus_proofs::".as_slice(), &1u64.to_be_bytes()].concat(),
		);
		assert_eq!(
			rotation_offchain_key(7),
			[b"beefy_consensus_proofs::rotation::".as_slice(), &7u64.to_be_bytes()].concat(),
		);
	}
}

/// Whether an apk state is still missing the commitment for the set it will rotate into.
///
/// A proof that fills this in is doing work even if it finalizes nothing new, since the client
/// cannot accept the rotation until it knows the incoming set's keys.
pub fn next_commitment_unknown(state: &[u8], proof_type: u8) -> bool {
	proof_type == PROOF_TYPE_APK &&
		beefy_verifier_primitives::ApkConsensusState::decode(&mut &state[..])
			.map(|state| state.next_authorities.apk_commitment.is_zero())
			.unwrap_or(false)
}

#[cfg(test)]
mod commitment_tests {
	use super::*;
	use beefy_verifier_primitives::{ApkAuthoritySet, ApkConsensusState};
	use primitive_types::H256;

	fn state(next: H256) -> Vec<u8> {
		ApkConsensusState {
			latest_beefy_height: 100,
			beefy_activation_block: 0,
			mmr_root_hash: H256::zero(),
			current_authorities: ApkAuthoritySet {
				id: 7,
				len: 2,
				apk_commitment: H256::repeat_byte(1),
			},
			next_authorities: ApkAuthoritySet { id: 8, len: 2, apk_commitment: next },
		}
		.encode()
	}

	#[test]
	fn an_empty_next_commitment_is_the_only_thing_worth_learning() {
		assert!(next_commitment_unknown(&state(H256::zero()), PROOF_TYPE_APK));
		assert!(!next_commitment_unknown(&state(H256::repeat_byte(2)), PROOF_TYPE_APK));
	}

	#[test]
	fn other_proof_types_never_learn_a_commitment() {
		assert!(!next_commitment_unknown(&state(H256::zero()), PROOF_TYPE_NAIVE));
		assert!(!next_commitment_unknown(&state(H256::zero()), PROOF_TYPE_SP1));
	}

	#[test]
	fn a_state_that_is_not_the_apk_shape_is_not_missing_anything() {
		assert!(!next_commitment_unknown(&[0u8; 3], PROOF_TYPE_APK));
	}
}

/// BEEFY host-function backed crypto used by `beefy-verifier`.
pub struct SubstrateCrypto;

impl ismp::messaging::Keccak256 for SubstrateCrypto {
	fn keccak256(bytes: &[u8]) -> primitive_types::H256 {
		sp_io::hashing::keccak_256(bytes).into()
	}
}

impl beefy_verifier::EcdsaRecover for SubstrateCrypto {
	fn secp256k1_recover(prehash: &[u8; 32], signature: &[u8; 65]) -> anyhow::Result<[u8; 64]> {
		sp_io::crypto::secp256k1_ecdsa_recover(signature, prehash)
			.map_err(|_| anyhow::anyhow!("Failed to recover secp256k1 public key"))
	}
}
