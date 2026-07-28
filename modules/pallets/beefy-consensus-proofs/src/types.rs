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

	/// The rotation prefix extends the messaging one, so what actually keeps a rotation blob
	/// off a messaging key is the length, not the leading bytes. Offchain lookups are exact
	/// key, so no height and set id can ever alias.
	#[test]
	fn messaging_and_rotation_keys_cannot_alias() {
		assert!(ROTATION_OFFCHAIN_PREFIX.starts_with(MESSAGING_OFFCHAIN_PREFIX));
		assert_eq!(messaging_offchain_key(u64::MAX).len(), MESSAGING_OFFCHAIN_PREFIX.len() + 8);
		assert_eq!(rotation_offchain_key(u64::MAX).len(), ROTATION_OFFCHAIN_PREFIX.len() + 8);
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
