#![cfg(test)]

use std::env;

use pallet_intents_rpc::RpcBidInfo;
use polkadot_sdk::*;
use primitive_types::H256;
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, Bytes};
use sp_keyring::sr25519::Keyring;
use subxt::ext::subxt_rpcs::rpc_params;
use subxt_utils::Hyperbridge;

/// Helper to build and author a `place_bid` extrinsic for a given account.
async fn author_place_bid(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc_client: &subxt::backend::rpc::RpcClient,
	commitment: &H256,
	user_op: &[u8],
	who: Keyring,
) -> Result<Bytes, anyhow::Error> {
	let call = subxt::dynamic::tx(
		"IntentsCoprocessor",
		"place_bid",
		vec![
			subxt::dynamic::Value::from_bytes(commitment.as_bytes()),
			subxt::dynamic::Value::from_bytes(user_op),
		],
	);
	let call_data = client.tx().call_data(&call)?;

	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), who.to_account_id().to_ss58check()],
		)
		.await?;
	Ok(extrinsic)
}

/// Verifies that `intents_getBidsForOrder` returns bids from both the
/// in-memory cache (mempool) and on-chain/offchain storage (finalized bids),
/// and that the results are correctly merged and deduplicated.
#[tokio::test]
#[ignore]
async fn test_bid_discovery_via_rpc() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let url = &format!("ws://127.0.0.1:{}", port);
	let (client, rpc_client) = subxt_utils::client::ws_client::<Hyperbridge>(url, u32::MAX).await?;

	let commitment = H256::random();
	let alice_op: Vec<u8> = vec![0xAA, 0xBB, 0xCC];
	let bob_op: Vec<u8> = vec![0xDD, 0xEE, 0xFF];

	// Push bids from Alice and Bob into the mempool
	let alice_ext =
		author_place_bid(&client, &rpc_client, &commitment, &alice_op, Keyring::Alice).await?;
	let _: H256 = rpc_client.request("author_submitExtrinsic", rpc_params![alice_ext]).await?;

	let bob_ext =
		author_place_bid(&client, &rpc_client, &commitment, &bob_op, Keyring::Bob).await?;
	let _: H256 = rpc_client.request("author_submitExtrinsic", rpc_params![bob_ext]).await?;

	// Allow the tx-pool watcher to process import notifications.
	tokio::time::sleep(std::time::Duration::from_millis(500)).await;

	let bids: Vec<RpcBidInfo> =
		rpc_client.request("intents_getBidsForOrder", rpc_params![commitment]).await?;
	assert_eq!(bids.len(), 2, "both bids should be discoverable from the mempool");

	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);

	// Alice and Bob bids should now be in both the in-memory cache and offchain storage.
	let bids: Vec<RpcBidInfo> =
		rpc_client.request("intents_getBidsForOrder", rpc_params![commitment]).await?;
	assert_eq!(bids.len(), 2, "finalized bids should persist from offchain storage");

	// Submit a new mempool-only bid from Charlie
	let charlie_op: Vec<u8> = vec![0x11, 0x22, 0x33];
	let charlie_ext =
		author_place_bid(&client, &rpc_client, &commitment, &charlie_op, Keyring::Charlie).await?;
	let _: H256 = rpc_client.request("author_submitExtrinsic", rpc_params![charlie_ext]).await?;

	tokio::time::sleep(std::time::Duration::from_millis(500)).await;

	let bids: Vec<RpcBidInfo> =
		rpc_client.request("intents_getBidsForOrder", rpc_params![commitment]).await?;
	assert_eq!(
		bids.len(),
		3,
		"RPC should return offchain bids (Alice, Bob) and mempool bid (Charlie)"
	);

	// Verify all three user_ops are present.
	let user_ops: Vec<&Vec<u8>> = bids.iter().map(|b| &b.user_op).collect();
	assert!(user_ops.contains(&&alice_op), "Alice's bid missing");
	assert!(user_ops.contains(&&bob_op), "Bob's bid missing");
	assert!(user_ops.contains(&&charlie_op), "Charlie's mempool-only bid missing");

	// All bids should reference the same commitment.
	assert!(bids.iter().all(|b| b.commitment == commitment));

	Ok(())
}
