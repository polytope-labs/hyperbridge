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

#[derive(sp_std::fmt::Debug, Encode, Decode, PartialEq, Eq, Clone)]
/// Client state definition for the light client
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
	/// Index in signatures vector
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

#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq)]
/// Mmr Update with proof
pub struct MmrProof {
	/// Signed commitment
	pub signed_commitment: SignedCommitment,
	/// Latest leaf added to mmr
	pub latest_mmr_leaf: MmrLeaf<u32, H256, H256, H256>,
	/// Proof for the latest mmr leaf
	pub mmr_proof: sp_mmr_primitives::LeafProof<H256>,
	/// Proof for authorities in current session
	pub authority_proof: Vec<Vec<(usize, [u8; 32])>>,
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
	/// leaf index for parachain heads proof
	pub index: u32,
	/// ParaId for parachain
	pub para_id: u32,
}

#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// Parachain proofs definition
pub struct ParachainProof {
	/// List of parachains we have a proof for
	pub parachains: Vec<ParachainHeader>,

	/// Proof for parachain header inclusion in the parachain headers root
	pub proof: Vec<[u8; 32]>,
	/// Total leaves count for the proof
	pub total_leaves: u32,
}

#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq)]
/// Parachain headers update with proof
pub struct ConsensusMessage {
	/// Parachain headers
	pub parachain: ParachainProof,
	/// proof for finalized mmr root
	pub mmr: MmrProof,
}

/// Represents a node in a Merkle proof, containing a hash and its index at a specific layer.
#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Node {
	/// The positional index of the node in its layer of the Merkle tree.
	pub index: u32,
	/// The hash of the node.
	pub hash: H256,
}

/// Represents a canonical BEEFY Merkle Mountain Range (MMR) leaf.
///
/// This struct contains the essential data about a finalized block that is committed to the MMR.
#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct BeefyMmrLeaf {
	/// The version of the MMR leaf format.
	pub version: MmrLeafVersion,
	/// A tuple containing the block number and hash of the parent block.
	pub parent_block_and_hash: (u32, H256),
	/// The authority set that will be active in the next BEEFY session.
	pub beefy_next_authority_set: BeefyAuthoritySet<H256>,
	/// The k-index of the leaf, used in MMR calculations.
	pub k_index: u32,
	/// The sequential index of this leaf in the MMR.
	pub leaf_index: u32,
	/// An extra data field
	pub extra: H256,
}

/// Represents the proof components for verifying the relay chain's consensus state
#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct RelaychainProof {
	/// Signed commitment
	pub signed_commitment: SignedCommitment,
	/// Latest leaf added to mmr
	pub latest_mmr_leaf: BeefyMmrLeaf,
	/// Proof for the latest mmr leaf
	pub mmr_proof: Vec<H256>,
	/// Proof for authorities in current/next session
	pub proof: Vec<H256>,
}

/// Represents a complete BEEFY consensus proof.
///
/// This proof contains all the necessary data to verify a BEEFY finality proof from the relay chain
/// and to prove the inclusion of specific parachain headers within that finalized block.
#[derive(sp_std::fmt::Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct BeefyConsensusProof {
	/// The proof items for the relay chain consensus
	pub relay: RelaychainProof,
	/// The proof items for parachain headers
	pub parachain: ParachainProof,
}

#[cfg(feature = "std")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
/// finality proof
pub struct EncodedVersionedFinalityProof(pub sp_core::Bytes);
