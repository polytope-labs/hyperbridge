#[cfg(test)]
mod tests {
	use std::time::{SystemTime, UNIX_EPOCH};

	use crate::{prove_header_update, TendermintRpcClient};
	use tendermint_verifier::{TrustedState, VerificationError, VerificationOptions};

	const STANDARD_RPC_URL: &str =
		"";
	const POLYGON_RPC_URL: &str = "https://polygon-amoy-heimdall-rpc.publicnode.com:443";

	#[tokio::test]
	async fn test_standard_tendermint_integration() {
		println!("Testing Standard Tendermint (Osmosis)");
		run_integration_test(STANDARD_RPC_URL).await.unwrap();
	}

	#[tokio::test]
	async fn test_polygon_peppermint_basic_rpc() {
		println!("Testing Polygon's Peppermint Fork (Basic RPC)");
		test_polygon_basic_rpc(POLYGON_RPC_URL).await.unwrap();
	}

	// Fails with Invalid Proof, debugging
	#[tokio::test]
	async fn test_polygon_peppermint_full_verification() {
		println!("Testing Polygon's Peppermint Fork (Full Verification)");
		run_integration_test(POLYGON_RPC_URL).await.unwrap();
	}

	/// Full integration test: prover and verifier
	async fn run_integration_test(rpc_url: &str) -> Result<(), Box<dyn std::error::Error>> {
		let client = TendermintRpcClient::new(rpc_url).await?;
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
		println!("Consensus proof generation started");
		let consensus_proof = prove_header_update(&client, &trusted_state, target_height).await?;
		println!("Consensus proof generated");
		consensus_proof.validate()?;
		let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
		println!("Verification started");
		match tendermint_verifier::verify_header_update(
			trusted_state,
			consensus_proof,
			VerificationOptions::default(),
			current_time,
		) {
			Ok(updated_state) => {
				println!(
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

	/// Basic Polygon RPC test: header and validator retrieval
	async fn test_polygon_basic_rpc(rpc_url: &str) -> Result<(), Box<dyn std::error::Error>> {
		let client = TendermintRpcClient::new(rpc_url).await?;
		ensure_healthy(&client).await?;
		let chain_id = client.chain_id().await?;
		println!("Chain ID: {}", chain_id);
		let latest_height = client.latest_height().await?;
		println!("Latest height: {}", latest_height);
		let test_height = latest_height.saturating_sub(10);
		let header = client.signed_header(test_height).await?;
		println!("Successfully retrieved signed header for height {}", test_height);
		println!("Header chain ID: {}", header.header.chain_id);
		println!("Header time: {}", header.header.time);
		let validators = client.validators(test_height).await?;
		println!("Successfully retrieved {} validators", validators.len());
		if !validators.is_empty() {
			println!("First validator pub key type: {:?}", validators[0].pub_key);
		}
		Ok(())
	}

	async fn ensure_healthy(
		client: &TendermintRpcClient,
	) -> Result<(), Box<dyn std::error::Error>> {
		match client.is_healthy().await {
			Ok(true) => Ok(()),
			Ok(false) => Err("RPC client is not healthy".into()),
			Err(e) => Err(format!("Failed to check health: {}", e).into()),
		}
	}
}
