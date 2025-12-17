use codec::{Decode, Encode};
use evm_state_machine::substrate_evm::{
	contract_info_key, fetch_child_root_from_main_proof, fetch_trie_id_from_main_proof,
	verify_child_trie_values, AccountInfo, AccountType,
};
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
		StateMachineId,
	},
	host::{IsmpHost, StateMachine},
	messaging::Keccak256,
	router::{IsmpRouter, PostResponse, Request, Response},
	Error,
};
use ismp_testsuite::mocks::Host as MockHost;
use polkadot_sdk::sp_runtime::testing::H256;
use primitive_types::U256;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sp_core::{storage::ChildInfo, H160};
use std::time::Duration;

fn get_rpc_url() -> String {
	std::env::var("SUBSTRATE_RPC_URL")
		.unwrap_or_else(|_| "https://asset-hub-westend-rpc.n.dwellir.com".to_string())
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

#[tokio::test]
async fn test_verify_revive_state_proof() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt::try_init();

	let contract_hex = "0x0008a66a96003b32c08fb8ee22616ef5f470c10e";
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
