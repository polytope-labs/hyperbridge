#[cfg(test)]
mod tests {
	use std::time::{SystemTime, UNIX_EPOCH};

	use crate::{prove_header_update, CometBFTClient};
	use ismp_polygon::Milestone;
	use tendermint_primitives::{Client, TrustedState, VerificationError, VerificationOptions};
	use tesseract_polygon::HeimdallClient;
	use tokio::time::{timeout, Duration};
	use tracing::trace;

	fn get_standard_rpc_url() -> String {
		std::env::var("STANDARD_TENDERMINT_URL")
			.expect("STANDARD_TENDERMINT_URL environment variable must be set")
	}

	fn get_polygon_rpc_url() -> String {
		std::env::var("POLYGON_HEIMDALL")
			.expect("POLYGON_HEIMDALL environment variable must be set")
	}

	fn get_polygon_rest_url() -> String {
		std::env::var("POLYGON_HEIMDALL_REST")
			.expect("POLYGON_HEIMDALL_REST environment variable must be set")
	}

	fn get_polygon_execution_rpc_url() -> String {
		std::env::var("POLYGON_EXECUTION_RPC")
			.expect("POLYGON_EXECUTION_RPC environment variable must be set")
	}

	const VALIDATOR_SET_TRANSITIONS: u32 = 8;

	#[tokio::test]
	#[ignore]
	async fn test_standard_tendermint_integration() {
		let _ = tracing_subscriber::fmt::try_init();
		trace!(
			"Testing Standard Tendermint with {} validator set transitions",
			VALIDATOR_SET_TRANSITIONS
		);
		let result = timeout(
			Duration::from_secs(600),
			run_integration_test_standard(&get_standard_rpc_url()),
		)
		.await;
		match result {
			Ok(inner) => inner.unwrap(),
			Err(_) => panic!("Test timed out after 10 minutes"),
		}
	}

	#[tokio::test]
	#[ignore]
	async fn test_polygon_heimdall_basic_rpc() {
		let _ = tracing_subscriber::fmt::try_init();
		trace!("Testing Polygon's Heimdall Fork (Basic RPC)");
		let result =
			timeout(Duration::from_secs(600), test_polygon_basic_rpc(&get_polygon_rpc_url())).await;
		match result {
			Ok(inner) => inner.unwrap(),
			Err(_) => panic!("Test timed out after 10 minutes"),
		}
	}

	#[tokio::test]
	#[ignore]
	async fn test_polygon_heimdall_full_verification() {
		let _ = tracing_subscriber::fmt::try_init();
		trace!(
			"Testing Polygon's Heimdall Fork (Full Verification) with {} validator set transitions",
			VALIDATOR_SET_TRANSITIONS
		);
		let result = timeout(
			Duration::from_secs(600),
			run_integration_test_heimdall(&get_polygon_rpc_url()),
		)
		.await;
		match result {
			Ok(inner) => inner.unwrap(),
			Err(_) => panic!("Test timed out after 10 minutes"),
		}
	}

	#[tokio::test]
	#[ignore]
	async fn test_abci_query_for_milestone_proof() {
		use cometbft_rpc::endpoint::abci_query::AbciQuery;

		let _ = tracing_subscriber::fmt::try_init();

		let client = HeimdallClient::new(
			&get_polygon_rpc_url(),
			&get_polygon_rest_url(),
			&get_polygon_execution_rpc_url(),
		)
		.expect("Failed to create client");

		let (milestone_number, milestone) =
			client.get_latest_milestone().await.expect("Failed to fetch milestone");

		let latest_height = client.latest_height().await.expect("Failed to fetch latest height");
		let abci_query: AbciQuery = client
			.get_ics23_proof(milestone_number, latest_height)
			.await
			.expect("ABCI query failed");

		let milestone_proto =
			Milestone::proto_decode(&abci_query.value).expect("Failed to decode milestone");
		assert_eq!(milestone_proto, milestone);

		assert_eq!(abci_query.code, cometbft::abci::Code::Ok);
		assert!(abci_query.proof.is_some(), "Proof should be present");
		assert!(!abci_query.value.is_empty(), "Value should not be empty");
	}

