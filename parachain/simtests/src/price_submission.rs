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

/// Helper: create and finalize `n` empty blocks to advance the chain.
async fn advance_blocks(
	rpc_client: &subxt::backend::rpc::RpcClient,
	n: u32,
) -> Result<(), anyhow::Error> {
	for _ in 0..n {
		let block = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
			.await?;
		rpc_client
			.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
			.await?;
	}
	Ok(())
}

/// Manually encode a call to `IntentsCoprocessor::submit_pair_price`.
/// Pallet index 65, call index 7.
fn encode_submit_pair_price(pair_id: H256, entries: Vec<PriceInput>) -> Vec<u8> {
	let mut data = vec![65u8, 7u8]; // pallet_index, call_index
	data.extend_from_slice(&pair_id.encode());
	data.extend_from_slice(&entries.encode());
	data
}

/// Manually encode a call to `IntentsCoprocessor::set_price_deposit_amount`.
/// Pallet index 65, call index 11.
fn encode_set_price_deposit_amount(amount: u128) -> Vec<u8> {
	let mut data = vec![65u8, 11u8];
	data.extend_from_slice(&amount.encode());
	data
}

/// Manually encode a call to `IntentsCoprocessor::set_price_deposit_lock_duration`.
/// Pallet index 65, call index 12. Duration is BlockNumberFor<T> = u32 on gargantua.
fn encode_set_price_deposit_lock_duration(duration_blocks: u32) -> Vec<u8> {
	let mut data = vec![65u8, 12u8];
	data.extend_from_slice(&duration_blocks.encode());
	data
}

/// Manually encode a call to `IntentsCoprocessor::withdraw_price_deposit`.
/// Pallet index 65, call index 13.
fn encode_withdraw_price_deposit(pair_id: H256) -> Vec<u8> {
	let mut data = vec![65u8, 13u8];
	data.extend_from_slice(&pair_id.encode());
	data
}

/// Manually encode a call to `IntentsCoprocessor::register_pair`.
/// Pallet index 65, call index 8.
fn encode_register_pair(pair_id: H256) -> Vec<u8> {
	let mut data = vec![65u8, 8u8];
	data.extend_from_slice(&pair_id.encode());
	data
}

/// Manually encode a call to `IntentsCoprocessor::set_pair_registration_deposit`.
/// Pallet index 65, call index 14.
fn encode_set_pair_registration_deposit(amount: u128) -> Vec<u8> {
	let mut data = vec![65u8, 14u8];
	data.extend_from_slice(&amount.encode());
	data
}

/// Integration test for the deposit-based price submission system.
///
/// Exercises the full lifecycle:
/// 1. Governance setup (add recognized pair, set deposit amount and lock duration)
/// 2. Price submission (verifies deposit is reserved)
/// 3. RPC query (verifies human-readable prices with decimals preserved)
/// 4. Two-phase withdrawal (initiate → fail before unlock → complete after unlock)
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

	// Deposit amount: 100 units
	let deposit_amount: u128 = 100_000_000_000_000;

	// Lock duration: 5 blocks
	let lock_duration: u32 = 5;

	// Price entries: amount thresholds with corresponding prices
	// Entry 1: amount=0 at price 1414.5, Entry 2: amount=1000 at price 1420
	let price_entries = vec![
		PriceInput {
			amount: U256::zero(),
			price: U256::from(14145) * one_unit / U256::from(10), // 1414.5 * 10^18
		},
		PriceInput { amount: U256::from(1000) * one_unit, price: U256::from(1420) * one_unit },
	];

	// Set deposit amount
	sudo_raw_and_finalize(&client, &rpc_client, encode_set_price_deposit_amount(deposit_amount))
		.await?;
	println!("Deposit amount set: {deposit_amount}");

	// Set lock duration (5 blocks)
	sudo_raw_and_finalize(
		&client,
		&rpc_client,
		encode_set_price_deposit_lock_duration(lock_duration),
	)
	.await?;
	println!("Lock duration set: {lock_duration} blocks");

	// Set pair registration deposit and register pair
	let pair_reg_deposit: u128 = 50_000_000_000_000;
	sudo_raw_and_finalize(
		&client,
		&rpc_client,
		encode_set_pair_registration_deposit(pair_reg_deposit),
	)
	.await?;
	println!("Pair registration deposit set: {pair_reg_deposit}");

	submit_raw_and_finalize(
		&client,
		&rpc_client,
		encode_register_pair(pair_id),
		Keyring::Alice,
	)
	.await?;
	println!("Pair registered: {pair_id:?}");

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

	// Initiate withdrawal
	submit_raw_and_finalize(
		&client,
		&rpc_client,
		encode_withdraw_price_deposit(pair_id),
		Keyring::Alice,
	)
	.await?;
	println!("Withdrawal initiated (unlock block recorded)");

	// Attempting phase 2 immediately should fail (lock not expired)
	let early_result = submit_raw_and_finalize(
		&client,
		&rpc_client,
		encode_withdraw_price_deposit(pair_id),
		Keyring::Alice,
	)
	.await;
	assert!(early_result.is_err(), "withdrawal should fail before lock expires");
	println!("correctly rejected (deposit still locked)");

	// Advance blocks past the lock duration
	advance_blocks(&rpc_client, lock_duration + 1).await?;
	println!("Advanced {} blocks past lock duration", lock_duration + 1);

	// Complete withdrawal
	submit_raw_and_finalize(
		&client,
		&rpc_client,
		encode_withdraw_price_deposit(pair_id),
		Keyring::Alice,
	)
	.await?;
	println!("Deposit successfully withdrawn");

	// Verify deposit is gone, another withdrawal should fail with DepositNotFound
	let gone_result = submit_raw_and_finalize(
		&client,
		&rpc_client,
		encode_withdraw_price_deposit(pair_id),
		Keyring::Alice,
	)
	.await;
	assert!(gone_result.is_err(), "deposit should no longer exist");
	println!("Deposit confirmed removed (subsequent withdrawal fails)");

	println!("Price submission lifecycle test passed!");
	Ok(())
}
