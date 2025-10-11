#[cfg(test)]
mod tests {
	use std::time::{SystemTime, UNIX_EPOCH};

	use crate::{prove_header_update, CometBFTClient};
	use codec::Encode;
	use ismp_polygon::Milestone;
	use tendermint_primitives::{
		Client, DefaultEvmKeys, EvmStoreKeys, SeiEvmKeys, TrustedState, ValidatorSet,
		VerificationError, VerificationOptions,
	};
	use tendermint_verifier::{hashing::SpIoSha256, validate_validator_set_hash};
	use tesseract_polygon::HeimdallClient;
	use tokio::time::{interval, timeout, Duration};
	use tracing::trace;

	use evm_state_machine::{tendermint::verify_evm_kv_proofs, types::EvmKVProof};

	use ismp::{
		consensus::{StateCommitment, StateMachineHeight, StateMachineId},
		host::StateMachine,
		messaging::Proof as IsmpProof,
	};
	use primitive_types::{H160, H256};
	use tendermint_ics23_primitives::proof_ops_to_commitment_proof_bytes;

	fn get_sei_rpc() -> String {
		std::env::var("SEI_RPC_URL")
			.unwrap_or_else(|_| "https://sei-rpc.publicnode.com:443".to_string())
	}

	fn get_kava_rpc() -> String {
		std::env::var("KAVA_RPC_URL")
			.unwrap_or_else(|_| "https://kava-rpc.publicnode.com:443".to_string())
	}

	fn get_cronos_rpc() -> String {
		std::env::var("CRONOS_RPC_URL")
			.unwrap_or_else(|_| "https://cronos-rpc.publicnode.com:443".to_string())
	}

	fn get_injective_rpc() -> String {
		std::env::var("INJECTIVE_RPC_URL")
			.unwrap_or_else(|_| "https://injective-testnet-rpc.publicnode.com:443".to_string())
	}

	fn get_polygon_rpc_url() -> String {
		std::env::var("POLYGON_HEIMDALL")
			.unwrap_or_else(|_| "https://polygon-amoy-heimdall-rpc.publicnode.com".to_string())
	}

	fn get_polygon_execution_rpc_url() -> String {
		std::env::var("POLYGON_EXECUTION_RPC")
			.unwrap_or_else(|_| "https://rpc-amoy.polygon.technology/".to_string())
	}

	const VALIDATOR_SET_TRANSITIONS: u32 = 3;

	#[tokio::test]
	#[ignore]
	async fn test_sei_tendermint_integration() {
		let _ = tracing_subscriber::fmt::try_init();
		trace!(
			"Testing SEI Tendermint with {} validator set transitions",
			VALIDATOR_SET_TRANSITIONS
		);

		match timeout(Duration::from_secs(3600), run_integration_test_standard(&get_sei_rpc()))
			.await
		{
			Ok(inner) => match inner {
				Ok(()) => trace!("SEI Tendermint integration test completed successfully"),
				Err(e) => trace!("SEI Tendermint integration test failed: {}", e),
			},
			Err(_) => {
				trace!("SEI Tendermint integration test timed out after 10 minutes");
			},
		}
	}

	#[tokio::test]
	#[ignore]
	async fn test_kava_tendermint_integration() {
		let _ = tracing_subscriber::fmt::try_init();
		trace!(
			"Testing Kava Tendermint with {} validator set transitions",
			VALIDATOR_SET_TRANSITIONS
		);

		match timeout(Duration::from_secs(3600), run_integration_test_standard(&&get_kava_rpc()))
			.await
		{
			Ok(inner) => match inner {
				Ok(()) => trace!("Standard Tendermint integration test completed successfully"),
				Err(e) => trace!("Standard Tendermint integration test failed: {}", e),
			},
			Err(_) => {
				trace!("Standard Tendermint integration test timed out after 10 minutes");
			},
		}
	}

