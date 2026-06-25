//! Simnode integration tests for phantom order lifecycle in
//! `pallet-intents-coprocessor`.
//!
//! Tests exercise the full phantom order flow driven by `on_initialize`:
//!
//! - `test_set_phantom_order_config_stores_and_emits_event` — governance config triggers the hook
//!   which populates `CurrentPhantomOrder` and emits the event.
//! - `test_phantom_order_replaces_previous_on_interval` — with an interval of 1 block, consecutive
//!   empty blocks produce distinct commitments.
//! - `test_multiple_fillers_can_bid_on_phantom_order` — Alice and Bob each place one bid; both are
//!   discoverable via `intents_getBidsForOrder`.
//! - `test_duplicate_phantom_bid_rejected` — a filler that already holds a bid for the current
//!   phantom order is rejected with `DuplicatePhantomBid`.
//! - `test_bid_rejected_after_phantom_window_closes` — a bid arriving after the governance-set
//!   window has passed is rejected with `PhantomOrderBidWindowClosed`.
//! - `test_set_phantom_bid_window_via_governance` — sudo can update `PhantomBidWindow`; the new
//!   value is persisted to storage.
//! - `test_phantom_order_full_flow_config_bid_rpc` — end-to-end: configure, three fillers bid, all
//!   three appear in the RPC response with the correct commitment.
//!
//! Run with a simnode on the default port:
//!
//! ```text
//! cargo test -p simtests phantom_orders -- --ignored --test-threads=1
//! ```

#![cfg(test)]

use std::env;

use codec::{Decode, Encode};
use ismp::{consensus::StateMachineId, host::StateMachine};
use pallet_intents_rpc::RpcBidInfo;
use polkadot_sdk::*;
use primitive_types::H256;
use sc_consensus_manual_seal::CreatedBlock;
use sp_core::{crypto::Ss58Codec, Bytes};
use sp_keyring::sr25519::Keyring;
use subxt::{
	dynamic::Value,
	ext::{scale_value::Composite, subxt_rpcs::rpc_params},
	tx::SubmittableTransaction,
};
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

// Storage key for `Ismp::LatestStateMachineHeight[StateMachineId { Evm(chain_id), ETH0 }]`.
// The on_initialize hook reads this map to set the phantom order deadline; a bare simnode has no
// confirmed external heights, so the tests seed one here.
fn latest_state_machine_height_key(chain_id: u32) -> Vec<u8> {
	let id = StateMachineId { state_id: StateMachine::Evm(chain_id), consensus_state_id: *b"ETH0" };
	let encoded = id.encode();
	[
		sp_core::twox_128(b"Ismp").to_vec(),
		sp_core::twox_128(b"LatestStateMachineHeight").to_vec(),
		sp_core::hashing::blake2_128(&encoded).to_vec(),
		encoded,
	]
	.concat()
}

