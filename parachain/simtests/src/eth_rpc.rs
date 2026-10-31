#![cfg(test)]

use std::env;

use polkadot_sdk::*;
use primitive_types::H256;
use sc_consensus_manual_seal::CreatedBlock;
use serde_json::{json, Value};
use sp_core::{crypto::Ss58Codec, Bytes};
use sp_io::hashing::keccak_256;
use sp_keyring::sr25519::Keyring;
use substrate_eth_rpc::pallet_address;
use subxt::{
	dynamic::Value as DynValue,
	ext::subxt_rpcs::{client::RpcParams, rpc_params, RpcClient},
};
use subxt_utils::Hyperbridge;

const TRANSFER_TOPIC: &str = "0xf1e64c742f11e9d021f557d03df972f0fafc2745c6c961a489e71fa4dc8fd66f";

fn keccak_hex(bytes: &[u8]) -> String {
	format!("0x{}", hex::encode(keccak_256(bytes)))
}

fn address_of(pallet: &str) -> String {
	pallet_address(pallet).to_string().to_lowercase()
}

fn hex_to_u64(value: &Value) -> u64 {
	let text = value.as_str().expect("hex quantity");
	u64::from_str_radix(text.trim_start_matches("0x"), 16).expect("hex quantity")
}

async fn eth_rpc_error(rpc_client: &RpcClient, method: &str, params: RpcParams) -> String {
	let err = rpc_client
		.request::<Value>(method, params)
		.await
		.expect_err(&format!("{method} should fail"));
	format!("{err:?}")
}

#[tokio::test]
#[ignore]
async fn eth_rpc_facade_serves_substrate_events() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let url = &format!("ws://127.0.0.1:{}", port);
	let (client, rpc_client) = subxt_utils::client::ws_client::<Hyperbridge>(url, u32::MAX).await?;

	let chain_id: String = rpc_client.request("eth_chainId", rpc_params![]).await?;
	assert_eq!(chain_id, "0x3b9ad9a9");
	let net_version: String = rpc_client.request("net_version", rpc_params![]).await?;
	assert_eq!(net_version, "1000004009");

	let bob: [u8; 32] = Keyring::Bob.to_account_id().into();
	let transfer = subxt::dynamic::tx(
		"Balances",
		"transfer_keep_alive",
		vec![
			DynValue::unnamed_variant("Id", [DynValue::from_bytes(bob)]),
			DynValue::u128(1_000_000_000_000),
		],
	);
	let call_data = client.tx().call_data(&transfer)?;
	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), Keyring::Alice.to_account_id().to_ss58check()],
		)
		.await?;
	let _: H256 = rpc_client
		.request("author_submitExtrinsic", rpc_params![extrinsic.clone()])
		.await?;
	let transfer_hash = keccak_hex(&extrinsic.0);

	for _ in 0..2 {
		rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, true])
			.await?;
	}

	let head: H256 = rpc_client.request("chain_getFinalizedHead", rpc_params![]).await?;
	let header: Value = rpc_client.request("chain_getHeader", rpc_params![head]).await?;
	let signed_block: Value = rpc_client.request("chain_getBlock", rpc_params![head]).await?;
	let head_number = hex_to_u64(&header["number"]);

	let block_number: Value = rpc_client.request("eth_blockNumber", rpc_params![]).await?;
	assert_eq!(hex_to_u64(&block_number), head_number);

	let block: Value =
		rpc_client.request("eth_getBlockByNumber", rpc_params!["latest", false]).await?;
	assert_eq!(block["hash"], json!(format!("{head:?}")));
	assert_eq!(block["parentHash"], header["parentHash"]);
	assert_eq!(hex_to_u64(&block["number"]), head_number);
	let extrinsics = signed_block["block"]["extrinsics"]
		.as_array()
		.expect("extrinsics")
		.iter()
		.map(|ext| hex::decode(ext.as_str().expect("extrinsic hex").trim_start_matches("0x")))
		.collect::<Result<Vec<_>, _>>()?;
	assert!(!extrinsics.is_empty());
	let expected: Vec<Value> = extrinsics.iter().map(|ext| json!(keccak_hex(ext))).collect();
	assert_eq!(block["transactions"], Value::Array(expected));

	let balances = address_of("balances");
	let logs: Vec<Value> = rpc_client
		.request(
			"eth_getLogs",
			rpc_params![json!({ "fromBlock": "0x0", "toBlock": "latest", "address": balances })],
		)
		.await?;
	assert!(!logs.is_empty(), "balances logs");
	let log = logs
		.iter()
		.find(|log| log["transactionHash"] == json!(transfer_hash))
		.unwrap_or_else(|| panic!("no log for the transfer extrinsic in {logs:#?}"));
	assert_eq!(log["address"], json!(balances));
	assert_eq!(log["topics"][0], json!(TRANSFER_TOPIC));
	assert_eq!(log["removed"], json!(false));
	assert!(log["blockTimestamp"].is_string(), "blockTimestamp in {log:#?}");
	assert_eq!(hex_to_u64(&log["blockNumber"]), head_number - 1);
	let transfer_block: H256 =
		rpc_client.request("chain_getBlockHash", rpc_params![head_number - 1]).await?;
	assert_eq!(log["blockHash"], json!(format!("{transfer_block:?}")));

	let ismp_logs: Vec<Value> = rpc_client
		.request(
			"eth_getLogs",
			rpc_params![
				json!({ "fromBlock": "0x0", "toBlock": "latest", "address": address_of("ismp") })
			],
		)
		.await?;
	assert_eq!(ismp_logs, Vec::<Value>::new());

	let err = eth_rpc_error(
		&rpc_client,
		"eth_getLogs",
		rpc_params![json!({ "fromBlock": "0x0", "toBlock": "0x1f5" })],
	)
	.await;
	assert!(err.contains("this block range should work"), "{err}");

	let err = eth_rpc_error(
		&rpc_client,
		"eth_call",
		rpc_params![
			json!({ "to": "0x1111111111111111111111111111111111111111", "data": "0x" }),
			"latest"
		],
	)
	.await;
	assert!(err.contains("only the storage address is callable"), "{err}");
	assert!(err.contains("32602") || err.contains("InvalidParams"), "{err}");

	Ok(())
}