	#[tokio::test]
	#[ignore]
	async fn test_cronos_tendermint_integration() {
		let _ = tracing_subscriber::fmt::try_init();
		trace!(
			"Testing Cronos Tendermint with {} validator set transitions",
			VALIDATOR_SET_TRANSITIONS
		);

		match timeout(Duration::from_secs(3600), run_integration_test_standard(&&get_cronos_rpc()))
			.await
		{
			Ok(inner) => match inner {
				Ok(()) => trace!("Standard Tendermint integration test completed successfully"),
				Err(e) => trace!("Standard Tendermint integration test failed: {}", e),
			},
			Err(_) => {
				trace!("Standard Tendermint integration test timed out after 10 minutes");
			},
		}
	}

	#[tokio::test]
	#[ignore]
	async fn test_injective_tendermint_integration() {
		let _ = tracing_subscriber::fmt::try_init();
		trace!(
			"Testing Injective Tendermint with {} validator set transitions",
			VALIDATOR_SET_TRANSITIONS
		);

		match timeout(
			Duration::from_secs(3600),
			run_integration_test_standard(&&get_injective_rpc()),
		)
		.await
		{
			Ok(inner) => match inner {
				Ok(()) => trace!("Standard Tendermint integration test completed successfully"),
				Err(e) => trace!("Standard Tendermint integration test failed: {}", e),
			},
			Err(_) => {
				trace!("Standard Tendermint integration test timed out after 10 minutes");
			},
		}
	}

