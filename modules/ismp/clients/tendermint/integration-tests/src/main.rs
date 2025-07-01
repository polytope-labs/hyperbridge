use std::time::{SystemTime, UNIX_EPOCH};

use tendermint_prover::{prove_header_update, TendermintRpcClient};
use tendermint_verifier::{
	ConsensusProof, TrustedState, UpdatedTrustedState, VerificationError, VerificationOptions,
};

// Osmosis testnet because Polygon Amoy Heimdall RPC endpoint is not working
const RPC_URL: &str = "https://rpc.osmotest5.osmosis.zone:443";

#[tokio::main]
async fn main() {
	println!("Starting Tendermint Prover-Verifier Integration Test");
	println!("Using RPC endpoint: {}", RPC_URL);

	if let Err(e) = run_integration_test().await {
		eprintln!("Integration test failed: {}", e);
		std::process::exit(1);
	}

	println!("All integration tests completed successfully!");
}

async fn run_integration_test() -> Result<(), Box<dyn std::error::Error>> {
	let client = match TendermintRpcClient::new(RPC_URL).await {
		Ok(client) => client,
		Err(e) => {
			return Err(format!("Failed to connect to RPC endpoint: {}", e).into());
		},
	};

	match client.is_healthy().await {
		Ok(true) => println!("RPC client is healthy"),
		Ok(false) => {
			return Err("RPC client is not healthy".into());
		},
		Err(e) => {
			return Err(format!("Failed to check health: {}", e).into());
		},
	}

	let chain_id = match client.chain_id().await {
		Ok(id) => {
			println!("Chain ID: {}", id);
			id
		},
		Err(e) => {
			return Err(format!("Failed to get chain ID: {}", e).into());
		},
	};

	let latest_height = match client.latest_height().await {
		Ok(height) => {
			println!("Latest height: {}", height);
			height
		},
		Err(e) => {
			return Err(format!("Failed to get latest height: {}", e).into());
		},
	};

	// Let's use a block that's a few blocks behind the latest
	let trusted_height = latest_height.saturating_sub(50);
	println!("Using trusted height: {}", trusted_height);

	let trusted_header = match client.signed_header(trusted_height).await {
		Ok(header) => header,
		Err(e) => {
			return Err(format!("Failed to get trusted header: {}", e).into());
		},
	};

	let trusted_validators = match client.validators(trusted_height).await {
		Ok(validators) => validators,
		Err(e) => {
			return Err(format!("Failed to get trusted validators: {}", e).into());
		},
	};

	let trusted_next_validators = match client.next_validators(trusted_height).await {
		Ok(validators) => validators,
		Err(e) => {
			return Err(format!("Failed to get trusted next validators: {}", e).into());
		},
	};

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

	if let Err(e) = trusted_state.validate() {
		return Err(format!("Invalid trusted state: {}", e).into());
	}

	println!("Created trusted state for height {}", trusted_height);

	let target_height = latest_height.saturating_sub(5);
	println!("Testing with target height: {}", target_height);

	let consensus_proof = match prove_header_update(&client, &trusted_state, target_height).await {
		Ok(proof) => {
			println!("Successfully created consensus proof for height {}", target_height);
			proof
		},
		Err(e) => {
			return Err(format!("Failed to create consensus proof: {}", e).into());
		},
	};

	if let Err(e) = consensus_proof.validate() {
		return Err(format!("Invalid consensus proof: {}", e).into());
	}

	println!("Consensus proof validated successfully");

	let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

	// For reviewers:Fails with Invalid proof - Debugging
	let verification_result = tendermint_verifier::verify_header_update(
		trusted_state,
		consensus_proof,
		VerificationOptions::default(),
		current_time,
	);

	match verification_result {
		Ok(updated_state) => {
			println!("Verification successful!");
			println!("Updated trusted state height: {}", updated_state.trusted_state.height);
			println!("Verified height: {}", updated_state.verified_height);
			println!("Height difference: {}", updated_state.height_difference());
			println!("Time difference: {} seconds", updated_state.time_difference());
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
