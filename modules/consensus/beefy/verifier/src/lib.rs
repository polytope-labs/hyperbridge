mod types;

use polkadot_sdk::*;
use anyhow::anyhow;
use beefy_verifier_primitives::{BeefyConsensusProof, ConsensusState, Node, ParachainHeader, ParachainProof, PartialMmrLeaf, RelaychainProof};
use codec::{Decode, Encode};
use ismp::{consensus::IntermediateState, error::Error, messaging::Keccak256};
use primitive_types::H256;
use sp_core::{ByteArray, ecdsa, hashing};
use polkadot_sdk::sp_runtime::traits::Header;
use ismp::consensus::{StateCommitment, StateMachineHeight, StateMachineId};
use merkle_mountain_range::{MerkleProof as MmrMerkleProof, util::MemStore, Merge as MmrMerge, Error as MmrError};
use rs_merkle::{Hasher, MerkleProof};
use crate::types::MmrLeaf;
use sp_core::{hashing::keccak_256 as keccak256};

const MMR_ROOT_PAYLOAD_ID: [u8; 2] = *b"mh";

#[derive(Clone)]
struct MerkleKeccak256;

impl Hasher for MerkleKeccak256 {
	type Hash = [u8; 32];

	fn hash(data: &[u8]) -> Self::Hash {
		keccak256(data)
	}
}

struct KeccakMerge;

impl MmrMerge for KeccakMerge {
	type Item = [u8; 32];

	fn merge(left: &Self::Item, right: &Self::Item) -> Result<Self::Item, MmrError> {
		let mut data = [0u8; 64];
		data[..32].copy_from_slice(left);
		data[32..].copy_from_slice(right);
		Ok(keccak256(&data))
	}

	fn merge_peaks(right: &Self::Item, left: &Self::Item) -> Result<Self::Item, MmrError> {
		let mut data = [0u8; 64];
		data[..32].copy_from_slice(right);
		data[32..].copy_from_slice(left);
		Ok(keccak256(&data))
	}
}

pub fn verify_consensus<H: Keccak256 + Send + Sync>(
	encoded_state: Vec<u8>,
	encoded_proof: Vec<u8>,
) -> anyhow::Result<(Vec<u8>, Vec<ParachainHeader>)> {
	let consensus_state = ConsensusState::decode(&mut &encoded_state[..])
		.map_err(|_| anyhow!("Failed to decode Beefy ConsensusState"))?;
	let proof = BeefyConsensusProof::decode(&mut &encoded_proof[..])
		.map_err(|_| anyhow!("Failed to decode BeefyConsensusProof"))?;

	let (new_state, parachain_headers) = verify_consensus_logic::<H>(consensus_state, proof)?;
	Ok((new_state.encode(), parachain_headers))
}

fn verify_consensus_logic<H: Keccak256 + Send + Sync>(
	trusted_state: ConsensusState,
	proof: BeefyConsensusProof,
) -> anyhow::Result<(ConsensusState, Vec<ParachainHeader>)> {
	let (state, heads_root) = verify_mmr_update_proof::<H>(trusted_state, proof.relay)?;
	let parachain_headers = verify_parachain_header_proof::<H>(heads_root, proof.parachain)?;
	Ok((state, parachain_headers))
}

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
        let signature = ecdsa::Signature::from_slice(&sig.signature).map_err(|_| anyhow!("Invalid signature format"))?;
		let recovered_pubkey = signature.recover(&commitment_hash).ok_or_else(|| anyhow!("Failed to recover public key"))?;

		let authority_address_hash = H::keccak256(&recovered_pubkey.0);
		authority_leaves.push(<[u8; 32]>::from(authority_address_hash));
		authority_indices.push(sig.index as usize);
		authorities.push( Node { index: sig.index as u32, hash: H256::from(authority_address_hash)});
    }
	let proof_hashes: Vec<[u8; 32]> = relay_proof.proof.iter().flatten().map(|h| h.hash.into()).collect();
	let merkle_proof = MerkleProof::<MerkleKeccak256>::new(proof_hashes);

	let valid = if is_current_authorities {
		merkle_proof.verify(<[u8; 32]>::from(trusted_state.current_authorities.keyset_commitment), &authority_indices, &authority_leaves, trusted_state.current_authorities.len as usize)
	} else {
		merkle_proof.verify(<[u8; 32]>::from(trusted_state.next_authorities.keyset_commitment), &authority_indices, &authority_leaves, trusted_state.next_authorities.len as usize)
	};

	if !valid {
		Err(anyhow!("Invalid Authorities proof"))?;
	}

	verify_mmr_leaf::<H>(&trusted_state, &relay_proof, mmr_root)?;

	if relay_proof.latest_mmr_leaf.beefy_next_authority_set.id > trusted_state.next_authorities.id {
		trusted_state.current_authorities = trusted_state.next_authorities.clone();
		trusted_state.next_authorities = relay_proof.latest_mmr_leaf.beefy_next_authority_set.clone();
	}

	trusted_state.latest_beefy_height = latest_height;

	Ok((trusted_state, relay_proof.latest_mmr_leaf.extra))

}

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
	let leaf_count = leaf_index(trusted_state.beefy_activation_block, relay.latest_mmr_leaf.parent_block_and_hash.0) + 1;

	let mmr_proof = MmrMerkleProof::new(mmr_root.into(), relay.mmr_proof.iter().map(|h| h.into()).collect());
	let leaf = MmrLeaf { k_index: relay.latest_mmr_leaf.k_index, leaf_index: relay.latest_mmr_leaf.leaf_index, hash: leaf_hash };
	let leaf = merkle_mountain_range::MmrLeaf::new()


	/*let valid = merkle_mountain_range_verify(mmr_root, &relay.mmr_proof, &leaves, leaf_count as usize);
	if !valid {
		Err(anyhow!("Invalid Mmr proof"))?;
	}*/

	Ok(())
}

fn verify_parachain_header_proof<H: Keccak256 + Send + Sync> (
	heads_root: H256,
	proof: ParachainProof,
) -> anyhow::Result<Vec<ParachainHeader>> {
	if proof.parachains.is_empty() {
		return Ok(Vec::new());
	}

	let mut leaves = Vec::with_capacity(proof.parachains.len());
	for para_header in &proof.parachains {
		let mut para_id_encoded = (para_header.para_id as u32).encode();
		let mut header_encoded = para_header.header.clone();

		let mut final_bytes = Vec::new();
		final_bytes.append(&mut para_id_encoded);
		final_bytes.append(&mut header_encoded);

		leaves.push(Node {
			index: para_header.index,
			hash: H256::from(H::keccak256(&final_bytes)),
		});
	}

	let formatted_proof: Vec<Vec<Node>> = proof.proof.iter().map(|layer| {
		layer.iter().map(|(index, hash)| Node { index: *index, hash: H256::from(*hash)}).collect()
	}).collect();

	/*let valid = merkle_multi_proof_verify(&heads_root, &formatted_proof, &leaves);
	if !valid {
		Err(anyhow!("Invalid parachain header proof"));
	}*/

	Ok(proof.parachains)
}

fn leaf_index(activation_block: u32, parent_number: u32) -> u32 {
	if activation_block == 0 {
		return parent_number;
	}
	parent_number - activation_block
}

fn check_participation_threshold(len: u32, total: u32) -> bool {
	len >= ((2 * total) / 3) + 1
}