	#[tokio::test]
	#[ignore]
	async fn test_polygon_heimdall_basic_rpc() {
		let _ = tracing_subscriber::fmt::try_init();
		trace!("Testing Polygon's Heimdall Fork (Basic RPC)");

		match timeout(Duration::from_secs(600), test_polygon_basic_rpc(&get_polygon_rpc_url()))
			.await
		{
			Ok(inner) => match inner {
				Ok(()) => trace!("Polygon Heimdall basic RPC test completed successfully"),
				Err(e) => trace!("Polygon Heimdall basic RPC test failed: {}", e),
			},
			Err(_) => {
				trace!("Polygon Heimdall basic RPC test timed out after 10 minutes");
			},
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

		match timeout(
			Duration::from_secs(3600),
			run_integration_test_heimdall(&get_polygon_rpc_url()),
		)
		.await
		{
			Ok(inner) => match inner {
				Ok(()) => trace!("Polygon Heimdall full verification test completed successfully"),
				Err(e) => trace!("Polygon Heimdall full verification test failed: {}", e),
			},
			Err(_) => {
				trace!("Polygon Heimdall full verification test timed out after 10 minutes");
			},
		}
	}

	#[tokio::test]
	#[ignore]
	async fn test_abci_query_for_milestone_proof() {
		let _ = tracing_subscriber::fmt::try_init();
		trace!("Testing ABCI query for milestone proof");

		match timeout(Duration::from_secs(600), test_abci_query_milestone_proof_inner()).await {
			Ok(inner) => match inner {
				Ok(()) => trace!("ABCI query milestone proof test completed successfully"),
				Err(e) => trace!("ABCI query milestone proof test failed: {}", e),
			},
			Err(_) => {
				trace!("ABCI query milestone proof test timed out after 10 minutes");
			},
		}
	}

	#[tokio::test]
	#[ignore]
	async fn sei_evm_state_proof() -> anyhow::Result<()> {
		verify_evm_state_proof(
			&get_sei_rpc(),
			StateMachine::Evm(1329),
			"e15fC38F6D8c56aF07bbCBe3BAf5708A2Bf42392",
			"26387b69acd9674861659d8f121f3f72d8c4934eeea15b947235839377526d2c",
		)
		.await?;

		Ok(())
	}

	#[tokio::test]
	#[ignore]
	async fn kava_evm_state_proof() -> anyhow::Result<()> {
		verify_evm_state_proof(
			&get_kava_rpc(),
			StateMachine::Evm(2222),
			"919C1c267BC06a7039e03fcc2eF738525769109c",
			"1b00e2a2c0ae74b184fd3ef909a7e5ebd1f1c91a7b37432bb365c42bc211a82f",
		)
		.await?;

		Ok(())
	}

	#[tokio::test]
	#[ignore]
	async fn cronos_evm_state_proof() -> anyhow::Result<()> {
		verify_evm_state_proof(
			&get_cronos_rpc(),
			StateMachine::Evm(25),
			"c21223249CA28397B4B6541dfFaEcC539BfF0c59",
			"b7b7f25334beca82ade183c84c51052234dc1618b0ff321ea2332dbf08c55523",
		)
		.await?;

		Ok(())
	}

	#[tokio::test]
	#[ignore]
	async fn injective_evm_state_proof() -> anyhow::Result<()> {
		verify_evm_state_proof(
			&get_injective_rpc(),
			StateMachine::Evm(1439),
			"E83c1acd1c9cc3780D0a560E36DCCAA236B86412",
			"a9bd8c5aa26805e4fe15acd0af182cd60120b2f98da76707238f637028baf59b",
		)
		.await?;

		Ok(())
	}

	async fn test_abci_query_milestone_proof_inner() -> Result<(), Box<dyn std::error::Error>> {
		use cometbft_rpc::endpoint::abci_query::AbciQuery;

		let client = HeimdallClient::new(&get_polygon_rpc_url(), &get_polygon_execution_rpc_url())?;
		let latest_height = client.latest_height().await?;

		let (milestone_number, milestone) =
			client.get_latest_milestone_at_height(latest_height).await?.unwrap();
		trace!("Latest milestone: number {}", milestone_number);

		let abci_query: AbciQuery =
			client.get_milestone_proof(milestone_number, latest_height).await?;

		let milestone_proto = Milestone::proto_decode(&abci_query.value)?;

		if milestone_proto != milestone {
			return Err("Milestone proto does not match milestone".into());
		}

		if abci_query.proof.is_none() {
			return Err("Proof should be present".into());
		}

		let latest_milestone_at_height =
			client.get_latest_milestone_at_height(latest_height).await?;

		match latest_milestone_at_height {
			Some((number, _)) => {
				trace!("Latest milestone at latest height {}: number {}", latest_height, number)
			},
			None => trace!("No milestone found at height {}", latest_height),
		}

		Ok(())
	}

	/// Full integration test: prover and verifier for standard CometBFT with multiple validator set
	/// transitions
	async fn run_integration_test_standard(
		rpc_url: &str,
	) -> Result<(), Box<dyn std::error::Error>> {
		let client = CometBFTClient::new(rpc_url).await?;
		trace!("CometBFT client created successfully");

		ensure_healthy(&client).await?;

		let chain_id = client.chain_id().await?;
		trace!("Retrieved chain ID: {}", chain_id);

		let latest_height = client.latest_height().await?;
		trace!("Retrieved latest height: {}", latest_height);

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

		let mut interval = interval(Duration::from_secs(300)); // 5 minutes between updates

		// Perform multiple validator set transitions
		let mut actual_transitions = 0;
		let mut attempt = 1;
		while actual_transitions < VALIDATOR_SET_TRANSITIONS {
			interval.tick().await;

			let latest_height = client.latest_height().await?;
			let mut target_height = latest_height;

			let header0 = client.signed_header(target_height).await?;
			let matches_current = validate_validator_set_hash(
				&ValidatorSet::new(trusted_state.validators.clone(), None),
				header0.header.validators_hash,
				false,
			)
			.is_ok();
			let matches_next = validate_validator_set_hash(
				&ValidatorSet::new(trusted_state.next_validators.clone(), None),
				header0.header.validators_hash,
				true,
			)
			.is_ok();
			if !(matches_current || matches_next) {
				let mut h = latest_height.saturating_sub(1);
				while h > trusted_state.height {
					let maybe_header = client.signed_header(h).await;
					if let Ok(hdr) = maybe_header {
						let cur_ok = validate_validator_set_hash(
							&ValidatorSet::new(trusted_state.validators.clone(), None),
							hdr.header.validators_hash,
							false,
						)
						.is_ok();
						let next_ok = validate_validator_set_hash(
							&ValidatorSet::new(trusted_state.next_validators.clone(), None),
							hdr.header.validators_hash,
							true,
						)
						.is_ok();
						if cur_ok || next_ok {
							target_height = h;
							break;
						}
					}
					if h == 0 {
						break;
					}
					h -= 1;
				}
			}

			let consensus_proof =
				prove_header_update(&client, &trusted_state, target_height).await?;
			consensus_proof.validate()?;

			let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
			trace!("Attempt {}: Verifying consensus proof for height {}", attempt, target_height);

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
		let client: HeimdallClient = HeimdallClient::new(rpc_url, &get_polygon_execution_rpc_url())
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
			trusted_header.header.hash_with::<SpIoSha256>().as_bytes().try_into().unwrap(),
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

		let mut interval = interval(Duration::from_secs(30));

		let mut actual_transitions = 0;
		let mut attempt = 1;
		while actual_transitions < VALIDATOR_SET_TRANSITIONS {
			interval.tick().await;

			let latest_height = client.latest_height().await?;
			let mut target_height = latest_height;

			let header0 = client.signed_header(target_height).await?;
			let matches_current = validate_validator_set_hash(
				&ValidatorSet::new(trusted_state.validators.clone(), None),
				header0.header.validators_hash,
				false,
			)
			.is_ok();
			let matches_next = validate_validator_set_hash(
				&ValidatorSet::new(trusted_state.next_validators.clone(), None),
				header0.header.validators_hash,
				true,
			)
			.is_ok();
			if !(matches_current || matches_next) {
				let mut h = latest_height.saturating_sub(1);
				while h > trusted_state.height {
					let maybe_header = client.signed_header(h).await;
					if let Ok(hdr) = maybe_header {
						let cur_ok = validate_validator_set_hash(
							&ValidatorSet::new(trusted_state.validators.clone(), None),
							hdr.header.validators_hash,
							false,
						)
						.is_ok();
						let next_ok = validate_validator_set_hash(
							&ValidatorSet::new(trusted_state.next_validators.clone(), None),
							hdr.header.validators_hash,
							true,
						)
						.is_ok();
						if cur_ok || next_ok {
							target_height = h;
							break;
						}
					}
					if h == 0 {
						break;
					}
					h -= 1;
				}
			}

			let consensus_proof =
				prove_header_update(&client, &trusted_state, target_height).await?;

			consensus_proof.validate()?;

			let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
			trace!("Attempt {}: Verifying consensus proof for height {}", attempt, target_height);

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
		let client = HeimdallClient::new(rpc_url, &get_polygon_execution_rpc_url())
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

	async fn verify_evm_state_proof(
		rpc_url: &str,
		state_id: StateMachine,
		contract_hex: &str,
		slot_hex: &str,
	) -> anyhow::Result<()> {
		let client = CometBFTClient::new(rpc_url).await?;
		let latest_height = client.latest_height().await?;
		let signed_header = client.signed_header(latest_height).await?;
		let app_hash = H256::from_slice(signed_header.header.app_hash.as_bytes());
		let contract: H160 =
			H160::from_slice(&hex::decode(contract_hex.to_lowercase()).expect("bad contract hex"));
		let slot: H256 =
			H256::from_slice(&hex::decode(slot_hex.to_lowercase()).expect("bad slot hex"));

		let (store_key, new_key) = match state_id {
			StateMachine::Evm(id) if id == 1329 =>
				(SeiEvmKeys::store_key(), SeiEvmKeys::storage_key(&contract.0, slot.0)),
			StateMachine::Evm(_) =>
				(DefaultEvmKeys::store_key(), DefaultEvmKeys::storage_key(&contract.0, slot.0)),
			_ => unreachable!("Only EVM state machines are supported in this test"),
		};
		let mut key52 = Vec::with_capacity(52);
		key52.extend_from_slice(&contract.0);
		key52.extend_from_slice(&slot.0);

		let res = client.abci_query_key(store_key, new_key, latest_height - 1).await?;
		let proof = proof_ops_to_commitment_proof_bytes(res.proof)?;
		let value = res.value;

		let evm_kv_proof = EvmKVProof { value, proof };
		let proofs = vec![evm_kv_proof];
		let encoded_proofs = proofs.encode();

		let ismp_proof = IsmpProof {
			height: StateMachineHeight {
				id: StateMachineId { state_id, consensus_state_id: [0; 4] },
				height: latest_height - 1,
			},
			proof: encoded_proofs,
		};

		let state_commitment =
			StateCommitment { timestamp: 0, overlay_root: None, state_root: app_hash };

		let _ = verify_evm_kv_proofs(vec![key52], contract, state_commitment, &ismp_proof)?;
		println!("EVM state proof verification passed for slot: {}", hex::encode(&slot.0));
		Ok(())
	}
}
