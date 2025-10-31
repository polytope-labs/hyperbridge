use near_primitives_ismp::prover::{Client, ConsensusProof, ProverError, TrustedState};

/// Main function to prove a header update
///
/// This constructs a consensus proof that can be verified by the verifier.
/// It fetches the next light client block after the trusted state and includes
/// the necessary validator sets for verification.
///
/// # Arguments
///
/// * `rpc_client` - The RPC client to fetch data from
/// * `trusted_state` - The current trusted state of the light client
/// * `target_height` - Optional target height to prove (if None, proves next available block)
///
/// # Returns
///
/// - `Ok(ConsensusProof)`: The consensus proof for the update
/// - `Err(ProverError)`: If proof generation fails
pub async fn prove_header_update(
	rpc_client: &impl Client,
	trusted_state: &TrustedState,
	target_height: Option<u64>,
) -> Result<ConsensusProof, ProverError> {
	// Fetch the next light client block after the trusted state
	let light_client_block = rpc_client
		.next_light_client_block(trusted_state.last_block_hash)
		.await?
		.ok_or_else(|| {
			ProverError::NoLightClientBlock(format!(
				"No next light client block found after {:?}",
				trusted_state.last_block_hash
			))
		})?;

	// Verify we got the right block if target height is specified
	if let Some(target) = target_height {
		if light_client_block.inner_lite.height != target {
			return Err(ProverError::InvalidHeight(format!(
				"Expected block at height {}, got {}",
				target, light_client_block.inner_lite.height
			)));
		}
	}

	// Get the current epoch's validators (used for verification)
	let current_validators = trusted_state.current_block_producers.clone();

	// Check if this is an epoch boundary (next_bps is present)
	let next_validators = if let Some(ref next_bps) = light_client_block.next_bps {
		Some(next_bps.clone())
	} else {
		// Not an epoch boundary, use next epoch validators if available
		trusted_state.next_block_producers.clone()
	};

	// Construct the consensus proof
	let consensus_proof =
		ConsensusProof::new(light_client_block, current_validators, next_validators);

	// Validate the proof before returning
	consensus_proof.validate().map_err(|e| {
		ProverError::ProofConstructionError(format!("Proof validation failed: {}", e))
	})?;

	Ok(consensus_proof)
}

/// Prove a header for misbehaviour detection
///
/// This is similar to prove_header_update but can be used to fetch blocks
/// at specific heights for detecting misbehaviour (e.g., conflicting blocks at same height).
///
/// # Arguments
///
/// * `rpc_client` - The RPC client to fetch data from
/// * `trusted_state` - The current trusted state of the light client
/// * `target_height` - The height to fetch for misbehaviour detection
///
/// # Returns
///
/// - `Ok(ConsensusProof)`: The consensus proof at the target height
/// - `Err(ProverError)`: If proof generation fails
pub async fn prove_misbehaviour_header(
	rpc_client: &impl Client,
	trusted_state: &TrustedState,
	target_height: u64,
) -> Result<ConsensusProof, ProverError> {
	// For misbehaviour, we want to fetch a specific block
	// We use the same mechanism but enforce the target height
	prove_header_update(rpc_client, trusted_state, Some(target_height)).await
}

/// Create an initial trusted state from a known block hash
///
/// This is a helper function to bootstrap the light client with an initial trusted state.
///
/// # Arguments
///
/// * `rpc_client` - The RPC client to fetch data from
/// * `block_hash` - The hash of the trusted block to start from
///
/// # Returns
///
/// - `Ok(TrustedState)`: The initial trusted state
/// - `Err(ProverError)`: If fetching the initial state fails
pub async fn create_initial_trusted_state(
	rpc_client: &impl Client,
	block_hash: near_primitives::hash::CryptoHash,
) -> Result<TrustedState, ProverError> {
	// Fetch the light client block to get epoch info
	let light_client_block =
		rpc_client.next_light_client_block(block_hash).await?.ok_or_else(|| {
			ProverError::NoLightClientBlock(format!(
				"No light client block found for {:?}",
				block_hash
			))
		})?;

	let height = light_client_block.inner_lite.height;
	let epoch_id = light_client_block.inner_lite.epoch_id;
	let next_epoch_id = light_client_block.inner_lite.next_epoch_id;

	// Fetch validators for the current epoch
	let current_block_producers = rpc_client.validators(epoch_id).await?;

	// Get next epoch validators if available in the block
	let next_block_producers = light_client_block.next_bps;

	Ok(TrustedState::new(
		block_hash,
		height,
		epoch_id,
		next_epoch_id,
		current_block_producers,
		next_block_producers,
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	// Tests would go here - would require a mock client or actual RPC access
}
