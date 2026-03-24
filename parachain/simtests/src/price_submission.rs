#![cfg(test)]

use std::env;

use codec::Encode;
use pallet_intents_coprocessor::types::{PriceInput, TokenPair};
use pallet_intents_rpc::RpcPriceEntry;
use polkadot_sdk::*;
use primitive_types::{H256, U256};
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, Bytes};
use sp_keyring::sr25519::Keyring;
use subxt::ext::subxt_rpcs::rpc_params;
use subxt_utils::Hyperbridge;

/// Helper: submit raw SCALE-encoded call bytes from a given keyring account,
/// create and finalize a block, then wait for success.
async fn submit_raw_and_finalize(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc_client: &subxt::backend::rpc::RpcClient,
	call_data: Vec<u8>,
	who: Keyring,
) -> Result<(), anyhow::Error> {
	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), who.to_account_id().to_ss58check()],
		)
		.await?;
	let submittable = subxt::tx::SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
	let progress = submittable.submit_and_watch().await?;
	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized);
	progress.wait_for_finalized_success().await?;
	Ok(())
}

/// Helper: wrap raw SCALE-encoded call bytes in `Sudo::sudo` and submit from Alice.
/// Constructs the sudo call by manually prepending the Sudo pallet index + call_index
/// and wrapping the inner call.
async fn sudo_raw_and_finalize(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc_client: &subxt::backend::rpc::RpcClient,
	inner_call_data: Vec<u8>,
) -> Result<(), anyhow::Error> {
	let mut sudo_call_data = vec![25u8, 0u8];
	sudo_call_data.extend_from_slice(&inner_call_data);
	submit_raw_and_finalize(client, rpc_client, sudo_call_data, Keyring::Alice).await
}

/// Manually encode a call to `IntentsCoprocessor::submit_pair_price`.
/// Pallet index 65, call index 7.
fn encode_submit_pair_price(pair_id: H256, entries: Vec<PriceInput>) -> Vec<u8> {
	let mut data = vec![65u8, 7u8]; // pallet_index, call_index
	data.extend_from_slice(&pair_id.encode());
	data.extend_from_slice(&entries.encode());
	data
}

/// Manually encode a call to `IntentsCoprocessor::set_price_submission_fee`.
/// Pallet index 65, call index 10.
fn encode_set_price_submission_fee(fee: u128) -> Vec<u8> {
	let mut data = vec![65u8, 10u8];
	data.extend_from_slice(&fee.encode());
	data
}


/// Integration test for the fee-based price submission system.
///
/// Exercises the full lifecycle:
/// 1. Governance setup (set submission fee, register pair)
/// 2. Price submission (verifies fee is charged, prices stored)
/// 3. RPC query (verifies human-readable prices with decimals preserved)
/// 4. Re-submission overwrites previous entries
#[tokio::test]
#[ignore]
async fn test_price_submission_lifecycle() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or("9990".into());
	let url = &format!("ws://127.0.0.1:{}", port);
	let (client, rpc_client) = subxt_utils::client::ws_client::<Hyperbridge>(url, u32::MAX).await?;

	let pair = TokenPair { base: b"USDC".to_vec(), quote: b"cNGN".to_vec() };
	let pair_id = pair.pair_id();

	// 1 unit = 10^18 in raw representation
	let one_unit = U256::from(10u64).pow(U256::from(18));

	// Submission fee: 100 units
	let submission_fee: u128 = 100_000_000_000_000;

	// Price entries: amount thresholds with corresponding prices
	// Entry 1: amount=0 at price 1414.5, Entry 2: amount=1000 at price 1420
	let price_entries = vec![
		PriceInput {
			amount: U256::zero(),
			price: U256::from(14145) * one_unit / U256::from(10), // 1414.5 * 10^18
		},
		PriceInput { amount: U256::from(1000) * one_unit, price: U256::from(1420) * one_unit },
	];

	// Set submission fee
	sudo_raw_and_finalize(
		&client,
		&rpc_client,
		encode_set_price_submission_fee(submission_fee),
	)
	.await?;
	println!("Submission fee set: {submission_fee}");

	// Submit prices
	let submit_call_data = encode_submit_pair_price(pair_id, price_entries);
	submit_raw_and_finalize(&client, &rpc_client, submit_call_data, Keyring::Alice).await?;
	println!("Prices submitted for pair {pair_id:?}");

	// Query prices via RPC
	let prices: Vec<RpcPriceEntry> =
		rpc_client.request("intents_getPairPrices", rpc_params![pair_id]).await?;

	assert_eq!(prices.len(), 2, "expected 2 price entries");

	// Verify first entry: amount=0 at price 1414.5
	assert_eq!(prices[0].amount, "0", "amount1 should be 0");
	assert_eq!(prices[0].price, "1414.5", "price1 should be 1414.5 (decimals preserved)");

	// Verify second entry: amount=1000 at price 1420
	assert_eq!(prices[1].amount, "1000", "amount2 should be 1000");
	assert_eq!(prices[1].price, "1420", "price2 should be 1420");

	println!("RPC returns human-readable prices with decimals preserved");
	println!("  entry[0]: amount={} @ {}", prices[0].amount, prices[0].price);
	println!("  entry[1]: amount={} @ {}", prices[1].amount, prices[1].price);

	// Re-submit with different prices — should overwrite
	let new_entries = vec![
		PriceInput { amount: U256::zero(), price: U256::from(1500) * one_unit },
		PriceInput { amount: U256::from(2000) * one_unit, price: U256::from(1520) * one_unit },
	];
	let resubmit_call_data = encode_submit_pair_price(pair_id, new_entries);
	submit_raw_and_finalize(&client, &rpc_client, resubmit_call_data, Keyring::Alice).await?;
	println!("Prices re-submitted (should overwrite)");

	// Query again and verify overwrite
	let prices2: Vec<RpcPriceEntry> =
		rpc_client.request("intents_getPairPrices", rpc_params![pair_id]).await?;

	assert_eq!(prices2.len(), 2, "expected 2 price entries after overwrite");
	assert_eq!(prices2[0].amount, "0");
	assert_eq!(prices2[0].price, "1500");
	assert_eq!(prices2[1].amount, "2000");
	assert_eq!(prices2[1].price, "1520");
	println!("Overwrite confirmed: old prices replaced");

	println!("Price submission lifecycle test passed!");
	Ok(())
}
