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

//! BEEFY consensus proof verifier.
//!
//! Provides a chain-agnostic verifier for BEEFY finality proofs originating from a Polkadot-style
//! relay chain, as well as an [`sp1`] module that verifies SP1-compressed BEEFY proofs.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

#[cfg(feature = "apk")]
pub mod apk;
pub mod ecdsa;
pub mod error;
pub mod sp1;
#[cfg(test)]
mod test;

use alloc::{string::ToString, vec, vec::Vec};
use core::marker::PhantomData;

use crate::error::Error;
use beefy_verifier_primitives::{
	ConsensusMessage, ConsensusState, MmrProof, ParachainHeader, ParachainProof,
};
use codec::Encode;
use ismp::messaging::Keccak256;
use merkle_mountain_range::{
	Error as MmrError, Merge as MmrMerge, MerkleProof as MmrMerkleProof, leaf_index_to_mmr_size,
	leaf_index_to_pos,
};
use polkadot_sdk::{
	sp_consensus_beefy::{Commitment, mmr::MmrLeaf},
	sp_mmr_primitives::LeafProof,
};
use primitive_types::H256;
use rs_merkle::{Hasher, MerkleProof};

/// The payload ID for the MMR root hash in a BEEFY commitment
pub(crate) const MMR_ROOT_PAYLOAD_ID: [u8; 2] = *b"mh";

/// A trait for recovering secp256k1 public keys from ECDSA signatures.
/// This allows the verifier to be generic.
pub trait EcdsaRecover {
	/// Recover the uncompressed public key (64 bytes, without 0x04 prefix) from a 32-byte
	/// prehash and 65-byte signature. Signature format: [r (32) | s (32) | v (1)]
	fn secp256k1_recover(prehash: &[u8; 32], signature: &[u8; 65]) -> anyhow::Result<[u8; 64]>;
}

/// A hasher implementation for rs_merkle, generic over the hash function
pub struct MerkleHasher<H>(PhantomData<H>);

impl<H> Clone for MerkleHasher<H> {
	fn clone(&self) -> Self {
		Self(PhantomData)
	}
}

impl<H: Keccak256> Hasher for MerkleHasher<H> {
	type Hash = [u8; 32];

	fn hash(data: &[u8]) -> Self::Hash {
		H::keccak256(data).into()
	}
}

/// Merge strategy for the merkle mountain range crate, generic over the hash function
struct KeccakMerge<H>(PhantomData<H>);

impl<H: Keccak256> MmrMerge for KeccakMerge<H> {
	type Item = [u8; 32];

	fn merge(left: &Self::Item, right: &Self::Item) -> Result<Self::Item, MmrError> {
		let mut data = [0u8; 64];
		data[..32].copy_from_slice(left);
		data[32..].copy_from_slice(right);
		Ok(H::keccak256(&data).into())
	}
}

/// Verifies the inclusion of parachain headers in the parachain heads root via a merkle multi proof
pub fn verify_parachain_headers<H: Keccak256>(
	heads_root: H256,
	parachain_proof: ParachainProof,
) -> Result<Vec<ParachainHeader>, Error> {
	if parachain_proof.parachains.is_empty() {
		return Ok(vec![]);
	}

	let mut indexed_leaf_hashes = Vec::with_capacity(parachain_proof.parachains.len());

	for para_header in &parachain_proof.parachains {
		let leaf = (para_header.para_id, para_header.header.clone());
		let hash: [u8; 32] = H::keccak256(&leaf.encode()).into();
		indexed_leaf_hashes.push((para_header.index as usize, hash));
	}

	indexed_leaf_hashes.sort_by_key(|(index, _)| *index);

	let (leaf_indices, leaf_hashes): (Vec<usize>, Vec<[u8; 32]>) =
		indexed_leaf_hashes.into_iter().unzip();
	let merkle_proof = MerkleProof::<MerkleHasher<H>>::new(parachain_proof.proof.clone());
	let valid = merkle_proof.verify(
		heads_root.0,
		&leaf_indices,
		&leaf_hashes,
		parachain_proof.total_leaves as usize,
	);

	if !valid {
		Err(Error::InvalidParachainProof)?;
	}

	Ok(parachain_proof.parachains)
}

pub(crate) fn verify_mmr_leaf<H: Keccak256 + Send + Sync>(
	leaf: &MmrLeaf<u32, H256, H256, H256>,
	proof: &LeafProof<H256>,
	mmr_root: H256,
) -> Result<(), Error> {
	// `leaf_indices` is supplied by the relayer in the unsigned consensus message;
	// an empty vector previously panicked the runtime via the unchecked `[0]` index
	// after the BEEFY signature and authority membership checks had already succeeded.
	// This verifier checks a single MMR leaf, so reject any proof that does not carry
	// exactly one leaf index.
	if proof.leaf_indices.len() != 1 {
		Err(Error::InvalidMmrProof)?
	}
	let leaf_index = proof.leaf_indices[0];
	let leaf_hash = H::keccak256(&leaf.encode());
	let mmr_size = leaf_index_to_mmr_size(leaf_index);

	let mmr_proof = MmrMerkleProof::<[u8; 32], KeccakMerge<H>>::new(
		mmr_size,
		proof.items.iter().map(|h| (*h).into()).collect(),
	);
	let leaf_pos = leaf_index_to_pos(leaf_index);
	let leaf = (leaf_pos, leaf_hash.into());
	let valid = mmr_proof
		.verify(mmr_root.into(), vec![leaf])
		.map_err(|e| Error::MmrVerificationFailed(e.to_string()))?;

	if !valid {
		Err(Error::InvalidMmrProof)?
	}

	Ok(())
}
