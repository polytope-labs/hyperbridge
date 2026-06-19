//! Simnode integration tests for phantom order lifecycle in
//! `pallet-intents-coprocessor`.
//!
//! Each test exercises a distinct slice of the phantom order flow:
//!
//! - `test_register_phantom_order_stores_and_emits_event` — registration populates
//!   `CurrentPhantomOrder` and emits `PhantomOrderRegistered`.
//! - `test_register_phantom_order_replaces_previous` — a second registration atomically replaces
//!   the first.
//! - `test_multiple_fillers_can_bid_on_phantom_order` — Alice and Bob each place one bid; both are
//!   discoverable via `intents_getBidsForOrder`.
//! - `test_duplicate_phantom_bid_rejected` — a filler that already holds a bid for the current
//!   phantom order is rejected with `DuplicatePhantomBid`.
//! - `test_bid_rejected_after_phantom_window_closes` — a bid arriving after the governance-set
//!   window has passed is rejected with `PhantomOrderBidWindowClosed`.
//! - `test_set_phantom_bid_window_via_governance` — sudo can update `PhantomBidWindow`; the new
//!   value is persisted to storage.
//! - `test_phantom_order_full_flow_register_bid_rpc` — end-to-end: register, three fillers bid, all
//!   three appear in the RPC response with the correct commitment.
//!
//! All tests are `#[ignore]`d. Run them with a simnode listening on the
//! default port (`PORT=9990`) via:
//!
//! ```text
//! # Start the simnode WITHOUT --instant (instant seal fires before engine_createBlock,
//! # making event reads hit an empty block).
//! ./target/debug/hyperbridge simnode --chain gargantua-1000 --rpc-port 9990 --tmp
//!
//! cargo test -p simtests phantom_orders -- --ignored --test-threads=1
//! ```

#![cfg(test)]

use std::env;

use codec::Decode;
use pallet_intents_rpc::RpcBidInfo;
use polkadot_sdk::*;
use primitive_types::H256;
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, Bytes};
use sp_keyring::sr25519::Keyring;
use subxt::{ext::subxt_rpcs::rpc_params, tx::SubmittableTransaction};
use subxt_utils::Hyperbridge;

const SIMNODE_PORT_DEFAULT: &str = "9990";

// ---------------------------------------------------------------------------
// Storage key helpers
// ---------------------------------------------------------------------------

fn current_phantom_order_key() -> Vec<u8> {
	[
		sp_core::twox_128(b"IntentsCoprocessor").to_vec(),
		sp_core::twox_128(b"CurrentPhantomOrder").to_vec(),
	]
	.concat()
}

fn phantom_bid_window_key() -> Vec<u8> {
	[
		sp_core::twox_128(b"IntentsCoprocessor").to_vec(),
		sp_core::twox_128(b"PhantomBidWindow").to_vec(),
	]
	.concat()
}

// ---------------------------------------------------------------------------
// Common helpers
// ---------------------------------------------------------------------------

/// Seal an empty block and finalize it. Returns the new block hash.
async fn create_and_finalize_block(
	rpc: &subxt::backend::rpc::RpcClient,
) -> Result<H256, anyhow::Error> {
	let block: CreatedBlock<H256> =
		rpc.request("engine_createBlock", rpc_params![true, false]).await?;
	let finalized: bool = rpc.request("engine_finalizeBlock", rpc_params![block.hash]).await?;
	assert!(finalized, "block must finalize");
	Ok(block.hash)
}

/// Author a pre-encoded call signed by `signer` via `simnode_authorExtrinsic`,
/// seal a block, and wait for finalized success. Returns the block hash.
async fn author_and_seal(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc: &subxt::backend::rpc::RpcClient,
	call_data: Vec<u8>,
	signer: Keyring,
) -> Result<H256, anyhow::Error> {
	let extrinsic: Bytes = rpc
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), signer.to_account_id().to_ss58check()],
		)
		.await?;
	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
	let progress = submittable.submit_and_watch().await?;
	let block: CreatedBlock<H256> =
		rpc.request("engine_createBlock", rpc_params![true, false]).await?;
	let finalized: bool = rpc.request("engine_finalizeBlock", rpc_params![block.hash]).await?;
	assert!(finalized, "block must finalize");
	progress.wait_for_finalized_success().await?;
	Ok(block.hash)
}

