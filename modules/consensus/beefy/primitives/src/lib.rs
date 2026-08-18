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
	pub current_authorities: AuthoritySet,
	/// Authorities for the next session
	pub next_authorities: AuthoritySet,
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

/// Proof type identifier for aggregate public key proofs
pub const PROOF_TYPE_APK: u8 = 0x02;

/// Size of a compressed BLS12-381 G1 point, the group BEEFY signatures live in.
pub const BLS_G1_SIGNATURE_LEN: usize = 48;

/// Size of a compressed BLS12-381 G2 point, the group BEEFY public keys live in.
pub const BLS_G2_PUBLIC_KEY_LEN: usize = 96;

/// Wire size of a paired `ecdsa_bls_crypto` BEEFY key: `ecdsa(33) || G1(48) || G2(96)`.
pub const PAIRED_AUTHORITY_LEN: usize = 33 + BLS_G1_SIGNATURE_LEN + BLS_G2_PUBLIC_KEY_LEN;

/// Where the BLS halves start, after the ECDSA key.
const PAIRED_G1_OFFSET: usize = 33;
const PAIRED_G2_OFFSET: usize = PAIRED_G1_OFFSET + BLS_G1_SIGNATURE_LEN;

/// A BEEFY authority key on a relay chain that uses paired `ecdsa_bls_crypto`, exactly as the
/// relay stores it.
///
/// `DoublePublicKey` publishes the same secret in both BLS groups, so a validator has a G1 and a
/// G2 half describing one key. BEEFY verifies signatures against the G2 half while an APK proof
/// aggregates the G1 halves, which is why both accessors exist.
#[derive(Clone, sp_std::fmt::Debug, PartialEq, Eq, Encode, Decode)]
pub struct PairedAuthority(pub [u8; PAIRED_AUTHORITY_LEN]);

impl PairedAuthority {
	/// The compressed G1 half.
	pub fn g1(&self) -> [u8; BLS_G1_SIGNATURE_LEN] {
		let mut out = [0u8; BLS_G1_SIGNATURE_LEN];
		out.copy_from_slice(&self.0[PAIRED_G1_OFFSET..PAIRED_G2_OFFSET]);
		out
	}

	/// The compressed G2 half.
	pub fn g2(&self) -> [u8; BLS_G2_PUBLIC_KEY_LEN] {
		let mut out = [0u8; BLS_G2_PUBLIC_KEY_LEN];
		out.copy_from_slice(&self.0[PAIRED_G2_OFFSET..PAIRED_AUTHORITY_LEN]);
		out
	}
}

/// Size of a curve point as the APK circuit's verifier takes it: raw big-endian coordinates, 48
/// bytes each, with no padding. Not the EIP-2537 layout.
pub const APK_G1_LEN: usize = 96;
/// The same for G2, four coordinates.
pub const APK_G2_LEN: usize = 192;
/// The participation bitlist, one bit per validator slot.
pub const APK_BITLIST_WORDS: usize = 5;

/// `Beefy::NextAuthorities` on a relay chain.
///
/// The *next* set, not the current one. A client verifying under set N learns the commitment for
/// set N+1, which is what lets it verify the update after a rotation. Note this is not
/// `well_known_keys::NEXT_AUTHORITIES`, which is Babe's: both end in `twox_128("NextAuthorities")`
/// and only the pallet prefix differs.
pub const RELAY_BEEFY_NEXT_AUTHORITIES: [u8; 32] = [
	0x08, 0xc4, 0x19, 0x74, 0xa9, 0x7d, 0xbf, 0x15, 0xcf, 0xbe, 0xc2, 0x83, 0x65, 0xbe, 0xa2, 0xda,
	0xaa, 0xcf, 0x00, 0xb9, 0xb4, 0x1f, 0xda, 0x7a, 0x92, 0x68, 0x82, 0x1c, 0x2a, 0x2b, 0x3e, 0x4c,
];

/// Engine id of the digest item carrying an authority set's APK commitment.
pub const APK_ENGINE_ID: [u8; 4] = *b"APKC";

/// The payload hyperbridge writes into a header digest once it has hashed an authority set.
///
/// This is a wire format shared by the runtime that writes it and every verifier that reads it. A
/// client that has already authenticated a hyperbridge header, through the parachain heads root in
/// the BEEFY MMR leaf, can take the commitment straight out of that header with no further proof.
#[derive(Clone, sp_std::fmt::Debug, PartialEq, Eq, Encode, Decode)]
pub struct ApkCommitmentDigest {
	/// The BEEFY validator set the keys belong to, which is the relay's current set id plus one,
	/// since the commitment describes the next set.
	pub set_id: u64,
	/// How many validators are in that set, which is what the threshold is taken against.
	///
	/// Carried here rather than read from the mmr leaf so everything a client believes about the
	/// incoming set comes from one authenticated place. Taking the size from the leaf and the
	/// commitment from here would leave the two able to describe different sets.
	pub len: u32,
	/// Poseidon2 over the set's G1 keys, padded to the circuit width with the identity point.
	pub commitment: [u8; 32],
}

