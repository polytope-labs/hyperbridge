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
mod test;

use crate::error::Error;
use anyhow::anyhow;
use beefy_verifier_primitives::{
	BeefyConsensusProof, ConsensusState, Node, PartialMmrLeaf, RelaychainProof,
};
use codec::Encode;
use ismp::messaging::Keccak256;
use k256::{
	PublicKey,
	ecdsa::{RecoveryId, Signature as K256Signature, VerifyingKey},
	elliptic_curve::sec1::ToEncodedPoint,
};
use merkle_mountain_range::{
	Error as MmrError, Merge as MmrMerge, MerkleProof as MmrMerkleProof, leaf_index_to_mmr_size,
	leaf_index_to_pos,
};
use polkadot_sdk::{
	sp_consensus_beefy::mmr::{BeefyAuthoritySet, MmrLeafVersion},
	sp_runtime::traits::Header,
	*,
};
use primitive_types::H256;
use rs_merkle::{Hasher, MerkleProof};
use sp_core::{ByteArray, ecdsa, hashing::keccak_256 as keccak256};

/// The payload ID for the MMR root hash in a BEEFY commitment
const MMR_ROOT_PAYLOAD_ID: [u8; 2] = *b"mh";

/// A hasher implementation for rs_merkle which uses Keccak256 hashing
#[derive(Clone)]
pub struct MerkleKeccak256;

impl Hasher for MerkleKeccak256 {
	type Hash = [u8; 32];

	fn hash(data: &[u8]) -> Self::Hash {
		keccak256(data)
	}
}

/// Merge strategy for the merkle mountain range crate that uses Keccak256
struct KeccakMerge;

impl MmrMerge for KeccakMerge {
	type Item = [u8; 32];

	fn merge(left: &Self::Item, right: &Self::Item) -> Result<Self::Item, MmrError> {
		let mut data = [0u8; 64];
		data[..32].copy_from_slice(left);
		data[32..].copy_from_slice(right);
		Ok(keccak256(&data))
	}
}

/// Verify the consensus proof and return the new trusted consensys state and parachain head root
/// by this consensus proof
pub fn verify_consensus<H: Keccak256 + Send + Sync>(
	trusted_state: ConsensusState,
	proof: BeefyConsensusProof,
) -> anyhow::Result<(Vec<u8>, H256)> {
	let (state, heads_root) = verify_mmr_update_proof::<H>(trusted_state, proof.relay)?;
	Ok((state.encode(), heads_root))
}

/// Verifies a new Mmr root update, the relay chain accumulates it's blocks into a merkle mountain
/// range tree which light clients can use as a source for log_2(n) ancestry proofs. This new mmr
/// root hash is signed by the relay chain authority set and we can verify the membership of the
/// authorities that signed this new root using a merkle multi proof and a merkle commitment to the
/// total authorities
fn verify_mmr_update_proof<H: Keccak256 + Send + Sync>(
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

	println!("\n======= VERIFIER DEBUG =======");
	let target_root = if is_current_authorities {
		trusted_state.current_authorities.keyset_commitment
	} else {
		trusted_state.next_authorities.keyset_commitment
	};
	println!("Target Merkle Root: {:?}", H256::from(target_root));
	println!("Verifier-side calculated authority leaf hashes:");

	for (i, sig) in relay_proof.signed_commitment.signatures.iter().enumerate() {
		let recovery_id =
			RecoveryId::from_byte(sig.signature[64]).ok_or(Error::InvalidRecoveryId)?;
		let k256_sig = K256Signature::from_slice(&sig.signature[0..64])
			.map_err(|_| Error::InvalidSignatureFormat)?;

		let recovered_verifying_key =
			VerifyingKey::recover_from_prehash(commitment_hash.as_ref(), &k256_sig, recovery_id)
				.map_err(|_| Error::FailedToRecoverPublicKey)?;

		let uncompressed_point = recovered_verifying_key.to_encoded_point(false);
		let uncompressed_bytes_64 = &uncompressed_point.as_bytes()[1..];

		let hashed_uncompressed = H::keccak256(uncompressed_bytes_64);

		let mut eth_address = [0u8; 20];
		eth_address.copy_from_slice(&hashed_uncompressed.as_ref()[12..]);

		let authority_address_hash = H::keccak256(&eth_address);

		authority_leaves.push(authority_address_hash.into());
		authority_indices.push(sig.index as usize);
	}

	let proof_hashes: Vec<[u8; 32]> = relay_proof.proof.iter().map(|h| (*h).into()).collect();
	let merkle_proof = MerkleProof::<MerkleKeccak256>::new(proof_hashes);

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

	let mmr_proof = MmrMerkleProof::<[u8; 32], KeccakMerge>::new(
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