	/// Full integration test: prover and verifier for standard CometBFT with multiple validator set
	/// transitions
	async fn run_integration_test_standard(
		rpc_url: &str,
	) -> Result<(), Box<dyn std::error::Error>> {
		let client = CometBFTClient::new(rpc_url).await?;
		ensure_healthy(&client).await?;
		let chain_id = client.chain_id().await?;
		let latest_height = client.latest_height().await?;

		let trusted_height = latest_height.saturating_sub(50);
		let trusted_header = client.signed_header(trusted_height).await?;
		let trusted_validators = client.validators(trusted_height).await?;
		let trusted_next_validators = client.next_validators(trusted_height).await?;

		let mut trusted_state = TrustedState::new(
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

		trace!(
			"Starting verification with {} validator set transitions",
			VALIDATOR_SET_TRANSITIONS
		);
		trace!("Initial trusted height: {}", trusted_state.height);

		// Perform multiple validator set transitions
		let mut actual_transitions = 0;
		let mut attempt = 1;
		while actual_transitions < VALIDATOR_SET_TRANSITIONS {
			let consensus_proof =
				prove_header_update(&client, &trusted_state, trusted_state.height + 5).await?;
			consensus_proof.validate()?;

			let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
			trace!("Attempt {}: Verifying consensus proof", attempt);

			match tendermint_verifier::verify_header_update(
				trusted_state.clone(),
				consensus_proof.clone(),
				current_time,
			) {
				Ok(updated_state) => {
					let is_validator_set_change = trusted_state.next_validators_hash ==
						consensus_proof.signed_header.header.validators_hash.as_bytes();
					if is_validator_set_change {
						actual_transitions += 1;
					}
					trace!(
						"Update {} successful! Updated trusted state height: {} ({}/{})",
						attempt,
						updated_state.trusted_state.height,
						actual_transitions,
						VALIDATOR_SET_TRANSITIONS
					);
					trusted_state = updated_state.trusted_state;
				},
				Err(VerificationError::NotEnoughTrust(tally)) => {
					return Err(format!(
						"Transition {}: Not enough trust: {}",
						actual_transitions, tally
					)
					.into());
				},
				Err(VerificationError::Invalid(detail)) => {
					return Err(format!(
						"Transition {}: Invalid proof: {}",
						actual_transitions, detail
					)
					.into());
				},
				Err(e) => {
					return Err(format!(
						"Transition {}: Verification failed: {:?}",
						actual_transitions, e
					)
					.into());
				},
			}
			attempt += 1;
		}

		trace!("Successfully completed {} validator set transitions", VALIDATOR_SET_TRANSITIONS);
		Ok(())
	}

	/// Full integration test: prover and verifier for Heimdall with multiple validator set
	/// transitions
	async fn run_integration_test_heimdall(
		rpc_url: &str,
	) -> Result<(), Box<dyn std::error::Error>> {
		let client: HeimdallClient =
			HeimdallClient::new(rpc_url, &get_polygon_rest_url(), &get_polygon_execution_rpc_url())
				.expect("Failed to create client");
		ensure_healthy(&client).await?;
		let chain_id = client.chain_id().await?;
		let latest_height = client.latest_height().await?;

		let trusted_height = latest_height.saturating_sub(50);
		let trusted_header = client.signed_header(trusted_height).await?;
		let trusted_validators = client.validators(trusted_height).await?;
		let trusted_next_validators = client.next_validators(trusted_height).await?;

		let mut trusted_state = TrustedState::new(
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

		trace!(
			"Starting verification with {} validator set transitions",
			VALIDATOR_SET_TRANSITIONS
		);
		trace!("Initial trusted height: {}", trusted_state.height);

		let mut actual_transitions = 0;
		let mut attempt = 1;
		while actual_transitions < VALIDATOR_SET_TRANSITIONS {
			let consensus_proof =
				prove_header_update(&client, &trusted_state, trusted_state.height + 5).await?;
			consensus_proof.validate()?;

			let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
			trace!("Attempt {}: Verifying consensus proof", attempt);

			match tendermint_verifier::verify_header_update(
				trusted_state.clone(),
				consensus_proof.clone(),
				current_time,
			) {
				Ok(updated_state) => {
					let is_validator_set_change = trusted_state.next_validators_hash ==
						consensus_proof.signed_header.header.validators_hash.as_bytes();
					if is_validator_set_change {
						actual_transitions += 1;
					}
					trace!(
						"Update {} successful! Updated trusted state height: {} ({}/{})",
						attempt,
						updated_state.trusted_state.height,
						actual_transitions,
						VALIDATOR_SET_TRANSITIONS
					);
					trusted_state = updated_state.trusted_state;
				},
				Err(VerificationError::NotEnoughTrust(tally)) => {
					return Err(format!(
						"Transition {}: Not enough trust: {}",
						actual_transitions, tally
					)
					.into());
				},
				Err(VerificationError::Invalid(detail)) => {
					return Err(format!(
						"Transition {}: Invalid proof: {}",
						actual_transitions, detail
					)
					.into());
				},
				Err(e) => {
					return Err(format!(
						"Transition {}: Verification failed: {:?}",
						actual_transitions, e
					)
					.into());
				},
			}
			attempt += 1;
		}

		trace!("Successfully completed {} validator set transitions", VALIDATOR_SET_TRANSITIONS);
		Ok(())
	}

	/// Basic Heimdall RPC test: header and validator retrieval
	async fn test_polygon_basic_rpc(rpc_url: &str) -> Result<(), Box<dyn std::error::Error>> {
		let client =
			HeimdallClient::new(rpc_url, &get_polygon_rest_url(), &get_polygon_execution_rpc_url())
				.expect("Failed to create client");
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
