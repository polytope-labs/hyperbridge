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

mod error;
#[cfg(test)]
mod test;

use core::marker::PhantomData;

use crate::error::Error;
use beefy_verifier_primitives::{
	BeefyConsensusProof, ConsensusState, ParachainHeader, ParachainProof, RelaychainProof,
};
use codec::Encode;
use ismp::messaging::Keccak256;
use merkle_mountain_range::{
	Error as MmrError, Merge as MmrMerge, MerkleProof as MmrMerkleProof, leaf_index_to_mmr_size,
	leaf_index_to_pos,
};
use polkadot_sdk::sp_consensus_beefy::mmr::{BeefyAuthoritySet, MmrLeafVersion};
use primitive_types::H256;
use rs_merkle::{Hasher, MerkleProof};

/// The payload ID for the MMR root hash in a BEEFY commitment
const MMR_ROOT_PAYLOAD_ID: [u8; 2] = *b"mh";

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

/// Verify the consensus proof and return the new trusted consensus state and verified parachain
/// headers
pub fn verify_consensus<H: Keccak256 + EcdsaRecover + Send + Sync>(
	trusted_state: ConsensusState,
	proof: BeefyConsensusProof,
) -> anyhow::Result<(Vec<u8>, Vec<ParachainHeader>)> {
	let (state, heads_root) = verify_mmr_update_proof::<H>(trusted_state, proof.relay)?;
	let verified_headers = verify_parachain_headers::<H>(heads_root, proof.parachain)?;
	Ok((state.encode(), verified_headers))
}

/// Verifies a new Mmr root update, the relay chain accumulates it's blocks into a merkle mountain
/// range tree which light clients can use as a source for log_2(n) ancestry proofs. This new mmr
/// root hash is signed by the relay chain authority set and we can verify the membership of the
/// authorities that signed this new root using a merkle multi proof and a merkle commitment to the
/// total authorities
fn verify_mmr_update_proof<H: Keccak256 + EcdsaRecover + Send + Sync>(
	mut trusted_state: ConsensusState,
	relay_proof: RelaychainProof,
) -> Result<(ConsensusState, H256), Error> {
	let signatures_length = relay_proof.signed_commitment.signatures.len();
	let latest_height = relay_proof.signed_commitment.commitment.block_number;

	if trusted_state.latest_beefy_height >= latest_height {
		return Err(Error::StaleHeight {
			trusted_height: trusted_state.latest_beefy_height,
			current_height: latest_height,
		})
	}

	if !check_participation_threshold(
		signatures_length as u32,
		trusted_state.current_authorities.len,
	) && !check_participation_threshold(
		signatures_length as u32,
		trusted_state.next_authorities.len,
	) {
		return Err(Error::SuperMajorityRequired);
	}

	let commitment = relay_proof.signed_commitment.commitment.clone();

	if commitment.validator_set_id != trusted_state.current_authorities.id &&
		commitment.validator_set_id != trusted_state.next_authorities.id
	{
		return Err(Error::UnknownAuthoritySet { id: commitment.validator_set_id });
	}

	let is_current_authorities =
		commitment.validator_set_id == trusted_state.current_authorities.id;

	let mmr_root_data = commitment
		.payload
		.get_raw(&MMR_ROOT_PAYLOAD_ID)
		.ok_or(Error::MmrRootHashMissing)?;

	if mmr_root_data.len() != 32 {
		return Err(Error::InvalidMmrRootHashLength { len: mmr_root_data.len() });
	}
	let mmr_root = H256::from_slice(mmr_root_data);

	let commitment_hash = H::keccak256(&commitment.encode());
	let mut authority_leaves: Vec<[u8; 32]> = Vec::new();
	let mut authority_indices = Vec::new();

	for sig in relay_proof.signed_commitment.signatures.iter() {
		let uncompressed = H::secp256k1_recover(&commitment_hash.0, &sig.signature)
			.map_err(|_| Error::FailedToRecoverPublicKey)?;

		let hashed_uncompressed = H::keccak256(&uncompressed);

		let mut eth_address = [0u8; 20];
		eth_address.copy_from_slice(&hashed_uncompressed.as_ref()[12..]);

		let authority_address_hash = H::keccak256(&eth_address);

		authority_leaves.push(authority_address_hash.into());
		authority_indices.push(sig.index as usize);
	}

	let proof_hashes: Vec<[u8; 32]> = relay_proof.proof.iter().map(|h| (*h).into()).collect();
	let merkle_proof = MerkleProof::<MerkleHasher<H>>::new(proof_hashes);

	let valid = if is_current_authorities {
		merkle_proof.verify(
			trusted_state.current_authorities.keyset_commitment.into(),
			&authority_indices,
			&authority_leaves,
			trusted_state.current_authorities.len as usize,
		)
	} else {
		merkle_proof.verify(
			trusted_state.next_authorities.keyset_commitment.into(),
			&authority_indices,
			&authority_leaves,
			trusted_state.next_authorities.len as usize,
		)
	};

	if !valid {
		return Err(Error::InvalidAuthoritiesProof)
	}

	verify_mmr_leaf::<H>(&relay_proof, mmr_root)?;

	if relay_proof.latest_mmr_leaf.beefy_next_authority_set.id > trusted_state.next_authorities.id {
		trusted_state.current_authorities = trusted_state.next_authorities.clone();
		trusted_state.next_authorities =
			relay_proof.latest_mmr_leaf.beefy_next_authority_set.clone();
	}

	trusted_state.latest_beefy_height = latest_height;

	Ok((trusted_state, relay_proof.latest_mmr_leaf.extra))
}

