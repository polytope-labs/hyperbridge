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

use anyhow::anyhow;
use beefy_verifier_primitives::{
	BeefyConsensusProof, ConsensusState, Node, PartialMmrLeaf, RelaychainProof,
};
use codec::Encode;
use ismp::messaging::Keccak256;
use merkle_mountain_range::{Error as MmrError, Merge as MmrMerge, MerkleProof as MmrMerkleProof};
use polkadot_sdk::{sp_runtime::traits::Header, *};
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

/// Verifies a new Mmr root update, the relay chain accumulates it's blocks into a merkle mountain range tree
/// which light clients can use as a source for log_2(n) ancestry proofs. This new mmr root hash is signed by the
/// relay chain authority set and we can verify the membership of the authorities that signed this new root
/// using a merkle multi proof and a merkle commitment to the total authorities
fn verify_mmr_update_proof<H: Keccak256 + Send + Sync>(
	mut trusted_state: ConsensusState,
	relay_proof: RelaychainProof,
) -> anyhow::Result<(ConsensusState, H256)> {
	let signatures_length = relay_proof.signed_commitment.signatures.len();
	let latest_height = relay_proof.signed_commitment.commitment.block_number;

	if trusted_state.latest_beefy_height >= latest_height {
		Err(anyhow!("Stale height"))?
	}

	if !check_participation_threshold(
		signatures_length as u32,
		trusted_state.current_authorities.len,
	) && !check_participation_threshold(
		signatures_length as u32,
		trusted_state.next_authorities.len,
	) {
		return Err(anyhow!("Super Majority Required"));
	}

	let commitment = relay_proof.signed_commitment.commitment.clone();

	if commitment.validator_set_id != trusted_state.current_authorities.id &&
		commitment.validator_set_id != trusted_state.next_authorities.id
	{
		return Err(anyhow!("Unknown Authority set"));
	}

	let is_current_authorities =
		commitment.validator_set_id == trusted_state.current_authorities.id;

	let mmr_root_data = commitment
		.payload
		.get_raw(&MMR_ROOT_PAYLOAD_ID)
		.ok_or_else(|| anyhow!("Mmr Root Hash Missing"))?;

	if mmr_root_data.len() != 32 {
		return Err(anyhow!("Invalid Mmr root hash lenght"));
	}
	let mmr_root = H256::from_slice(mmr_root_data);

	let commitment_hash = H::keccak256(&commitment.encode());
	let mut authorities = Vec::with_capacity(signatures_length);
	let mut authority_leaves: Vec<[u8; 32]> = Vec::new();
	let mut authority_indices = Vec::new();

	for sig in &relay_proof.signed_commitment.signatures {
		let signature = ecdsa::Signature::from_slice(&sig.signature)
			.map_err(|_| anyhow!("Invalid signature format"))?;
		let recovered_pubkey = signature
			.recover(&commitment_hash)
			.ok_or_else(|| anyhow!("Failed to recover public key"))?;

		let authority_address_hash = H::keccak256(&recovered_pubkey.0);
		authority_leaves.push(<[u8; 32]>::from(authority_address_hash));
		authority_indices.push(sig.index as usize);
		authorities
			.push(Node { index: sig.index, hash: H256::from(authority_address_hash) });
	}
	let proof_hashes: Vec<[u8; 32]> =
		relay_proof.proof.iter().flatten().map(|h| h.hash.into()).collect();
	let merkle_proof = MerkleProof::<MerkleKeccak256>::new(proof_hashes);

	let valid = if is_current_authorities {
		merkle_proof.verify(
			<[u8; 32]>::from(trusted_state.current_authorities.keyset_commitment),
			&authority_indices,
			&authority_leaves,
			trusted_state.current_authorities.len as usize,
		)
	} else {
		merkle_proof.verify(
			<[u8; 32]>::from(trusted_state.next_authorities.keyset_commitment),
			&authority_indices,
			&authority_leaves,
			trusted_state.next_authorities.len as usize,
		)
	};

	if !valid {
		Err(anyhow!("Invalid Authorities proof"))?;
	}

	verify_mmr_leaf::<H>(&trusted_state, &relay_proof, mmr_root)?;

	if relay_proof.latest_mmr_leaf.beefy_next_authority_set.id > trusted_state.next_authorities.id {
		trusted_state.current_authorities = trusted_state.next_authorities.clone();
		trusted_state.next_authorities =
			relay_proof.latest_mmr_leaf.beefy_next_authority_set.clone();
	}

	trusted_state.latest_beefy_height = latest_height;

	Ok((trusted_state, relay_proof.latest_mmr_leaf.extra))
}

/// Verifies the mmr leaf with a given mmr root
fn verify_mmr_leaf<H: Keccak256 + Send + Sync>(
	trusted_state: &ConsensusState,
	relay: &RelaychainProof,
	mmr_root: H256,
) -> anyhow::Result<()> {
	let partial_leaf = PartialMmrLeaf {
		version: relay.latest_mmr_leaf.version.clone(),
		parent_number_and_hash: relay.latest_mmr_leaf.parent_block_and_hash,
		beefy_next_authority_set: relay.latest_mmr_leaf.beefy_next_authority_set.clone(),
	};
	let leaf_hash = H::keccak256(&partial_leaf.encode());
	let leaf_count = leaf_index(
		trusted_state.beefy_activation_block,
		relay.latest_mmr_leaf.parent_block_and_hash.0,
	) + 1;

	let mmr_proof = MmrMerkleProof::<[u8; 32], KeccakMerge>::new(
		leaf_count as u64,
		relay.mmr_proof.iter().map(|h| (*h).into()).collect(),
	);
	let leaf = (relay.latest_mmr_leaf.leaf_index as u64, leaf_hash.into());
	let valid = mmr_proof
		.verify(mmr_root.into(), vec![leaf])
		.map_err(|e| anyhow!("MMR verification failed during calculation: {:?}", e.to_string()))?;

	if !valid {
		Err(anyhow!("Invalid Mmr proof: calculated root does not match provided root"))?;
	}

	Ok(())
}

/// Calculates the mmr leaf index for a block whose parent number is given
fn leaf_index(activation_block: u32, parent_number: u32) -> u32 {
	if activation_block == 0 {
		return parent_number;
	}
	parent_number - activation_block
}

/// Checks for supermajority participation
fn check_participation_threshold(len: u32, total: u32) -> bool {
	len >= ((2 * total) / 3) + 1
}
