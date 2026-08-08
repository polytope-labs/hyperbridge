// Copyright (C) 2022 Polytope Labs.
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

//! Primitive BEEFY types used by verifier and prover

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::all)]
#![deny(missing_docs)]

use codec::{Decode, Encode};
use polkadot_sdk::*;
use sp_consensus_beefy::mmr::{BeefyAuthoritySet, MmrLeaf, MmrLeafVersion};
use sp_core::H256;
use sp_std::prelude::*;

/// Client state definition for the light client
#[derive(sp_std::fmt::Debug, Encode, Decode, PartialEq, Eq, Clone)]
pub struct ConsensusState {
	/// Latest beefy height
	pub latest_beefy_height: u32,
	/// Height at which beefy was activated.
	pub beefy_activation_block: u32,
	/// Latest mmr root hash
	pub mmr_root_hash: H256,
	/// Authorities for the current session
	pub current_authorities: BeefyAuthoritySet<H256>,
	/// Authorities for the next session
	pub next_authorities: BeefyAuthoritySet<H256>,
}

/// Hash length definition for hashing algorithms used
pub const HASH_LENGTH: usize = 32;
/// Authority Signature type
pub type TSignature = [u8; 65];
/// Represents a Hash in this library
pub type Hash = [u8; 32];

#[derive(Clone, sp_std::fmt::Debug, PartialEq, Eq, Encode, Decode)]
/// Authority signature and its index in the signatures array
pub struct SignatureWithAuthorityIndex {
	/// Authority signature
	pub signature: TSignature,
	/// 0-based index of the authority in the authority set
	pub index: u32,
}

#[derive(Clone, sp_std::fmt::Debug, PartialEq, Eq, Encode, Decode)]
/// Signed commitment
pub struct SignedCommitment {
	/// Commitment
	pub commitment: sp_consensus_beefy::Commitment<u32>,
	/// Signatures for this commitment
	pub signatures: Vec<SignatureWithAuthorityIndex>,
}

#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// Mmr Update with proof
pub struct MmrProof {
	/// Signed commitment
	pub signed_commitment: SignedCommitment,
	/// Latest leaf added to mmr
	pub latest_mmr_leaf: MmrLeaf<u32, H256, H256, H256>,
	/// Proof for the latest mmr leaf
	pub mmr_proof: sp_mmr_primitives::LeafProof<H256>,
	/// Flat proof hashes for authorities merkle multi-proof.
	pub authority_proof: Vec<[u8; 32]>,
}

#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// A partial representation of the mmr leaf
pub struct PartialMmrLeaf {
	/// Leaf version
	pub version: MmrLeafVersion,
	/// Parent block number and hash
	pub parent_number_and_hash: (u32, H256),
	/// Next beefy authorities
	pub beefy_next_authority_set: BeefyAuthoritySet<H256>,
}

#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// Parachain header and metadata needed for merkle inclusion proof
pub struct ParachainHeader {
	/// scale encoded parachain header
	pub header: Vec<u8>,
	/// 0-based leaf index in the parachain heads merkle tree
	pub index: u32,
	/// ParaId for parachain
	pub para_id: u32,
}

#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// Parachain proofs definition
pub struct ParachainProof {
	/// List of parachains we have a proof for
	pub parachains: Vec<ParachainHeader>,
	/// Flat proof hashes for parachain header merkle multi-proof.
	pub proof: Vec<[u8; 32]>,
	/// Total leaves count for the proof
	pub total_leaves: u32,
}

#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// Parachain headers update with proof
pub struct ConsensusMessage {
	/// Parachain headers
	pub parachain: ParachainProof,
	/// proof for finalized mmr root
	pub mmr: MmrProof,
}

/// Proof type identifier for naive proofs
pub const PROOF_TYPE_NAIVE: u8 = 0x00;

/// Proof type identifier for SP1 ZK proofs
pub const PROOF_TYPE_SP1: u8 = 0x01;

/// Proof type identifier for aggregate BLS12-381 proofs
pub const PROOF_TYPE_BLS: u8 = 0x02;

/// Size of a compressed BLS12-381 G1 point, the group BEEFY signatures live in.
pub const BLS_G1_SIGNATURE_LEN: usize = 48;

/// Size of a compressed BLS12-381 G2 point, the group BEEFY public keys live in.
pub const BLS_G2_PUBLIC_KEY_LEN: usize = 96;

/// Size of an uncompressed BLS12-381 G2 point as EIP-2537 encodes it: four 64 byte field
/// elements, `x.c0 || x.c1 || y.c0 || y.c1`.
pub const BLS_G2_UNCOMPRESSED_LEN: usize = 256;

/// Size of an uncompressed BLS12-381 G1 point: `x || y`, each 64 bytes.
pub const BLS_G1_UNCOMPRESSED_LEN: usize = 128;

/// `(p - 1) / 2` for the BLS12-381 base field, big-endian.
const HALF_MODULUS: [u8; 48] = [
	0x0d, 0x00, 0x88, 0xf5, 0x1c, 0xbf, 0xf3, 0x4d, 0x25, 0x8d, 0xd3, 0xdb, 0x21, 0xa5, 0xd6, 0x6b,
	0xb2, 0x3b, 0xa5, 0xc2, 0x79, 0xc2, 0x89, 0x5f, 0xb3, 0x98, 0x69, 0x50, 0x7b, 0x58, 0x7b, 0x12,
	0x0f, 0x55, 0xff, 0xff, 0x58, 0xa9, 0xff, 0xff, 0xdc, 0xff, 0x7f, 0xff, 0xff, 0xff, 0xd5, 0x55,
];

