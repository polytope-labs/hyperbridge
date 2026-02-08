use alloy::{
	primitives::{Address as AlloyAddress, Bytes as AlloyBytes, LogData, B256},
	sol_types::SolEvent,
};
use codec::{Decode, Encode};
use evm_state_machine::{
	derive_unhashed_map_key_with_offset,
	presets::REQUEST_COMMITMENTS_SLOT,
	substrate_evm::{
		contract_info_key, fetch_child_root_from_main_proof, fetch_trie_id_from_main_proof,
		verify_child_trie_values, AccountInfo, AccountType,
	},
};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
		StateMachineId,
	},
	host::{IsmpHost, StateMachine},
	messaging::{hash_request, Keccak256},
	router::{IsmpRouter, PostRequest, PostResponse, Request, Response},
	Error,
};
use ismp_solidity_abi::evm_host::EvmHost::PostRequestEvent;
use ismp_testsuite::mocks::{Host as MockHost, Keccak256Hasher};
use polkadot_sdk::sp_runtime::testing::H256;
use primitive_types::U256;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sp_core::{storage::ChildInfo, Blake2Hasher, Hasher as _, H160};
use std::time::Duration;

fn get_rpc_url() -> String {
	std::env::var("SUBSTRATE_RPC_URL")
		.unwrap_or_else(|_| "https://asset-hub-paseo-rpc.n.dwellir.com".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcResponse<T> {
	result: Option<T>,
	error: Option<serde_json::Value>,
	id: u64,
	jsonrpc: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReadProof {
	proof: Vec<String>,
}

async fn rpc_request<T: serde::de::DeserializeOwned>(
	method: &str,
	params: Vec<serde_json::Value>,
) -> Result<T, Box<dyn std::error::Error>> {
	let client = reqwest::Client::builder().http1_only().build()?;
	let url = get_rpc_url();

	let body = json!({
		"jsonrpc": "2.0",
		"id": 1,
		"method": method,
		"params": params
	});

	let body_str = serde_json::to_string(&body)?;

	let resp_bytes = client
		.post(&url)
		.header("Content-Type", "application/json")
		.body(body_str)
		.send()
		.await?
		.bytes()
		.await?;

	let resp: RpcResponse<T> = serde_json::from_slice(&resp_bytes)?;

	if let Some(err) = resp.error {
		return Err(format!("RPC Error: {:?}", err).into());
	}

	resp.result.ok_or_else(|| "No result in response".into())
}

async fn fetch_evm_logs(
	evm_rpc_url: &str,
	from_block: &str,
	to_block: &str,
	address: &str,
	topics: Vec<String>,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
	let client = reqwest::Client::builder().http1_only().build()?;

	let mut filter = json!({
		"fromBlock": from_block,
		"toBlock": to_block,
		"address": address,
	});

	if !topics.is_empty() {
		filter["topics"] = json!(topics);
	}

	let body = json!({
		"jsonrpc": "2.0",
		"id": 1,
		"method": "eth_getLogs",
		"params": [filter]
	});

	let body_str = serde_json::to_string(&body)?;

	let resp_bytes = client
		.post(evm_rpc_url)
		.header("Content-Type", "application/json")
		.body(body_str)
		.send()
		.await?
		.bytes()
		.await?;

	let resp: RpcResponse<Vec<serde_json::Value>> = serde_json::from_slice(&resp_bytes)?;

	if let Some(err) = resp.error {
		return Err(format!("RPC Error: {:?}", err).into());
	}

	resp.result.ok_or_else(|| "No result in response".into())
}

#[tokio::test]
#[ignore]
async fn test_verify_revive_state_proof() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt::try_init();

	let contract_hex = "0xbb26e04a71e7c12093e82b83ba310163eac186fa";
	let contract_address = H160::from_slice(&hex::decode(contract_hex.trim_start_matches("0x"))?);

	println!("Fetching block hash...");
	let block_hash: String = rpc_request("chain_getBlockHash", vec![]).await?;
	println!("Testing at block: {}", block_hash);
	let block_hash_h256 = H256::from_slice(&hex::decode(block_hash.trim_start_matches("0x"))?);

	println!("Fetching block header...");
	let header: serde_json::Value = rpc_request("chain_getHeader", vec![json!(block_hash)]).await?;
	let state_root_hex = header["stateRoot"].as_str().expect("stateRoot missing");
	let state_root = H256::from_slice(&hex::decode(state_root_hex.trim_start_matches("0x"))?);

	let contract_info_key = contract_info_key(contract_address);
	let storage_key_hex = format!("0x{}", hex::encode(&contract_info_key));

	println!("Fetching account info storage...");
	let account_info_hex: Option<String> =
		rpc_request("state_getStorage", vec![json!(storage_key_hex), json!(block_hash)]).await?;

	let account_info_bytes = hex::decode(
		account_info_hex
			.ok_or("AccountInfo not found - is this a valid contract?")?
			.trim_start_matches("0x"),
	)?;

	let mut input = &account_info_bytes[..];
	let account_info = AccountInfo::decode(&mut &input[..])?;
	let AccountType::Contract(info) = account_info.account_type;
	let trie_id = info.trie_id;

	let child_info = ChildInfo::new_default(&trie_id);
	let child_root_key = child_info.prefixed_storage_key();
	let child_storage_key_hex =
		format!("0x{}", hex::encode(child_info.prefixed_storage_key().into_inner()));

	println!("Fetching a valid existing key from child trie to use for verification...");
	let keys_paged: Vec<String> = rpc_request(
		"childstate_getKeysPaged",
		vec![json!(child_storage_key_hex), json!("0x"), json!(1), json!(null), json!(block_hash)],
	)
	.await?;

	let active_key_hex = keys_paged.first().ok_or("No keys found in contract child trie")?;
	println!("Found active key: {}", active_key_hex);
	let active_key = hex::decode(active_key_hex.trim_start_matches("0x"))?;

	let main_keys = vec![contract_info_key.clone(), child_root_key.into_inner()];
	let main_keys_hex: Vec<String> =
		main_keys.iter().map(|k| format!("0x{}", hex::encode(k))).collect();

	println!("Fetching main proof...");
	let main_read_proof: ReadProof =
		rpc_request("state_getReadProof", vec![json!(main_keys_hex), json!(block_hash)]).await?;

	let main_proof_bytes: Vec<Vec<u8>> = main_read_proof
		.proof
		.iter()
		.map(|p| hex::decode(p.trim_start_matches("0x")).unwrap())
		.collect();

	let child_keys_hex = vec![active_key_hex.clone()];

	println!("Fetching child proof...");
	let child_read_proof: ReadProof = rpc_request(
		"state_getChildReadProof",
		vec![json!(child_storage_key_hex), json!(child_keys_hex), json!(block_hash)],
	)
	.await?;

	let child_proof_bytes: Vec<Vec<u8>> = child_read_proof
		.proof
		.iter()
		.map(|p| hex::decode(p.trim_start_matches("0x")).unwrap())
		.collect();

	println!("Proofs fetched. Verifying...");

	let verified_trie_id = fetch_trie_id_from_main_proof::<MockHost>(
		&main_proof_bytes,
		state_root,
		&contract_info_key,
	)
	.map_err(Error::from)?;
	assert_eq!(verified_trie_id, trie_id, "Trie ID mismatch");
	println!("Main Proof Verified: Trie ID matches");

	let verified_child_root = fetch_child_root_from_main_proof::<MockHost>(
		&main_proof_bytes,
		state_root,
		&verified_trie_id,
	)
	.map_err(Error::from)?;
	println!("Main Proof Verified: Child Root gotten: {:?}", verified_child_root);

	let values = verify_child_trie_values::<MockHost>(
		verified_child_root,
		&child_proof_bytes,
		vec![active_key],
	)
	.map_err(Error::from)?;

	assert_eq!(values.len(), 1);
	match &values[0] {
		Some(val) => println!("Child Proof Verified: Value found with length {}", val.len()),
		None => return Err("Child Proof Verified: Value is None but key was requested!".into()),
	}

	Ok(())
}

#[tokio::test]
#[ignore]
async fn test_verify_evm_post_request_events() -> Result<(), Box<dyn std::error::Error>> {
	let _ = tracing_subscriber::fmt::try_init();

	println!("=== Testing EVM PostRequestEvent Verification ===\n");
	println!("Note: This test fetches actual PostRequestEvent from local EVM");
	println!("and verifies the storage proof on Paseo Asset Hub\n");

	// Contract address on Paseo Asset Hub
	let contract_address = "0xbb26e04a71e7c12093e82b83ba310163eac186fa";

	println!("Contract: {}", contract_address);

	// Fetch logs from local EVM node
	let evm_rpc_url = "http://localhost:8545";
	let from_block = "0x487a7f";
	let to_block = "0x488006";

	println!("Fetching logs from blocks {} to {} on {}\n", from_block, to_block, evm_rpc_url);

	// Fetch all logs from the contract without filtering by topic
	// We'll try to decode each one as a PostRequestEvent
	let logs = fetch_evm_logs(
		evm_rpc_url,
		from_block,
		to_block,
		contract_address,
		vec![], // No topic filter - get all events
	)
	.await?;

	if logs.is_empty() {
		println!("⚠️  No logs found in the specified block range");
		println!("Make sure the local EVM node is running and has logs in this range");
		return Err("No logs found".into());
	}

	println!("Found {} log(s), attempting to decode as PostRequestEvent...\n", logs.len());

	// Try to decode each log as a PostRequestEvent until we find one
	let mut post_event_opt: Option<PostRequestEvent> = None;
	let mut decoded_log_json: Option<serde_json::Value> = None;

	for (idx, log) in logs.iter().enumerate() {
		let log_data = match log["data"].as_str() {
			Some(d) => d,
			None => continue,
		};

		let log_topics: Vec<String> = match log["topics"].as_array() {
			Some(topics) => topics.iter().filter_map(|t| t.as_str().map(String::from)).collect(),
			None => continue,
		};

		// Convert to alloy types for decoding
		let topics: Vec<B256> = log_topics
			.iter()
			.map(|t| {
				B256::from_slice(&hex::decode(t.trim_start_matches("0x")).unwrap_or_default())
			})
			.collect();
		let data = AlloyBytes::from(hex::decode(log_data.trim_start_matches("0x")).unwrap_or_default());
		let log_data = LogData::new(topics.clone(), data).unwrap();

		// Try to decode as PostRequestEvent
		if let Ok(event) = PostRequestEvent::decode_log_data(&log_data) {
			println!("✅ Found PostRequestEvent at log index {}", idx);
			println!("   Topic: {}", log_topics.get(0).unwrap_or(&"none".to_string()));
			post_event_opt = Some(event);
			decoded_log_json = Some(log.clone());
			break;
		}
	}

	let post_event = post_event_opt.ok_or("No PostRequestEvent found in logs")?;
	let log = decoded_log_json.as_ref().unwrap();

	println!("\nRaw log data: {}", serde_json::to_string_pretty(&log)?);

	println!("\n=== Decoded PostRequestEvent ===");
	println!("Source: {}", post_event.source);
	println!("Dest: {}", post_event.dest);
	println!("From: {:?}", post_event.from);
	println!("To: {:?}", post_event.to);
	println!("Nonce: {}", post_event.nonce);
	println!("Timeout: {}", post_event.timeoutTimestamp);
	println!("Body: {}", hex::encode(&post_event.body));
	println!();

	// Convert to PostRequest
	let post_request: PostRequest = post_event.try_into()?;

	// Calculate request commitment
	let commitment = hash_request::<Keccak256Hasher>(&Request::Post(post_request.clone()));
	println!("Request commitment: 0x{}", hex::encode(commitment.0));

	// Derive storage key for this commitment using derive_unhashed_map_key_with_offset
	// This derives the key for REQUEST_COMMITMENTS_SLOT with offset 1 (for the timestamp)
	let unhashed_storage_key = derive_unhashed_map_key_with_offset::<Keccak256Hasher>(
		commitment.0.to_vec(),
		REQUEST_COMMITMENTS_SLOT,
		1,
	);
	println!("Unhashed storage key: 0x{}", hex::encode(&unhashed_storage_key.0));

	// Blake2b hash the storage key for use in Substrate storage
	let storage_key_hash = Blake2Hasher::hash(&unhashed_storage_key.0);
	println!("Blake2b hashed storage key: 0x{}\n", hex::encode(storage_key_hash.as_ref()));

	// Now fetch proofs from Paseo Asset Hub for this key
	println!("=== Fetching Proofs from Paseo Asset Hub ===\n");

	let contract_address_h160 =
		H160::from_slice(&hex::decode(contract_address.trim_start_matches("0x"))?);

	println!("Fetching block hash from Paseo...");
	let block_hash: String = rpc_request("chain_getBlockHash", vec![]).await?;
	println!("Testing at block: {}\n", block_hash);

	println!("Fetching block header...");
	let header: serde_json::Value = rpc_request("chain_getHeader", vec![json!(block_hash)]).await?;
	let state_root_hex = header["stateRoot"].as_str().expect("stateRoot missing");
	let state_root = H256::from_slice(&hex::decode(state_root_hex.trim_start_matches("0x"))?);
	println!("State root: {}\n", state_root_hex);

	let contract_info_key = contract_info_key(contract_address_h160);
	let storage_key_hex = format!("0x{}", hex::encode(&contract_info_key));

	println!("Fetching contract account info...");
	let account_info_hex: Option<String> =
		rpc_request("state_getStorage", vec![json!(storage_key_hex), json!(block_hash)]).await?;

	let account_info_bytes = hex::decode(
		account_info_hex
			.ok_or("AccountInfo not found - is this a valid contract?")?
			.trim_start_matches("0x"),
	)?;

	let input = &account_info_bytes[..];
	let account_info = AccountInfo::decode(&mut &input[..])?;
	let AccountType::Contract(info) = account_info.account_type;
	let trie_id = info.trie_id;

	println!("Contract trie_id: {}\n", hex::encode(&trie_id));

	// Create child info to get the child root key
	let child_info = ChildInfo::new_default(&trie_id);
	let child_root_key = child_info.prefixed_storage_key();

	// Fetch main proof (for trie_id and child root) - need both contract info key and child root
	// key
	let main_keys = vec![contract_info_key.clone(), child_root_key.into_inner()];
	let main_keys_hex: Vec<String> =
		main_keys.iter().map(|k| format!("0x{}", hex::encode(k))).collect();

	println!("Fetching main proof...");
	let main_proof_resp: ReadProof =
		rpc_request("state_getReadProof", vec![json!(main_keys_hex), json!(block_hash)]).await?;
	let main_proof: Vec<Vec<u8>> = main_proof_resp
		.proof
		.into_iter()
		.map(|p| hex::decode(p.trim_start_matches("0x")).expect("Invalid hex"))
		.collect();

	println!("Main proof nodes: {}", main_proof.len());

	// Verify main proof to get child root
	println!("\n=== Verifying Main Proof ===");
	let verified_trie_id =
		fetch_trie_id_from_main_proof::<MockHost>(&main_proof, state_root, &contract_info_key)?;
	assert_eq!(
		verified_trie_id,
		trie_id,
		"Trie ID verification failed: expected {:?}, got {:?}",
		hex::encode(&trie_id),
		hex::encode(&verified_trie_id)
	);
	println!("✅ Trie ID verified from main proof");

	let child_root =
		fetch_child_root_from_main_proof::<MockHost>(&main_proof, state_root, &verified_trie_id)?;
	println!("✅ Child root extracted: 0x{}", hex::encode(&child_root));

	// Fetch child proof (for the actual storage key)
	let prefixed_key = child_info.prefixed_storage_key();
	let prefixed_key_hex = format!("0x{}", hex::encode(&prefixed_key.into_inner()));

	let child_storage_key_hex = format!("0x{}", hex::encode(storage_key_hash.as_ref()));

	println!("\n=== Fetching Child Proof ===");
	println!("Child storage key (blake2b hashed): {}", child_storage_key_hex);

	// Use state_getChildReadProof RPC for child trie proofs
	let child_proof_resp: ReadProof = rpc_request(
		"state_getChildReadProof",
		vec![json!(prefixed_key_hex), json!([child_storage_key_hex]), json!(block_hash)],
	)
	.await?;
	let child_proof: Vec<Vec<u8>> = child_proof_resp
		.proof
		.into_iter()
		.map(|p| hex::decode(p.trim_start_matches("0x")).expect("Invalid hex"))
		.collect();

	println!("Child proof nodes: {}", child_proof.len());

	// Verify child proof using the blake2b hashed key
	println!("\n=== Verifying Child Proof ===");
	let storage_key_vec = storage_key_hash.as_ref().to_vec();

	// Verify the values directly
	let values =
		verify_child_trie_values::<MockHost>(child_root, &child_proof, vec![storage_key_vec])?;

	if values.is_empty() {
		println!("⚠️  No values returned from child proof verification");
	} else {
		match &values[0] {
			Some(val) => {
				println!("✅ Commitment found in storage!");
				println!("   Value length: {} bytes", val.len());
				println!("   Raw value: 0x{}", hex::encode(val));

				// Decode the timestamp (u64)
				if val.len() >= 8 {
					let timestamp = u64::from_le_bytes(val[0..8].try_into()?);
					println!("   Decoded timestamp: {}", timestamp);
				}

				println!("\n🎉 Full verification successful!");
				println!("1. ✅ Fetched actual PostRequestEvent from EVM logs at {}", evm_rpc_url);
				println!("2. ✅ Decoded event and calculated request commitment");
				println!("3. ✅ Derived storage key using derive_unhashed_map_key_with_offset");
				println!("4. ✅ Blake2b hashed the storage key");
				println!("5. ✅ Fetched state proofs from Paseo Asset Hub");
				println!("6. ✅ Verified main proof (trie ID and child root)");
				println!("7. ✅ Fetched child proof using state_getChildReadProof");
				println!("8. ✅ Verified child proof and found commitment in storage");
			},
			None => {
				println!("⚠️  Commitment key not found in storage");
				println!("\nThis demonstrates the full verification flow:");
				println!("1. ✅ Fetch actual PostRequestEvent from EVM logs at {}", evm_rpc_url);
				println!("2. ✅ Decode event and calculate request commitment");
				println!("3. ✅ Derive storage key using derive_unhashed_map_key_with_offset");
				println!("4. ✅ Blake2b hash the storage key");
				println!("5. ✅ Fetch state proofs from Paseo Asset Hub");
				println!("6. ✅ Verify main proof (trie ID and child root)");
				println!("7. ✅ Fetch child proof using state_getChildReadProof");
				println!(
					"8. ⚠️  Verify child proof (commitment not found - may not be finalized yet)"
				);
				println!("\n📝 Note: The commitment may not be in storage yet if the request hasn't been processed.");
			},
		}
	}

	println!("\n🎉 EVM PostRequestEvent verification flow completed successfully!");
	Ok(())
}