/// Verifies the inclusion of parachain headers in the parachain heads root via a merkle multi proof
fn verify_parachain_headers<H: Keccak256>(
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
	let proof_hashes: Vec<[u8; 32]> =
		parachain_proof.proof.iter().map(|node| (*node).into()).collect();
	let merkle_proof = MerkleProof::<MerkleHasher<H>>::new(proof_hashes);
	let valid = merkle_proof.verify(
		heads_root.0,
		&leaf_indices,
		&leaf_hashes,
		parachain_proof.total_leaves as usize,
	);

	if !valid {
		return Err(Error::InvalidParachainProof);
	}

	Ok(parachain_proof.parachains)
}

fn verify_mmr_leaf<H: Keccak256 + Send + Sync>(
	relay: &RelaychainProof,
	mmr_root: H256,
) -> Result<(), Error> {
	#[derive(Encode)]
	struct CanonicalMmrLeaf {
		version: MmrLeafVersion,
		parent_number_and_hash: (u32, H256),
		beefy_next_authority_set: BeefyAuthoritySet<H256>,
		leaf_extra: H256,
	}

	let canonical_leaf = CanonicalMmrLeaf {
		version: relay.latest_mmr_leaf.version.clone(),
		parent_number_and_hash: relay.latest_mmr_leaf.parent_block_and_hash,
		beefy_next_authority_set: relay.latest_mmr_leaf.beefy_next_authority_set.clone(),
		leaf_extra: relay.latest_mmr_leaf.extra,
	};
	let leaf_hash = H::keccak256(&canonical_leaf.encode());
	let mmr_size = leaf_index_to_mmr_size(relay.latest_mmr_leaf.leaf_index as u64);

	let mmr_proof = MmrMerkleProof::<[u8; 32], KeccakMerge<H>>::new(
		mmr_size,
		relay.mmr_proof.iter().map(|h| (*h).into()).collect(),
	);
	let leaf_pos = leaf_index_to_pos(relay.latest_mmr_leaf.leaf_index as u64);
	let leaf = (leaf_pos, leaf_hash.into());
	let valid = mmr_proof
		.verify(mmr_root.into(), vec![leaf])
		.map_err(|e| Error::MmrVerificationFailed(e.to_string()))?;

	if !valid {
		return Err(Error::InvalidMmrProof)
	}

	Ok(())
}

/// Checks for supermajority participation
fn check_participation_threshold(len: u32, total: u32) -> bool {
	len >= ((2 * total) / 3) + 1
}