/// Compress an uncompressed G2 point into the 96 byte form the relay chain commits to.
///
/// EIP-2537 only accepts uncompressed points, so an EVM-bound proof carries those, while the
/// keyset commitment and the Rust verifier both work on the compressed encoding. Converting this
/// direction is pure byte manipulation: take `x`, ordered `c1` then `c0`, and set the compression
/// flag plus a sign bit recording which square root `y` is. The reverse would need an Fp2 square
/// root, which is why proofs never travel compressed to the EVM.
pub fn compress_g2(point: &[u8; BLS_G2_UNCOMPRESSED_LEN]) -> [u8; BLS_G2_PUBLIC_KEY_LEN] {
	let mut out = [0u8; BLS_G2_PUBLIC_KEY_LEN];
	// Each 64 byte field element carries its 48 byte value in the trailing bytes.
	out[..48].copy_from_slice(&point[80..128]); // x.c1
	out[48..].copy_from_slice(&point[16..64]); // x.c0

	let y_c1 = &point[208..256];
	let sign = y_c1 > &HALF_MODULUS[..];

	out[0] |= 0x80; // compression flag
	if sign {
		out[0] |= 0x20;
	}

	out
}

/// Compress an uncompressed G1 point into the 48 byte form `w3f-bls` serialises.
///
/// Same convention as [`compress_g2`], with a single field element instead of a pair.
pub fn compress_g1(point: &[u8; BLS_G1_UNCOMPRESSED_LEN]) -> [u8; BLS_G1_SIGNATURE_LEN] {
	let mut out = [0u8; BLS_G1_SIGNATURE_LEN];
	out.copy_from_slice(&point[16..64]); // x

	let y = &point[80..128];
	let sign = y > &HALF_MODULUS[..];

	out[0] |= 0x80;
	if sign {
		out[0] |= 0x20;
	}

	out
}

/// A validator that contributed to an aggregate BLS signature.
///
/// Only the public key is carried. The individual signatures are summed by the prover into
/// [`BlsMmrProof::aggregate_signature`], since verification never needs them apart.
#[derive(Clone, sp_std::fmt::Debug, PartialEq, Eq, Encode, Decode)]
pub struct BlsSigner {
	/// Compressed G2 public key, as committed to by the relay chain's keyset commitment.
	pub public_key: [u8; BLS_G2_PUBLIC_KEY_LEN],
	/// 0-based index of the authority in the authority set
	pub index: u32,
}

/// An MMR root update proven by an aggregate BLS12-381 signature rather than by recovering each
/// authority's ECDSA signature individually.
///
/// The verifier checks this in one pairing operation regardless of how many validators signed,
/// which is the whole point of the BLS path. The tradeoff is that the signers' public keys travel
/// with the proof, so its size grows with the number of signers.
#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct BlsMmrProof {
	/// The commitment that was signed
	pub commitment: sp_consensus_beefy::Commitment<u32>,
	/// The validators that signed, with the public keys the aggregate was formed over
	pub signers: Vec<BlsSigner>,
	/// Sum of the signers' G1 signatures, as a compressed G1 point
	pub aggregate_signature: [u8; BLS_G1_SIGNATURE_LEN],
	/// Latest leaf added to mmr
	pub latest_mmr_leaf: MmrLeaf<u32, H256, H256, H256>,
	/// Proof for the latest mmr leaf
	pub mmr_proof: sp_mmr_primitives::LeafProof<H256>,
	/// Root of the tree over the authorities' BLS public keys. The relay chain commits this as one
	/// extra leaf of the authority set tree, so it is proven rather than trusted.
	pub bls_commitment: H256,
	/// Flat proof hashes proving [`Self::bls_commitment`] is the authority set's extra leaf,
	/// against the keyset commitment. That tree holds `len + 1` leaves: the authorities, then
	/// this one.
	pub keyset_proof: Vec<[u8; 32]>,
	/// Flat proof hashes proving the signers' public keys against [`Self::bls_commitment`]
	pub authority_proof: Vec<[u8; 32]>,
}

/// A BEEFY consensus update proven by an aggregate BLS signature.
#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct BlsConsensusMessage {
	/// Parachain headers
	pub parachain: ParachainProof,
	/// proof for finalized mmr root
	pub mmr: BlsMmrProof,
}

/// SP1 BEEFY proof. The proof bytes are prefixed with [`PROOF_TYPE_SP1`] by the prover.
#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Sp1BeefyProof {
	/// Relay chain block number
	pub block_number: u32,
	/// Validator set ID that signed the commitment
	pub validator_set_id: u64,
	/// Latest MMR leaf data
	pub mmr_leaf: MmrLeaf<u32, H256, H256, H256>,
	/// Parachain headers finalized by this proof. SP1 proves inclusion via public inputs.
	pub headers: Vec<ParachainHeader>,
	/// SP1 proof bytes
	pub proof: Vec<u8>,
	/// Prover-chosen nonce committed into the SP1 public values. The verifier reconstructs the
	/// committed public inputs with this value, so the proof is only valid for this exact nonce.
	/// Rewarding verifiers (`pallet-beefy-consensus-proofs`) bind it to the extrinsic submission
	/// account, making the proof non-transferable.
	pub nonce: H256,
}

/// finality proof
#[cfg(feature = "std")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct EncodedVersionedFinalityProof(pub sp_core::Bytes);