/// Wrap `inner` in `Sudo::sudo`, author as Alice, seal, and await success.
async fn sudo_and_seal(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc: &subxt::backend::rpc::RpcClient,
	inner: subxt::dynamic::Value,
) -> Result<H256, anyhow::Error> {
	let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![inner]);
	let call_data = client.tx().call_data(&sudo_call)?;
	author_and_seal(client, rpc, call_data, Keyring::Alice).await
}

/// Author a `register_phantom_order` extrinsic for `commitment` / `chain`,
/// seal the block, and await finalized success.
async fn register_phantom_order(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc: &subxt::backend::rpc::RpcClient,
	commitment: H256,
	chain: &[u8],
	signer: Keyring,
) -> Result<H256, anyhow::Error> {
	let call = subxt::dynamic::tx(
		"IntentsCoprocessor",
		"register_phantom_order",
		vec![
			subxt::dynamic::Value::from_bytes(commitment.as_bytes()),
			subxt::dynamic::Value::from_bytes(chain),
		],
	);
	let call_data = client.tx().call_data(&call)?;
	author_and_seal(client, rpc, call_data, signer).await
}

/// Build a signed `place_bid` extrinsic for `commitment` / `user_op` but do
/// NOT submit or seal it — returns the raw extrinsic bytes so the caller can
/// batch multiple bids into a single block.
async fn author_place_bid(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc: &subxt::backend::rpc::RpcClient,
	commitment: H256,
	user_op: &[u8],
	signer: Keyring,
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
	let extrinsic: Bytes = rpc
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), signer.to_account_id().to_ss58check()],
		)
		.await?;
	Ok(extrinsic)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Registering a phantom order must populate `CurrentPhantomOrder` with the
/// supplied commitment and emit a `PhantomOrderRegistered` event.
#[tokio::test]
#[ignore]
async fn test_register_phantom_order_stores_and_emits_event() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	let commitment = H256::random();
	let chain = b"EVM-8453";

	let block_hash =
		register_phantom_order(&client, &rpc, commitment, chain, Keyring::Alice).await?;

	// CurrentPhantomOrder storage must be populated.
	let raw = client
		.storage()
		.at(block_hash)
		.fetch_raw(current_phantom_order_key())
		.await?
		.expect("CurrentPhantomOrder must be set after register_phantom_order");

	// First 32 bytes must equal the registered commitment.
	assert_eq!(
		&raw[..32],
		commitment.as_bytes(),
		"stored commitment must match the registered one",
	);

	// The chain bytes (UTF-8) must follow the block number.
	// Layout: [H256 (32)] [u32 LE (4)] [SCALE compact len (1)] [chain bytes]
	let chain_len = (raw[36] >> 2) as usize;
	let stored_chain = &raw[37..37 + chain_len];
	assert_eq!(stored_chain, chain, "stored chain must match the registered chain");

	// PhantomOrderRegistered event must have been emitted in that block.
	let events = client.events().at(block_hash).await?;
	let emitted = events.iter().any(|ev| {
		ev.ok()
			.map(|e| {
				e.pallet_name() == "IntentsCoprocessor" &&
					e.variant_name() == "PhantomOrderRegistered"
			})
			.unwrap_or(false)
	});
	assert!(emitted, "PhantomOrderRegistered event must be emitted");

	Ok(())
}

/// A second `register_phantom_order` call must atomically replace the first;
/// only the newest commitment must remain in `CurrentPhantomOrder`.
#[tokio::test]
#[ignore]
async fn test_register_phantom_order_replaces_previous() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	let first = H256::random();
	let second = H256::random();
	let chain = b"EVM-8453";

	register_phantom_order(&client, &rpc, first, chain, Keyring::Alice).await?;
	register_phantom_order(&client, &rpc, second, chain, Keyring::Alice).await?;

	let raw = client
		.storage()
		.at_latest()
		.await?
		.fetch_raw(current_phantom_order_key())
		.await?
		.expect("CurrentPhantomOrder must be set");

	assert_eq!(&raw[..32], second.as_bytes(), "second registration must overwrite the first",);

	Ok(())
}

