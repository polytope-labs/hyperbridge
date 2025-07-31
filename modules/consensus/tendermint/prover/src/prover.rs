use tendermint_primitives::{Client, ConsensusProof, ProverError, TrustedState};

/// Main function to prove a header update
/// This constructs a consensus proof that can be verified by the verifier
pub async fn prove_header_update(
	rpc_client: &impl Client,
	trusted_state: &TrustedState,
	target_height: u64,
) -> Result<ConsensusProof, ProverError> {
	let target_signed_header = rpc_client.signed_header(target_height).await?;

	// Fetch the ancestry (all headers from trusted height + 1 to target height - 1)
	let ancestry = if target_height > trusted_state.height + 1 {
		rpc_client
			.signed_headers_range(trusted_state.height + 1, target_height - 1)
			.await?
	} else {
		Vec::new()
	};

	let next_validators = if target_signed_header.header.next_validators_hash.is_empty() {
		None
	} else {
		Some(rpc_client.next_validators(target_height).await?)
	};

	let consensus_proof = ConsensusProof::new(target_signed_header, ancestry, next_validators);

	Ok(consensus_proof)
}

/// Prove a header for misbehaviour detection
pub async fn prove_misbehaviour_header(
	rpc_client: &impl Client,
	trusted_state: &TrustedState,
	target_height: u64,
) -> Result<ConsensusProof, ProverError> {
	let target_signed_header = rpc_client.signed_header(target_height).await?;

	// Fetch ancestry if needed
	let ancestry = if target_height > trusted_state.height {
		rpc_client
			.signed_headers_range(trusted_state.height + 1, target_height - 1)
			.await?
	} else {
		Vec::new()
	};

	let next_validators = if target_signed_header.header.next_validators_hash.is_empty() {
		None
	} else {
		Some(rpc_client.next_validators(target_height).await?)
	};

	let consensus_proof = ConsensusProof::new(target_signed_header, ancestry, next_validators);

	Ok(consensus_proof)
}
