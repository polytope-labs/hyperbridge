#[cfg(test)]
mod tests {
	use std::time::{SystemTime, UNIX_EPOCH};

	use crate::{prove_header_update, Client, CometBFTClient, HeimdallClient};
	use tendermint_verifier::{TrustedState, VerificationError, VerificationOptions};
	use tracing::trace;

	const STANDARD_RPC_URL: &str = "https://rpc.osmotest5.osmosis.zone:443";
	const POLYGON_RPC_URL: &str = "https://polygon-amoy-heimdall-rpc.publicnode.com:443";

	#[tokio::test]
	async fn test_standard_tendermint_integration() {
		trace!("Testing Standard Tendermint");
		run_integration_test_standard(STANDARD_RPC_URL).await.unwrap();
	}

	#[tokio::test]
	async fn test_polygon_heimdall_basic_rpc() {
		trace!("Testing Polygon's Heimdall Fork (Basic RPC)");
		test_polygon_basic_rpc(POLYGON_RPC_URL).await.unwrap();
	}

	// Fails with Invalid Proof, debugging
	#[tokio::test]
	async fn test_polygon_heimdall_full_verification() {
		trace!("Testing Polygon's Heimdall Fork (Full Verification)");
		run_integration_test_heimdall(POLYGON_RPC_URL).await.unwrap();
	}

	/// Full integration test: prover and verifier for standard CometBFT
	async fn run_integration_test_standard(
		rpc_url: &str,
	) -> Result<(), Box<dyn std::error::Error>> {
		let client = CometBFTClient::new(rpc_url).await?;
		ensure_healthy(&client).await?;
		let chain_id = client.chain_id().await?;
		let latest_height = client.latest_height().await?;
		let trusted_height = latest_height.saturating_sub(10);
		let trusted_header = client.signed_header(trusted_height).await?;
		let trusted_validators = client.validators(trusted_height).await?;
		let trusted_next_validators = client.next_validators(trusted_height).await?;
		let trusted_state = TrustedState::new(
			chain_id,
			trusted_height,
			trusted_header.header.time.unix_timestamp() as u64,
			trusted_header.header.hash().as_bytes().try_into().unwrap(),
			trusted_validators,
			trusted_next_validators,
			trusted_header.header.next_validators_hash.as_bytes().try_into().unwrap(),
			7200, // 2 hour trusting period
			VerificationOptions::default(),
		);
		trusted_state.validate()?;
		let target_height = latest_height.saturating_sub(5);
		trace!("Consensus proof generation started");
		let consensus_proof = prove_header_update(&client, &trusted_state, target_height).await?;
		trace!("Consensus proof generated");
		consensus_proof.validate()?;
		let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
		trace!("Verification started");
		match tendermint_verifier::verify_header_update(
			trusted_state,
			consensus_proof,
			VerificationOptions::default(),
			current_time,
		) {
			Ok(updated_state) => {
				trace!(
					"Verification successful! Updated trusted state height: {}",
					updated_state.trusted_state.height
				);
			},
			Err(VerificationError::NotEnoughTrust(tally)) => {
				return Err(format!("Not enough trust: {}", tally).into());
			},
			Err(VerificationError::Invalid(detail)) => {
				return Err(format!("Invalid proof: {}", detail).into());
			},
			Err(e) => {
				return Err(format!("Verification failed: {:?}", e).into());
			},
		}
		Ok(())
	}

	/// Full integration test: prover and verifier for Heimdall
	async fn run_integration_test_heimdall(
		rpc_url: &str,
	) -> Result<(), Box<dyn std::error::Error>> {
		let client = HeimdallClient::new(rpc_url);
		ensure_healthy(&client).await?;
		let chain_id = client.chain_id().await?;
		let latest_height = client.latest_height().await?;
		let trusted_height = latest_height.saturating_sub(10);
		let trusted_header = client.signed_header(trusted_height).await?;
		let trusted_validators = client.validators(trusted_height).await?;
		let trusted_next_validators = client.next_validators(trusted_height).await?;
		let trusted_state = TrustedState::new(
			chain_id,
			trusted_height,
			trusted_header.header.time.unix_timestamp() as u64,
			trusted_header.header.hash().as_bytes().try_into().unwrap(),
			trusted_validators,
			trusted_next_validators,
			trusted_header.header.next_validators_hash.as_bytes().try_into().unwrap(),
			3600, // 1 hour trusting period
			VerificationOptions::default(),
		);
		trusted_state.validate()?;
		let target_height = latest_height.saturating_sub(5);
		trace!("Consensus proof generation started");
		let consensus_proof = prove_header_update(&client, &trusted_state, target_height).await?;
		trace!("Consensus proof generated");
		consensus_proof.validate()?;
		let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
		trace!("Verification started");
		match tendermint_verifier::verify_header_update(
			trusted_state,
			consensus_proof,
			VerificationOptions::default(),
			current_time,
		) {
			Ok(updated_state) => {
				trace!(
					"Verification successful! Updated trusted state height: {}",
					updated_state.trusted_state.height
				);
			},
			Err(VerificationError::NotEnoughTrust(tally)) => {
				return Err(format!("Not enough trust: {}", tally).into());
			},
			Err(VerificationError::Invalid(detail)) => {
				return Err(format!("Invalid proof: {}", detail).into());
			},
			Err(e) => {
				return Err(format!("Verification failed: {:?}", e).into());
			},
		}
		Ok(())
	}

	/// Basic Heimdall RPC test: header and validator retrieval
	async fn test_polygon_basic_rpc(rpc_url: &str) -> Result<(), Box<dyn std::error::Error>> {
		let client = HeimdallClient::new(rpc_url);
		ensure_healthy(&client).await?;
		let chain_id = client.chain_id().await?;
		trace!("Chain ID: {}", chain_id);
		let latest_height = client.latest_height().await?;
		trace!("Latest height: {}", latest_height);
		let test_height = latest_height.saturating_sub(10);
		let header = client.signed_header(test_height).await?;
		trace!("Successfully retrieved signed header for height {}", test_height);
		trace!("Header chain ID: {}", header.header.chain_id);
		trace!("Header time: {}", header.header.time);
		let validators = client.validators(test_height).await?;
		trace!("Successfully retrieved {} validators", validators.len());
		if !validators.is_empty() {
			trace!("First validator pub key type: {:?}", validators[0].pub_key);
		}
		Ok(())
	}

	async fn ensure_healthy<T: Client>(client: &T) -> Result<(), Box<dyn std::error::Error>> {
		match client.is_healthy().await {
			Ok(true) => Ok(()),
			Ok(false) => Err("RPC client is not healthy".into()),
			Err(e) => Err(format!("Failed to check health: {}", e).into()),
		}
	}
}