/// Multiple distinct fillers must each be able to place one bid on the active
/// phantom order within the bid window, and all bids must be visible via the
/// `intents_getBidsForOrder` RPC.
#[tokio::test]
#[ignore]
async fn test_multiple_fillers_can_bid_on_phantom_order() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	let commitment = H256::random();
	let chain = b"EVM-8453";

	register_phantom_order(&client, &rpc, commitment, chain, Keyring::Alice).await?;

	// Queue Alice and Bob's bids before sealing so both land in the same block.
	let alice_op = vec![0xAA, 0xBB, 0xCC];
	let bob_op = vec![0xDD, 0xEE, 0xFF];

	let alice_ext = author_place_bid(&client, &rpc, commitment, &alice_op, Keyring::Alice).await?;
	let alice_progress = SubmittableTransaction::from_bytes(client.clone(), alice_ext.0)
		.submit_and_watch()
		.await?;

	let bob_ext = author_place_bid(&client, &rpc, commitment, &bob_op, Keyring::Bob).await?;
	let bob_progress = SubmittableTransaction::from_bytes(client.clone(), bob_ext.0)
		.submit_and_watch()
		.await?;

	create_and_finalize_block(&rpc).await?;
	alice_progress.wait_for_finalized_success().await?;
	bob_progress.wait_for_finalized_success().await?;

	let bids: Vec<RpcBidInfo> =
		rpc.request("intents_getBidsForOrder", rpc_params![commitment]).await?;
	assert_eq!(bids.len(), 2, "both phantom bids must appear in the RPC response");
	assert!(
		bids.iter().all(|b| b.commitment == commitment),
		"all returned bids must reference the registered commitment",
	);

	Ok(())
}

/// A filler that already holds a bid for the active phantom order must be
/// rejected with `DuplicatePhantomBid` on a second attempt.
#[tokio::test]
#[ignore]
async fn test_duplicate_phantom_bid_rejected() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	let commitment = H256::random();
	let chain = b"EVM-8453";

	register_phantom_order(&client, &rpc, commitment, chain, Keyring::Alice).await?;

	// First bid from Alice — must succeed.
	let first_call = subxt::dynamic::tx(
		"IntentsCoprocessor",
		"place_bid",
		vec![
			subxt::dynamic::Value::from_bytes(commitment.as_bytes()),
			subxt::dynamic::Value::from_bytes(&[0xAA, 0xBB, 0xCC]),
		],
	);
	author_and_seal(&client, &rpc, client.tx().call_data(&first_call)?, Keyring::Alice).await?;

	// Second bid from Alice on the same commitment — must be rejected.
	let dup_call_data = client.tx().call_data(&subxt::dynamic::tx(
		"IntentsCoprocessor",
		"place_bid",
		vec![
			subxt::dynamic::Value::from_bytes(commitment.as_bytes()),
			subxt::dynamic::Value::from_bytes(&[0x11, 0x22, 0x33]),
		],
	))?;
	let dup_ext: Bytes = rpc
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(dup_call_data), Keyring::Alice.to_account_id().to_ss58check()],
		)
		.await?;
	let dup_progress = SubmittableTransaction::from_bytes(client.clone(), dup_ext.0)
		.submit_and_watch()
		.await?;
	create_and_finalize_block(&rpc).await?;
	let result = dup_progress.wait_for_finalized_success().await;
	assert!(
		result.is_err(),
		"second bid from same filler must be rejected with DuplicatePhantomBid",
	);

	Ok(())
}