/// Seed a confirmed height for the destination chain via sudo `System::set_storage`, so the
/// on_initialize hook has a deadline to use and proceeds with phantom order generation.
async fn seed_state_machine_height(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc: &subxt::backend::rpc::RpcClient,
	chain_id: u32,
	height: u64,
) -> Result<H256, anyhow::Error> {
	let item = Value::unnamed_composite(vec![
		Value::from_bytes(latest_state_machine_height_key(chain_id)),
		Value::from_bytes(height.encode()),
	]);
	let call =
		subxt::dynamic::tx("System", "set_storage", vec![Value::unnamed_composite(vec![item])]);
	sudo_and_seal(client, rpc, call.into_value()).await
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

/// Build a `PhantomOrderConfiguration` dynamic value for EVM chain `chain_id`
/// with one standard token pair and the given block interval.
fn phantom_config_value(chain_id: u64, interval_blocks: u32) -> Value {
	let pair = Value::named_composite(vec![
		("token_a", Value::from_bytes([1u8; 20])),
		("token_b", Value::from_bytes([2u8; 20])),
		("standard_amount", Value::u128(1_000_000_000_000_000_000u128)),
	]);
	let chain = Value::named_composite(vec![
		("state_id", Value::variant("Evm", Composite::unnamed(vec![Value::u128(chain_id.into())]))),
		("consensus_state_id", Value::from_bytes(*b"ETH0")),
	]);
	Value::named_composite(vec![
		("chain", chain),
		("token_pairs", Value::unnamed_composite(vec![pair])),
		("interval_blocks", Value::u128(interval_blocks as u128)),
	])
}

/// Call `set_phantom_order_config` via sudo and seal a block. Returns the block hash.
async fn set_phantom_order_config(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc: &subxt::backend::rpc::RpcClient,
	chain_id: u64,
	interval_blocks: u32,
) -> Result<H256, anyhow::Error> {
	// Generation needs a confirmed destination height for the deadline; seed it first.
	seed_state_machine_height(client, rpc, chain_id as u32, 1_000_000).await?;
	let call = subxt::dynamic::tx(
		"IntentsCoprocessor",
		"set_phantom_order_config",
		vec![phantom_config_value(chain_id, interval_blocks)],
	);
	sudo_and_seal(client, rpc, call.into_value()).await
}

/// Read the first active phantom commitment from `CurrentPhantomOrder` storage at
/// the given block hash. Returns `None` when the storage slot is empty.
///
/// `CurrentPhantomOrder` is a `BoundedVec<(H256, PhantomOrderInfo), _>`, so the raw bytes
/// start with a one byte compact length (single byte for the small bounds here) before the
/// first entry's commitment.
async fn read_active_commitment(
	client: &subxt::OnlineClient<Hyperbridge>,
	block_hash: H256,
) -> Option<H256> {
	let raw = client
		.storage()
		.at(block_hash)
		.fetch_raw(current_phantom_order_key())
		.await
		.ok()??;
	if raw.len() < 33 {
		return None;
	}
	let mut bytes = [0u8; 32];
	bytes.copy_from_slice(&raw[1..33]);
	Some(H256::from(bytes))
}

/// Build a signed `place_bid` extrinsic for `commitment` / `user_op` but do
/// NOT submit or seal it. Returns the raw extrinsic bytes so the caller can
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

/// Governance config followed by one empty block must populate `CurrentPhantomOrder`
/// and emit a `PhantomOrderRegistered` event from `on_initialize`.
#[tokio::test]
#[ignore]
async fn test_set_phantom_order_config_stores_and_emits_event() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	set_phantom_order_config(&client, &rpc, 8453, 10).await?;

	// on_initialize fires in the next block.
	let block_hash = create_and_finalize_block(&rpc).await?;

	let raw = client
		.storage()
		.at(block_hash)
		.fetch_raw(current_phantom_order_key())
		.await?
		.expect("CurrentPhantomOrder must be set after config block");

	// BoundedVec layout: [compact len (1)] [H256 (32)] [u32 LE (4)] [SCALE compact len (1)] [chain
	// bytes]
	let chain_len = (raw[37] >> 2) as usize;
	let stored_chain = &raw[38..38 + chain_len];
	assert_eq!(stored_chain, b"EVM-8453", "chain must match the governance-set value");

	let events = client.events().at(block_hash).await?;
	let emitted = events.iter().any(|ev| {
		ev.ok()
			.map(|e| {
				e.pallet_name() == "IntentsCoprocessor" &&
					e.variant_name() == "PhantomOrderRegistered"
			})
			.unwrap_or(false)
	});
	assert!(emitted, "PhantomOrderRegistered event must be emitted by on_initialize");

	Ok(())
}

