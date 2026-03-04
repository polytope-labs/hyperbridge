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

/// Verifies that `intents_getBidsForOrder` returns bids discovered from the
/// transaction pool before they are included in a block, and that they persist
/// after finalization.
#[tokio::test]
#[ignore]
async fn test_bid_discovery_via_rpc() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let url = &format!("ws://127.0.0.1:{}", port);
	let (client, rpc_client) = subxt_utils::client::ws_client::<Hyperbridge>(url, u32::MAX).await?;

	let commitment = H256::random();
	let user_op: Vec<u8> = vec![0xde, 0xad, 0xbe, 0xef];

	let place_bid_call = subxt::dynamic::tx(
		"IntentsCoprocessor",
		"place_bid",
		vec![
			subxt::dynamic::Value::from_bytes(commitment.as_bytes()),
			subxt::dynamic::Value::from_bytes(&user_op),
		],
	);
	let call_data = client.tx().call_data(&place_bid_call)?;

	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), Keyring::Alice.to_account_id().to_ss58check()],
		)
		.await?;

	let _: H256 = rpc_client
		.request("author_submitExtrinsic", rpc_params![extrinsic])
		.await?;

	// Allow the tx-pool watcher to process the import notification.
	tokio::time::sleep(std::time::Duration::from_millis(500)).await;

	let bids: Vec<RpcBidInfo> = rpc_client
		.request("intents_getBidsForOrder", rpc_params![commitment])
		.await?;
	assert!(!bids.is_empty(), "bid should be discoverable from the mempool before block inclusion");
	assert_eq!(bids[0].commitment, commitment);
	assert!(!bids[0].confirmed);

	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);

	let bids: Vec<RpcBidInfo> = rpc_client
		.request("intents_getBidsForOrder", rpc_params![commitment])
		.await?;
	assert!(!bids.is_empty(), "bid should persist after block finalization");

	Ok(())
}
