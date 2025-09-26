use anyhow::anyhow;
use beefy_verifier_primitives::{BeefyConsensusProof, ConsensusState, Node, ParachainProof, RelaychainProof};
use codec::{Decode, Encode};
use ismp::{consensus::IntermediateState, error::Error, messaging::Keccak256};
use primitive_types::H256;
use polkadot_sdk::sp_core::{ByteArray, ecdsa};

const MMR_ROOT_PAYLOAD_ID: [u8; 2] = *b"mh";

pub fn verify_consensus<H: Keccak256 + Send + Sync>(
	encoded_state: Vec<u8>,
	encoded_proof: Vec<u8>,
) -> anyhow::Result<(Vec<u8>, Vec<IntermediateState>)> {
	let consensus_state = ConsensusState::decode(&mut &encoded_state[..])
		.map_err(|_| anyhow!("Failed to decode Beefy ConsensusState"))?;
	let proof = BeefyConsensusProof::decode(&mut &encoded_proof[..])
		.map_err(|_| anyhow!("Failed to decode BeefyConsensusProof"))?;

	let (new_state, intermediate) = verify_consensus_logic::<H>(consensus_state, proof)?;
	Ok((new_state.encode(), vec![intermediate]))
}

fn verify_consensus_logic<H: Keccak256 + Send + Sync>(
	trusted_state: ConsensusState,
	proof: BeefyConsensusProof,
) -> anyhow::Result<(ConsensusState, IntermediateState)> {
	let (state, heads_root) = verify_mmr_update_proof::<H>(trusted_state, proof.relay)?;
	let intermediate = verify_parachain_header_proof(heads_root, proof.parachain)?;
	Ok((state, intermediate))
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
		.ok_or_else(|| anyhow!("MmrRootHashMissing"))?;

	if mmr_root_data.len() != 32 {
		return Err(anyhow!("Invalid Mmr root hash lenght"));
	}
	let mmr_root = H256::from_slice(mmr_root_data);

    let commitment_hash = H::keccak256(&commitment.encode());
    let mut authorities = Vec::with_capacity(signatures_length);

    for sig in &relay_proof.signed_commitment.signatures {
        let signature = ecdsa::Signature::from_slice(&sig.signature).map_err(|_| anyhow!("Invalid signature format"))?;
		let recovered_pubkey = signature.recover(&commitment_hash).ok_or_else(|| anyhow!("Failed to recover public key"))?;

		let authority_address_hash = H::keccak256(&recovered_pubkey.0);
		authorities.push( Node { index: sig.index as u32, hash: H256::from(authority_address_hash)});
    }

	let valid = if is_current_authorities {
		//todo:
		true
	} else {
		false
	};

	if !valid {
		Err(anyhow!("Invalid Authorities proof"))?;
	}

	verify_mmr_leaf(&trusted_state, &relay_proof, mmr_root)?;

	if relay_proof.latest_mmr_leaf.beefy_next_authority_set.id > trusted_state.next_authorities.id {
		trusted_state.current_authorities = trusted_state.next_authorities.clone();
		trusted_state.next_authorities = relay_proof.latest_mmr_leaf.beefy_next_authority_set.clone();
	}

	trusted_state.latest_beefy_height = latest_height;

	Ok((trusted_state, relay_proof.latest_mmr_leaf.extra))

}

fn verify_mmr_leaf(
	trusted_state: &ConsensusState,
	relay: &RelaychainProof,
	mmr_root: H256,
) -> anyhow::Result<()> {
	todo!()
}

fn verify_parachain_header_proof(
	heads_root: H256,
	proof: ParachainProof,
) -> anyhow::Result<IntermediateState> {
	todo!()
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
