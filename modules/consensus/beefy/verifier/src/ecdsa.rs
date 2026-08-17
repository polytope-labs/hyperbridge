// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	 http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Verifying BEEFY finality from per signer ecdsa signatures.
//!
//! Each signer travels with its own signature and a merkle path proving membership of the
//! authority set, so both the proof and the work grow with the number of signers. The aggregate
//! route in [`crate::apk`] is the same protocol with that cost removed.

use alloc::{string::ToString, vec, vec::Vec};

use crate::{
	EcdsaRecover, MMR_ROOT_PAYLOAD_ID, MerkleHasher, error::Error, verify_mmr_leaf,
	verify_parachain_headers,
};
use beefy_verifier_primitives::{ConsensusMessage, ConsensusState, MmrProof, ParachainHeader};
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
use rs_merkle::MerkleProof;

/// Verify the consensus proof and return the new trusted consensus state and verified parachain
/// headers
pub fn verify_consensus<H: Keccak256 + EcdsaRecover + Send + Sync>(
	trusted_state: ConsensusState,
	proof: ConsensusMessage,
) -> Result<(Vec<u8>, Vec<ParachainHeader>), Error> {
	let (state, heads_root) = verify_mmr_update_proof::<H>(trusted_state, proof.mmr)?;
	let verified_headers = verify_parachain_headers::<H>(heads_root, proof.parachain)?;
	Ok((state.encode(), verified_headers))
}

/// Verifies a new Mmr root update, the relay chain accumulates it's blocks into a merkle mountain
/// range tree which light clients can use as a source for log_2(n) ancestry proofs. This new mmr
/// root hash is signed by the relay chain authority set and we can verify the membership of the
/// authorities that signed this new root using a merkle multi proof and a merkle commitment to the
/// total authorities
pub fn verify_mmr_update_proof<H: Keccak256 + EcdsaRecover + Send + Sync>(
	trusted_state: ConsensusState,
	mmr: MmrProof,
) -> Result<(ConsensusState, H256), Error> {
	let commitment = &mmr.signed_commitment.commitment;
	let preamble =
		prepare_update(&trusted_state, commitment, mmr.signed_commitment.signatures.len() as u32)?;

	// The ECDSA half signs the keccak hash of the commitment, and the authority set is committed
	// to as the keccak of each signer's Ethereum address, so the signers are identified by
	// recovering them rather than by being named in the proof.
	let commitment_hash = H::keccak256(&commitment.encode());
	let mut authority_leaves: Vec<[u8; 32]> = Vec::new();
	let mut authority_indices = Vec::new();

	for sig in mmr.signed_commitment.signatures.iter() {
		let uncompressed = H::secp256k1_recover(&commitment_hash.0, &sig.signature)
			.map_err(|_| Error::FailedToRecoverPublicKey)?;

		let hashed_uncompressed = H::keccak256(&uncompressed);

		let mut eth_address = [0u8; 20];
		eth_address.copy_from_slice(&hashed_uncompressed.as_ref()[12..]);

		let authority_address_hash = H::keccak256(&eth_address);

		authority_leaves.push(authority_address_hash.into());
		authority_indices.push(sig.index as usize);
	}

	verify_authority_membership::<H>(
		preamble.keyset_commitment,
		&mmr.authority_proof,
		&authority_indices,
		&authority_leaves,
		preamble.authority_count,
	)?;

	verify_mmr_leaf::<H>(&mmr.latest_mmr_leaf, &mmr.mmr_proof, preamble.mmr_root)?;

	let latest_height = commitment.block_number;
	let state = apply_update(trusted_state, &mmr.latest_mmr_leaf, latest_height);

	Ok((state, mmr.latest_mmr_leaf.leaf_extra))
}

/// The parts of an update that hold regardless of how the commitment was signed.
struct UpdatePreamble {
	/// Commitment to the authority set the signers must belong to.
	keyset_commitment: H256,
	/// Size of that authority set.
	authority_count: u32,
	/// MMR root carried in the commitment payload.
	mmr_root: H256,
}

/// Checks staleness, resolves which authority set the commitment claims to be signed under,
/// judges participation against that set alone, and extracts the MMR root from the payload.
fn prepare_update(
	trusted_state: &ConsensusState,
	commitment: &Commitment<u32>,
	signer_count: u32,
) -> Result<UpdatePreamble, Error> {
	if trusted_state.latest_beefy_height >= commitment.block_number {
		return Err(Error::StaleHeight {
			trusted_height: trusted_state.latest_beefy_height,
			current_height: commitment.block_number,
		});
	}

	let authority_set = if commitment.validator_set_id == trusted_state.current_authorities.id {
		&trusted_state.current_authorities
	} else if commitment.validator_set_id == trusted_state.next_authorities.id {
		&trusted_state.next_authorities
	} else {
		return Err(Error::UnknownAuthoritySet { id: commitment.validator_set_id });
	};

	if !check_participation_threshold(signer_count, authority_set.len) {
		return Err(Error::SuperMajorityRequired);
	}

	let mmr_root_data = commitment
		.payload
		.get_raw(&MMR_ROOT_PAYLOAD_ID)
		.ok_or(Error::MmrRootHashMissing)?;

	if mmr_root_data.len() != 32 {
		return Err(Error::InvalidMmrRootHashLength { len: mmr_root_data.len() });
	}

	Ok(UpdatePreamble {
		keyset_commitment: authority_set.keyset_commitment,
		authority_count: authority_set.len,
		mmr_root: H256::from_slice(mmr_root_data),
	})
}

/// Proves the signing authorities are members of the committed authority set.
fn verify_authority_membership<H: Keccak256>(
	keyset_commitment: H256,
	proof: &[[u8; 32]],
	indices: &[usize],
	leaves: &[[u8; 32]],
	authority_count: u32,
) -> Result<(), Error> {
	let merkle_proof = MerkleProof::<MerkleHasher<H>>::new(proof.to_vec());

	let valid =
		merkle_proof.verify(keyset_commitment.into(), indices, leaves, authority_count as usize);

	if !valid {
		Err(Error::InvalidAuthoritiesProof)?;
	}

	Ok(())
}

/// Rotates the tracked authority sets if the leaf announces a newer one, and records the height.
fn apply_update(
	mut trusted_state: ConsensusState,
	leaf: &MmrLeaf<u32, H256, H256, H256>,
	latest_height: u32,
) -> ConsensusState {
	if leaf.beefy_next_authority_set.id > trusted_state.next_authorities.id {
		trusted_state.current_authorities = trusted_state.next_authorities.clone();
		trusted_state.next_authorities = leaf.beefy_next_authority_set.clone();
	}

	trusted_state.latest_beefy_height = latest_height;

	trusted_state
}

/// Checks for supermajority participation
fn check_participation_threshold(len: u32, total: u32) -> bool {
	len >= ((2 * total) / 3) + 1
}