/// With `interval_blocks = 1`, each consecutive empty block must produce a
/// distinct commitment because the block number is part of the commitment preimage.
#[tokio::test]
#[ignore]
async fn test_phantom_order_replaces_previous_on_interval() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	// interval_blocks=1 means the hook re-fires every block.
	set_phantom_order_config(&client, &rpc, 8453, 1).await?;

	let block1 = create_and_finalize_block(&rpc).await?;
	let c1 = read_active_commitment(&client, block1)
		.await
		.expect("first commitment must exist");

	let block2 = create_and_finalize_block(&rpc).await?;
	let c2 = read_active_commitment(&client, block2)
		.await
		.expect("second commitment must exist");

	assert_ne!(c1, c2, "consecutive blocks must produce different commitments");

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

	set_phantom_order_config(&client, &rpc, 8453, 10).await?;

	let block_hash = create_and_finalize_block(&rpc).await?;
	let commitment = read_active_commitment(&client, block_hash)
		.await
		.expect("commitment must exist");

	let alice_ext =
		author_place_bid(&client, &rpc, commitment, &[0xAA, 0xBB], Keyring::Alice).await?;
	let alice_progress = SubmittableTransaction::from_bytes(client.clone(), alice_ext.0)
		.submit_and_watch()
		.await?;

	let bob_ext = author_place_bid(&client, &rpc, commitment, &[0xDD, 0xEE], Keyring::Bob).await?;
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
		"all returned bids must reference the active commitment",
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

	set_phantom_order_config(&client, &rpc, 8453, 10).await?;

	let block_hash = create_and_finalize_block(&rpc).await?;
	let commitment = read_active_commitment(&client, block_hash)
		.await
		.expect("commitment must exist");

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
///   Block N   — set bid window to 1 via sudo
///   Block N+1 — set phantom order config via sudo (on_initialize has no config yet)
///   Block N+2 — empty block; on_initialize fires, phantom created at N+2
///   Block N+3 — empty block; window: N+3 <= N+2+1 — still open
///   Block N+4 — bid executes; window: N+4 > N+2+1 — closed → rejected
#[tokio::test]
#[ignore]
async fn test_bid_rejected_after_phantom_window_closes() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	// Set the bid window to 1 block via governance (block N).
	let window_call = subxt::dynamic::tx(
		"IntentsCoprocessor",
		"set_phantom_bid_window",
		vec![subxt::dynamic::Value::u128(1)],
	);
	sudo_and_seal(&client, &rpc, window_call.into_value()).await?;

	// Set phantom order config (block N+1); on_initialize at N+1 has no config yet.
	set_phantom_order_config(&client, &rpc, 8453, 10).await?;

	// Block N+2: on_initialize fires, phantom created at block N+2.
	let block_hash = create_and_finalize_block(&rpc).await?;
	let commitment = read_active_commitment(&client, block_hash)
		.await
		.expect("commitment must exist at N+2");

	// Advance to block N+3 — window still open (N+3 <= N+2+1).
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

	// Seal block N+4 — bid executes here, window is now closed (N+4 > N+3).
	create_and_finalize_block(&rpc).await?;
	let result = bid_progress.wait_for_finalized_success().await;
	assert!(
		result.is_err(),
		"bid placed after window closure must be rejected with PhantomOrderBidWindowClosed",
	);

	// Reset the bid window so subsequent tests are unaffected.
	let reset_call = subxt::dynamic::tx(
		"IntentsCoprocessor",
		"set_phantom_bid_window",
		vec![subxt::dynamic::Value::u128(100)],
	);
	sudo_and_seal(&client, &rpc, reset_call.into_value()).await?;

	Ok(())
}

/// Governance (sudo) can update `PhantomBidWindow`; the new value must be
/// persisted to storage.
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

/// End-to-end phantom order flow: configure, have three fillers bid in a single
/// block, and verify all three bids are returned by `intents_getBidsForOrder`.
#[tokio::test]
#[ignore]
async fn test_phantom_order_full_flow_config_bid_rpc() -> Result<(), anyhow::Error> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc) = subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;

	set_phantom_order_config(&client, &rpc, 8453, 10).await?;

	let block_hash = create_and_finalize_block(&rpc).await?;
	let commitment = read_active_commitment(&client, block_hash)
		.await
		.expect("commitment must exist");

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

	create_and_finalize_block(&rpc).await?;
	for p in progresses {
		p.wait_for_finalized_success().await?;
	}

	let bids: Vec<RpcBidInfo> =
		rpc.request("intents_getBidsForOrder", rpc_params![commitment]).await?;
	assert_eq!(bids.len(), 3, "all three phantom bids must be discoverable via RPC");
	assert!(
		bids.iter().all(|b| b.commitment == commitment),
		"every returned bid must carry the active commitment",
	);

	let user_ops: Vec<&Vec<u8>> = bids.iter().map(|b| &b.user_op).collect();
	assert!(user_ops.contains(&&vec![0xAA]), "Alice's user_op must be present");
	assert!(user_ops.contains(&&vec![0xBB]), "Bob's user_op must be present");
	assert!(user_ops.contains(&&vec![0xCC]), "Charlie's user_op must be present");

	Ok(())
}
