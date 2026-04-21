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

use codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_core::sr25519;

/// Payload submitted via the `submit_proof` unsigned extrinsic.
///
/// The signed message is `keccak256(("beefy_consensus_proof_v1", submitter,
/// keccak256(proof)).encode())`; the signature in the outer extrinsic is expected to
/// verify against `submitter` interpreted as an SR25519 public key.
///
/// No nonce: replay is prevented by on-chain state progression. Once a proof is applied
/// `LastProvenHeight` / the BEEFY authority set id advance, and `verify_and_apply` then
/// rejects any resubmission of the same bytes with `StaleProof` or
/// `UnexpectedAuthoritySet`.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct SubmitProofPayload<AccountId> {
	/// The account that signed this payload and that will receive the reward (if any).
	pub submitter: AccountId,
	/// `bytes1 proof_type || abi-encoded proof body`, matching the wire format consumed by
	/// `ConsensusRouter.verify` on the EVM side.
	pub proof: alloc::vec::Vec<u8>,
}

/// Domain separator for the signed message.
pub const SIGNATURE_DOMAIN: &[u8] = b"pallet_beefy_consensus_proofs";

/// Offchain-storage prefix for raw verified proof bytes written by `submit_proof`.
/// Combined with the `proven_height` (`u64`, big-endian) to form the actual offchain key.
/// All proofs — rotation and messaging alike — share this single namespace since both
/// advance parachain height monotonically.
pub const OFFCHAIN_PREFIX: &[u8] = b"beefy_consensus_proofs::";

/// Proof type byte: naive BEEFY proof.
pub const PROOF_TYPE_NAIVE: u8 = 0x00;
/// Proof type byte: SP1 ZK BEEFY proof.
pub const PROOF_TYPE_SP1: u8 = 0x01;

/// `provides` tag for BEEFY consensus proofs — a single fixed slot. At most one proof
/// is retained in the pool at a time; higher `proven_height` wins. Unified across
/// rotation and messaging proofs so that the pool never holds a rotation alongside a
/// messaging proof that would supersede it on inclusion.
pub const PROOF_TAG: &[u8] = b"beefy_consensus_proof";

/// Signature type expected alongside [`SubmitProofPayload`].
pub type Signature = sr25519::Signature;

/// Offchain-storage key for a verified consensus proof keyed by `proven_height`.
/// Relayers reconstruct this key off of a [`MessagingProofs`](crate::pallet::MessagingProofs)
/// or [`RotationProofs`](crate::pallet::RotationProofs) entry and read the raw
/// ABI-encoded proof bytes from node-local offchain storage. A single namespace
/// covers both rotation and messaging proofs since both advance parachain height
/// monotonically.
pub fn offchain_key(proven_height: u64) -> alloc::vec::Vec<u8> {
	let mut key = alloc::vec::Vec::with_capacity(OFFCHAIN_PREFIX.len() + 8);
	key.extend_from_slice(OFFCHAIN_PREFIX);
	key.extend_from_slice(&proven_height.to_be_bytes());
	key
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