impl ApkCommitmentDigest {
	/// Pull the commitment out of a header's digest logs, if one is there.
	///
	/// Returns the first matching item, since at most one is ever written per block. A malformed
	/// payload reads as absent rather than as an error, on the grounds that some other producer
	/// wrote under this engine id and the honest answer is that there is no commitment here.
	pub fn find_in(digest: &sp_runtime::generic::Digest) -> Option<Self> {
		digest.logs().iter().find_map(|log| match log {
			sp_runtime::DigestItem::Consensus(id, payload) if *id == APK_ENGINE_ID =>
				Self::decode(&mut &payload[..]).ok(),
			_ => None,
		})
	}
}
/// A validator set as a client keeps it, holding what every proof format needs to check a
/// signature from that set.
///
/// One state serves all of them, so a client that advances the set fills in both roots rather than
/// only the one it uses. Leaving the other empty would strand whichever client relies on it until
/// something supplies it again. This is not the mmr leaf's authority set, whose shape belongs to
/// the relay chain.
#[derive(Clone, sp_std::fmt::Debug, PartialEq, Eq, Encode, Decode, Default)]
pub struct AuthoritySet {
	/// Id of the set
	pub id: u64,
	/// Number of validators in the set, which the threshold is taken against
	pub len: u32,
	/// Poseidon2 over the set's G1 keys, zero until a header digest supplies it
	pub bls_poseidon_hash: H256,
	/// Merkle root over the set's ecdsa keys, as the mmr leaf names it
	pub ecdsa_merkle_root: H256,
}

impl ConsensusState {
	/// What this state becomes once a verified header names an authority set, if anything.
	///
	/// A header naming the set after the one being waited on rolls the sets forward: the relay has
	/// moved on, so the current set is the one that was next. A header naming a set already held
	/// fills in its commitment if it has none, which is what a client waiting on the next set is
	/// looking for. Anything else, including a set already carrying one, returns `None`, so the
	/// same digest arriving in several headers changes nothing.
	pub fn with_bls_commitment(
		&self,
		set_id: u64,
		len: u32,
		commitment: H256,
		ecdsa_merkle_root: H256,
	) -> Option<(AuthoritySet, AuthoritySet)> {
		let incoming =
			AuthoritySet { id: set_id, len, bls_poseidon_hash: commitment, ecdsa_merkle_root };
		if set_id > self.next_authorities.id {
			return Some((self.next_authorities.clone(), incoming));
		}
		if set_id == self.next_authorities.id && self.next_authorities.bls_poseidon_hash.is_zero() {
			let mut next = self.next_authorities.clone();
			next.len = len;
			next.bls_poseidon_hash = commitment;
			return Some((self.current_authorities.clone(), next));
		}
		if set_id == self.current_authorities.id &&
			self.current_authorities.bls_poseidon_hash.is_zero()
		{
			let mut current = self.current_authorities.clone();
			current.len = len;
			current.bls_poseidon_hash = commitment;
			return Some((current, self.next_authorities.clone()));
		}
		None
	}
}

/// The relay chain half of an update proven by an aggregate public key proof.
///
/// Nothing here grows with the number of signers. The bitlist is fixed width, the aggregates are
/// one point each, and the proof is constant size.
#[derive(Clone, sp_std::fmt::Debug, PartialEq, Eq, Encode, Decode)]
pub struct ApkMmrProof {
	/// The commitment that was signed
	pub commitment: sp_consensus_beefy::Commitment<u32>,
	/// Which validators signed, one bit each, big-endian words
	pub bitlist: [[u8; 32]; APK_BITLIST_WORDS],
	/// Aggregate of the signers' G1 keys, proven correct by [`Self::apk_proof`]
	pub apk: [u8; APK_G1_LEN],
	/// The same aggregate in G2, bound to `apk` by the pairing check
	pub apk2: [u8; APK_G2_LEN],
	/// PLONK proof that `apk` aggregates exactly the validators named in `bitlist`
	pub apk_proof: Vec<u8>,
	/// Sum of the signers' signatures, a G1 point
	pub signature: [u8; APK_G1_LEN],
	/// Latest leaf added to mmr
	pub latest_mmr_leaf: MmrLeaf<u32, H256, H256, H256>,
	/// Proof for the latest mmr leaf
	pub mmr_proof: sp_mmr_primitives::LeafProof<H256>,
}

/// A BEEFY consensus update proven by an aggregate public key proof.
#[derive(Clone, sp_std::fmt::Debug, PartialEq, Eq, Encode, Decode)]
pub struct ApkConsensusMessage {
	/// Parachain headers
	pub parachain: ParachainProof,
	/// Proof for the finalized mmr root
	pub mmr: ApkMmrProof,
}

/// The relay chain half of a BLS BEEFY update: the signed commitment, the aggregate signature, and
/// the MMR leaf it attests to.
///
/// Which validators signed, and the proof that their aggregate key is the authority set's, are not
/// here. Those come from the APK proof, which is built and checked outside this crate.
#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct BlsMmrProof {
	/// The commitment that was signed
	pub commitment: sp_consensus_beefy::Commitment<u32>,
	/// Sum of the signers' G1 signatures, as a compressed G1 point
	pub aggregate_signature: [u8; BLS_G1_SIGNATURE_LEN],
	/// Latest leaf added to mmr
	pub latest_mmr_leaf: MmrLeaf<u32, H256, H256, H256>,
	/// Proof for the latest mmr leaf
	pub mmr_proof: sp_mmr_primitives::LeafProof<H256>,
}

/// A BEEFY consensus update signed with aggregate BLS12-381.
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