/// A bid placed after the governance-set bid window has expired must be
/// rejected with `PhantomOrderBidWindowClosed`.
///
/// Flow:
///   Block N   — set bid window to 1 block (via sudo)
///   Block N+1 — register phantom order (created_at_block = N+1)
///   Block N+2 — empty block (window still open: N+2 <= N+1+1)
///   Block N+3 — place bid (window closed: N+3 > N+1+1) → rejected
#[tokio::test]
#[ignore]
async fn test_bid_rejected_after_phantom_window_closes() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	let commitment = H256::random();
	let chain = b"EVM-8453";

	// Set the bid window to 1 block via governance (sudo).
	let window_call = subxt::dynamic::tx(
		"IntentsCoprocessor",
		"set_phantom_bid_window",
		vec![subxt::dynamic::Value::u128(1)],
	);
	sudo_and_seal(&client, &rpc, window_call.into_value()).await?;

	// Register the phantom order (block N+1).
	register_phantom_order(&client, &rpc, commitment, chain, Keyring::Alice).await?;

	// Advance one empty block to land at N+2 (window still open here).
	create_and_finalize_block(&rpc).await?;

	// Author a bid but do NOT seal yet.
	let bid_call_data = client.tx().call_data(&subxt::dynamic::tx(
		"IntentsCoprocessor",
		"place_bid",
		vec![
			subxt::dynamic::Value::from_bytes(commitment.as_bytes()),
			subxt::dynamic::Value::from_bytes(&[0xAA, 0xBB]),
		],
	))?;
	let bid_ext: Bytes = rpc
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(bid_call_data), Keyring::Bob.to_account_id().to_ss58check()],
		)
		.await?;
	let bid_progress = SubmittableTransaction::from_bytes(client.clone(), bid_ext.0)
		.submit_and_watch()
		.await?;

	// Seal block N+3 — the bid executes here, outside the window.
	create_and_finalize_block(&rpc).await?;
	let result = bid_progress.wait_for_finalized_success().await;
	assert!(
		result.is_err(),
		"bid placed after window closure must be rejected with PhantomOrderBidWindowClosed",
	);

	// Reset the bid window to a generous value so subsequent tests are not affected
	// by the 1-block window set at the start of this test.
	let reset_call = subxt::dynamic::tx(
		"IntentsCoprocessor",
		"set_phantom_bid_window",
		vec![subxt::dynamic::Value::u128(100)],
	);
	sudo_and_seal(&client, &rpc, reset_call.into_value()).await?;

	Ok(())
}

/// Governance (sudo) can update `PhantomBidWindow`; the new value must be
/// persisted to storage and reflected on the next governance query.
#[tokio::test]
#[ignore]
async fn test_set_phantom_bid_window_via_governance() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	let new_window: u32 = 42;

	let window_call = subxt::dynamic::tx(
		"IntentsCoprocessor",
		"set_phantom_bid_window",
		vec![subxt::dynamic::Value::u128(new_window as u128)],
	);
	let block_hash = sudo_and_seal(&client, &rpc, window_call.into_value()).await?;

	let raw = client
		.storage()
		.at(block_hash)
		.fetch_raw(phantom_bid_window_key())
		.await?
		.expect("PhantomBidWindow must be set after governance call");

	let stored = u32::decode(&mut &raw[..]).expect("PhantomBidWindow must decode as u32");
	assert_eq!(stored, new_window, "PhantomBidWindow must equal the governance-set value");

	Ok(())
}

/// End-to-end phantom order flow: register an order, have three fillers bid
/// in a single block, and verify all three bids are returned by
/// `intents_getBidsForOrder` with the correct commitment.
#[tokio::test]
#[ignore]
async fn test_phantom_order_full_flow_register_bid_rpc() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	let commitment = H256::random();
	let chain = b"EVM-8453";

	// Step 1: register the phantom order.
	register_phantom_order(&client, &rpc, commitment, chain, Keyring::Alice).await?;

	// Step 2: three fillers each author a bid.
	let fillers: &[(&[u8], Keyring)] =
		&[(&[0xAA], Keyring::Alice), (&[0xBB], Keyring::Bob), (&[0xCC], Keyring::Charlie)];
	let mut progresses = Vec::new();
	for (op, who) in fillers {
		let ext = author_place_bid(&client, &rpc, commitment, op, *who).await?;
		let p = SubmittableTransaction::from_bytes(client.clone(), ext.0)
			.submit_and_watch()
			.await?;
		progresses.push(p);
	}

	// Step 3: seal one block containing all three bids.
	create_and_finalize_block(&rpc).await?;
	for p in progresses {
		p.wait_for_finalized_success().await?;
	}

	// Step 4: verify all three are discoverable via RPC.
	let bids: Vec<RpcBidInfo> =
		rpc.request("intents_getBidsForOrder", rpc_params![commitment]).await?;
	assert_eq!(bids.len(), 3, "all three phantom bids must be discoverable via RPC");
	assert!(
		bids.iter().all(|b| b.commitment == commitment),
		"every returned bid must carry the registered commitment",
	);

	let user_ops: Vec<&Vec<u8>> = bids.iter().map(|b| &b.user_op).collect();
	assert!(user_ops.contains(&&vec![0xAA]), "Alice's user_op must be present");
	assert!(user_ops.contains(&&vec![0xBB]), "Bob's user_op must be present");
	assert!(user_ops.contains(&&vec![0xCC]), "Charlie's user_op must be present");

	Ok(())
}
