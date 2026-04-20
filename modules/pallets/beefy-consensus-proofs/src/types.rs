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
/// The signed message is `blake2_256(("beefy_consensus_proof_v1", submitter,
/// blake2_256(proof)).encode())`; the signature in the outer extrinsic is expected to
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
/// Combined with a stream discriminator and a `u64` (set_id or proven_height) to form
/// the actual offchain key via blake2_128.
pub const OFFCHAIN_PREFIX: &[u8] = b"beefy_consensus_proofs::";

/// Offchain-storage discriminator for rotation proofs.
pub const OFFCHAIN_ROT: &[u8] = b"rot";

/// Offchain-storage discriminator for messaging proofs.
pub const OFFCHAIN_MSG: &[u8] = b"msg";

/// Proof type byte: naive BEEFY proof.
pub const PROOF_TYPE_NAIVE: u8 = 0x00;
/// Proof type byte: SP1 ZK BEEFY proof.
pub const PROOF_TYPE_SP1: u8 = 0x01;

/// `provides` tag for messaging proofs (fixed — at most one in the pool).
pub const MSG_TAG: &[u8] = b"beefy_message_proof";
/// `provides` tag prefix for rotation proofs (`(prefix, next_set_id).encode()`).
pub const ROT_TAG: &[u8] = b"beefy_rotation_proof";

/// Signature type expected alongside [`SubmitProofPayload`].
pub type Signature = sr25519::Signature;
