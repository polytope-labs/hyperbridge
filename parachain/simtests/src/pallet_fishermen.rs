// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Simnode integration tests for `pallet-fishermen` and the collator-side
//! fisherman task.
//!
//! Tests one through three exercise the on-chain veto flow via
//! `simnode_authorExtrinsic` — a non-collator is rejected, a single collator
//! call deletes the commitment, and the priority extension keeps vetoes
//! ahead of normally-priced extrinsics in the same block. Test four spawns
//! the real fisherman task against mocked L2 RPCs and asserts it reaches
//! `EvmClient::new` on every provider. Test five is the full end-to-end
//! check: it spawns the fisherman task, injects a
//! `pallet_ismp::Event::StateMachineUpdated` into a simnode block via sudo,
//! and asserts the task reacts by submitting a veto whose effect is the
//! commitment being deleted from `pallet_ismp` storage.
//!
//! All tests are `#[ignore]`d. Run them with a simnode listening on
//! `PORT=9990` (default) via:
//!
//! ```
//! cargo test -p simtests pallet_fishermen -- --ignored
//! ```

#![cfg(test)]

use std::{env, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use codec::{Compact, Decode, Encode};
use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	host::StateMachine,
};
use polkadot_sdk::*;
use primitive_types::H256;
use sc_consensus_manual_seal::CreatedBlock;
use sc_service::TaskManager;
use serde_json::json;
use sp_core::{crypto::Ss58Codec, Bytes};
use sp_keyring::sr25519::Keyring;
use subxt::{
	backend::legacy::LegacyRpcMethods, ext::subxt_rpcs::rpc_params, tx::SubmittableTransaction,
};
use subxt_utils::{
	state_machine_commitment_storage_key, state_machine_update_time_storage_key,
	values::{state_machine_height_to_value, storage_kv_list_to_value},
	Hyperbridge,
};

use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient, SubstrateConfig};

/// Pallet index of `pallet-ismp` in the gargantua runtime. Hard-coded here
/// because we hand-roll a `RuntimeEvent` SCALE encoding below — keep in
/// sync with `parachain/runtimes/gargantua/src/lib.rs` (`#[runtime::pallet_index(41)]`).
const ISMP_PALLET_INDEX: u8 = 41;

/// Variant index of `pallet_ismp::Event::StateMachineUpdated` (it's the
/// first variant in the pallet's event enum — see
/// `modules/pallets/ismp/src/lib.rs`).
const STATE_MACHINE_UPDATED_VARIANT: u8 = 0;

/// SCALE variant byte for `frame_system::Phase::Initialization`.
const PHASE_INITIALIZATION: u8 = 2;

const SIMNODE_PORT_DEFAULT: &str = "9990";

/// Per-test fixture height. Each test takes a distinct `slot` so seeded
/// state from one test doesn't leak into the next when running sequentially
/// against a single simnode.
fn fixture_height(slot: u64) -> StateMachineHeight {
	StateMachineHeight {
		id: StateMachineId {
			state_id: ismp::host::StateMachine::Evm(42161),
			consensus_state_id: *b"ARB0",
		},
		height: 12345 + slot,
	}
}

/// Submit a sudo-wrapped call as Alice via `simnode_authorExtrinsic`, then
/// drive a manual-seal block and finalize it. Returns the block hash.
async fn submit_sudo_and_seal(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc: &subxt::backend::rpc::RpcClient,
	inner_call: subxt::dynamic::Value,
) -> Result<H256> {
	let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![inner_call]);
	let call_data = client.tx().call_data(&sudo_call)?;
	let extrinsic: Bytes = rpc
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), Keyring::Alice.to_account_id().to_ss58check()],
		)
		.await?;
	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
	let progress = submittable.submit_and_watch().await?;
	let block: CreatedBlock<H256> =
		rpc.request("engine_createBlock", rpc_params![true, false]).await?;
	let finalized: bool = rpc.request("engine_finalizeBlock", rpc_params![block.hash]).await?;
	assert!(finalized);
	progress.wait_for_finalized_success().await?;
	Ok(block.hash)
}

/// Seed a state commitment at the fixture height via sudo set_storage. The
/// stored commitment, update time, and bounded-map heights are written
/// directly without going through any consensus path.
async fn seed_state_commitment(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc: &subxt::backend::rpc::RpcClient,
	height: StateMachineHeight,
) -> Result<StateCommitment> {
	let commitment = StateCommitment {
		timestamp: 1,
		overlay_root: Some(H256::repeat_byte(0xAA)),
		state_root: H256::repeat_byte(0xAA),
	};
	let kv = vec![
		(state_machine_commitment_storage_key(height), commitment.encode()),
		(state_machine_update_time_storage_key(height), 1u64.encode()),
	];
	let set_call = subxt::dynamic::tx("System", "set_storage", vec![storage_kv_list_to_value(&kv)]);
	submit_sudo_and_seal(client, rpc, set_call.into_value()).await?;
	Ok(commitment)
}

/// Submit a `pallet_fishermen::veto_state_commitment` signed by `account`,
/// using `simnode_authorExtrinsic` so we don't need the account's private
/// key locally. Drives a block and finalizes it. Returns the block hash and
/// whether the veto landed (false means the txpool rejected the tx OR the
/// dispatch errored — both mean "outsider couldn't veto" in our model).
async fn submit_veto_signed_by(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc: &subxt::backend::rpc::RpcClient,
	signer: &str,
	height: StateMachineHeight,
) -> Result<(H256, bool)> {
	let veto_call = subxt::dynamic::tx(
		"Fishermen",
		"veto_state_commitment",
		vec![state_machine_height_to_value(&height)],
	);
	let call_data = client.tx().call_data(&veto_call)?;
	let extrinsic: Bytes = rpc
		.request("simnode_authorExtrinsic", rpc_params![Bytes::from(call_data), signer])
		.await?;
	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);

	// Submission can fail at the txpool level (e.g. `Invalid::Payment` for a
	// signer with zero balance). Treat that as a clean "rejected" outcome
	// rather than propagating, so the caller sees `ok=false`.
	let progress = match submittable.submit_and_watch().await {
		Ok(p) => p,
		Err(_) => {
			// Still author a block so the chain advances and downstream tests
			// don't see the seeded state mutated by an in-flight extrinsic.
			let block: CreatedBlock<H256> =
				rpc.request("engine_createBlock", rpc_params![true, false]).await?;
			let finalized: bool =
				rpc.request("engine_finalizeBlock", rpc_params![block.hash]).await?;
			assert!(finalized);
			return Ok((block.hash, false));
		},
	};
	let block: CreatedBlock<H256> =
		rpc.request("engine_createBlock", rpc_params![true, false]).await?;
	let finalized: bool = rpc.request("engine_finalizeBlock", rpc_params![block.hash]).await?;
	assert!(finalized);
	let dispatch_ok = progress.wait_for_finalized_success().await.is_ok();
	Ok((block.hash, dispatch_ok))
}

/// Returns true iff `pallet_ismp::StateCommitments[height]` is set on the
/// latest finalized block.
async fn state_commitment_present(
	client: &subxt::OnlineClient<Hyperbridge>,
	height: StateMachineHeight,
) -> Result<bool> {
	let raw = client
		.storage()
		.at_latest()
		.await?
		.fetch_raw(state_machine_commitment_storage_key(height))
		.await?;
	Ok(raw.is_some())
}

#[tokio::test]
#[ignore]
async fn outsider_cannot_veto() -> Result<()> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;
	let _rpc = LegacyRpcMethods::<Hyperbridge>::new(rpc_client.clone());

	let height = fixture_height(1);
	seed_state_commitment(&client, &rpc_client, height).await?;

	// A random ss58 address that's not in the validator set.
	let outsider = sp_core::crypto::AccountId32::from([7u8; 32]);
	let outsider_ss58 = outsider.to_ss58check();

	let (_, ok) = submit_veto_signed_by(&client, &rpc_client, &outsider_ss58, height).await?;
	assert!(!ok, "outsider's veto should have been rejected by IsCollator gate");
	assert!(
		state_commitment_present(&client, height).await?,
		"state commitment must still be present after a rejected outsider veto",
	);
	Ok(())
}

#[tokio::test]
#[ignore]
async fn single_collator_veto_deletes_commitment() -> Result<()> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;
	let _rpc = LegacyRpcMethods::<Hyperbridge>::new(rpc_client.clone());

	let height = fixture_height(2);
	seed_state_commitment(&client, &rpc_client, height).await?;
	assert!(state_commitment_present(&client, height).await?, "fixture must be seeded");

	let alice_ss58 = Keyring::Alice.to_account_id().to_ss58check();
	let (_, ok) = submit_veto_signed_by(&client, &rpc_client, &alice_ss58, height).await?;
	assert!(ok, "Alice (collator) must be allowed to veto");

	// Single-collator veto finalizes immediately — the commitment is gone after one call.
	assert!(
		!state_commitment_present(&client, height).await?,
		"state commitment must be deleted after a single collator's veto",
	);
	Ok(())
}

#[tokio::test]
#[ignore]
async fn veto_lands_above_normal_extrinsic_in_block() -> Result<()> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;
	let _rpc = LegacyRpcMethods::<Hyperbridge>::new(rpc_client.clone());

	let height = fixture_height(3);
	seed_state_commitment(&client, &rpc_client, height).await?;

	// Submit a normal signed extrinsic (Bob's `System::remark`) FIRST.
	let remark_call = subxt::dynamic::tx(
		"System",
		"remark",
		vec![subxt::dynamic::Value::from_bytes(b"hello from Bob".to_vec())],
	);
	let bob_ss58 = Keyring::Bob.to_account_id().to_ss58check();
	let bob_call_data = client.tx().call_data(&remark_call)?;
	let bob_ext: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(bob_call_data), bob_ss58.clone()],
		)
		.await?;
	let bob_progress = SubmittableTransaction::from_bytes(client.clone(), bob_ext.0)
		.submit_and_watch()
		.await?;

	// Then submit Alice's veto. Despite arriving second, the priority
	// extension must put it before Bob's remark in the next block.
	let veto_call = subxt::dynamic::tx(
		"Fishermen",
		"veto_state_commitment",
		vec![state_machine_height_to_value(&height)],
	);
	let alice_ss58 = Keyring::Alice.to_account_id().to_ss58check();
	let alice_call_data = client.tx().call_data(&veto_call)?;
	let alice_ext: Bytes = rpc_client
		.request("simnode_authorExtrinsic", rpc_params![Bytes::from(alice_call_data), alice_ss58])
		.await?;
	let alice_progress = SubmittableTransaction::from_bytes(client.clone(), alice_ext.0)
		.submit_and_watch()
		.await?;

	// Author one block, finalize.
	let block: CreatedBlock<H256> =
		rpc_client.request("engine_createBlock", rpc_params![true, false]).await?;
	let finalized: bool =
		rpc_client.request("engine_finalizeBlock", rpc_params![block.hash]).await?;
	assert!(finalized);
	bob_progress.wait_for_finalized_success().await?;
	alice_progress.wait_for_finalized_success().await?;

	// Inspect the block body. The veto extrinsic must come before the remark.
	let block_at = client.blocks().at(block.hash).await?;
	let extrinsics = block_at.extrinsics().await?;

	let mut veto_idx: Option<usize> = None;
	let mut remark_idx: Option<usize> = None;
	for (i, ext) in extrinsics.iter().enumerate() {
		let pallet = ext.pallet_name()?;
		let call = ext.variant_name()?;
		if pallet == "Fishermen" && call == "veto_state_commitment" {
			veto_idx = Some(i);
		}
		if pallet == "System" && call == "remark" {
			remark_idx = Some(i);
		}
	}
	let veto_idx = veto_idx.ok_or_else(|| anyhow!("veto extrinsic missing from block"))?;
	let remark_idx = remark_idx.ok_or_else(|| anyhow!("remark extrinsic missing from block"))?;
	assert!(
		veto_idx < remark_idx,
		"veto (idx {veto_idx}) must come before remark (idx {remark_idx}) due to PrioritizeVeto",
	);
	Ok(())
}

// ---------------------------------------------------------------------------
// Test 4: fisherman task end-to-end against mocked L2 RPCs.
// ---------------------------------------------------------------------------

/// JSON-RPC response factory for a minimal `eth_chainId` reply.
fn chain_id_response(id: u64, chain_id_hex: &str) -> String {
	json!({
		"jsonrpc": "2.0",
		"id": id,
		"result": chain_id_hex,
	})
	.to_string()
}

/// JSON-RPC response factory for `eth_blockNumber`.
fn block_number_response(id: u64, height_hex: &str) -> String {
	json!({
		"jsonrpc": "2.0",
		"id": id,
		"result": height_hex,
	})
	.to_string()
}

/// JSON-RPC response factory for `eth_getBlockByNumber`.
fn block_response(id: u64, height_hex: &str, state_root: &str) -> String {
	json!({
		"jsonrpc": "2.0",
		"id": id,
		"result": {
			"number": height_hex,
			"hash": "0x0000000000000000000000000000000000000000000000000000000000000001",
			"parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"stateRoot": state_root,
			"transactionsRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"receiptsRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"logsBloom": format!("0x{}", "0".repeat(512)),
			"difficulty": "0x0",
			"gasLimit": "0x0",
			"gasUsed": "0x0",
			"timestamp": "0x0",
			"extraData": "0x",
			"miner": "0x0000000000000000000000000000000000000000",
			"sha3Uncles": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"size": "0x0",
			"transactions": [],
			"uncles": [],
		},
	})
	.to_string()
}

/// One mock L2 RPC, plus the chain_id mock so the test can assert it
/// received at least one call (indicating `EvmClient::new` got far enough
/// to query its chain id during construction).
struct MockL2 {
	server: mockito::ServerGuard,
	chain_id_mock: mockito::Mock,
}

impl MockL2 {
	fn url(&self) -> String {
		self.server.url()
	}
}

async fn spawn_mock_l2(state_root: &str) -> MockL2 {
	let mut server = mockito::Server::new_async().await;
	let chain_id_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::PartialJsonString(r#"{"method":"eth_chainId"}"#.to_string()))
		.with_status(200)
		.with_header("content-type", "application/json")
		.with_body(chain_id_response(1, "0xa4b1"))
		.expect_at_least(1)
		.create_async()
		.await;
	server
		.mock("POST", "/")
		.match_body(mockito::Matcher::PartialJsonString(
			r#"{"method":"eth_blockNumber"}"#.to_string(),
		))
		.with_status(200)
		.with_header("content-type", "application/json")
		.with_body(block_number_response(1, "0x100"))
		.create_async()
		.await;
	server
		.mock("POST", "/")
		.match_body(mockito::Matcher::PartialJsonString(
			r#"{"method":"eth_getBlockByNumber"}"#.to_string(),
		))
		.with_status(200)
		.with_header("content-type", "application/json")
		.with_body(block_response(1, "0x100", state_root))
		.create_async()
		.await;
	MockL2 { server, chain_id_mock }
}

/// Build a Hyperbridge `IsmpProvider` against the running simnode.
async fn build_hyperbridge_provider(
	rpc_ws: String,
) -> Result<(Arc<dyn IsmpProvider>, StateMachine)> {
	let signer = "0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a".to_string();
	let cfg = SubstrateConfig {
		state_machine: None,
		hashing: None,
		consensus_state_id: None,
		rpc_ws,
		max_rpc_payload_size: None,
		signer: Some(signer),
		initial_height: None,
		max_concurent_queries: None,
		poll_interval: None,
		fee_token_decimals: None,
	};
	let resolved = cfg.resolve().await?;
	let state_machine = resolved.state_machine();
	let client = SubstrateClient::<KeccakSubstrateChain>::new(resolved).await?;
	Ok((Arc::new(client) as Arc<dyn IsmpProvider>, state_machine))
}

/// Build a multi-RPC `EvmClient` against the supplied mock URLs. Returns it
/// wrapped as `IsmpProvider` so it can be passed to `tesseract_fisherman::fish`.
async fn build_l2_provider(
	rpc_urls: Vec<String>,
	hyperbridge: Arc<dyn IsmpProvider>,
) -> Result<Arc<dyn IsmpProvider>> {
	let cfg = EvmConfig { rpc_urls, ..Default::default() };
	let resolved = cfg.resolve().await?;
	let mut client = EvmClient::new(resolved).await?;
	client.set_latest_finalized_height(hyperbridge).await?;
	Ok(Arc::new(client) as Arc<dyn IsmpProvider>)
}

#[tokio::test]
#[ignore]
async fn fisherman_task_spawns_against_simnode_and_mock_l2_rpcs() -> Result<()> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let hyperbridge_url = format!("ws://127.0.0.1:{port}");

	let mock_a =
		spawn_mock_l2("0x1111111111111111111111111111111111111111111111111111111111111111").await;
	let mock_b =
		spawn_mock_l2("0x2222222222222222222222222222222222222222222222222222222222222222").await;

	let (hyperbridge, coprocessor) = build_hyperbridge_provider(hyperbridge_url).await?;
	let l2 = build_l2_provider(vec![mock_a.url(), mock_b.url()], hyperbridge.clone()).await?;

	let task_manager = TaskManager::new(tokio::runtime::Handle::current(), None)?;
	tesseract_fisherman::fish(hyperbridge, l2, &task_manager.spawn_essential_handle(), coprocessor)
		.await?;

	tokio::time::sleep(Duration::from_secs(3)).await;

	mock_a.chain_id_mock.assert_async().await;
	mock_b.chain_id_mock.assert_async().await;

	drop(task_manager);
	Ok(())
}

// ---------------------------------------------------------------------------
// Test 5: end-to-end byzantine detection.
// Spawns the fisherman task, injects a `StateMachineUpdated` event into a
// simnode block, and asserts the task reacts by submitting a veto whose
// effect is the recorded state commitment being deleted.
// ---------------------------------------------------------------------------

/// Storage key for `frame_system::Events` (a `StorageValue`, so the key is
/// just `twox_128("System") || twox_128("Events")`).
fn system_events_storage_key() -> Vec<u8> {
	[sp_core::twox_128(b"System").to_vec(), sp_core::twox_128(b"Events").to_vec()].concat()
}

/// Hand-roll the SCALE encoding of a `Vec<EventRecord<RuntimeEvent, H256>>`
/// containing a single `pallet_ismp::Event::StateMachineUpdated`. Done by
/// hand because the simtests crate doesn't pull in `frame-system` directly,
/// and pulling it in just for this would inflate the build for everyone.
///
/// Layout:
/// ```text
/// Vec<EventRecord> = compact_len || record
/// EventRecord      = phase || event || topics
/// phase            = Phase::Initialization (variant byte 2)
/// event            = RuntimeEvent::Ismp variant byte
///                  || pallet_ismp::Event::StateMachineUpdated variant byte
///                  || state_machine_id.encode() || latest_height (LE u64)
/// topics           = Vec<H256> (length 0 → compact byte 0x00)
/// ```
fn encode_state_machine_updated_events(
	state_machine_id: StateMachineId,
	latest_height: u64,
) -> Vec<u8> {
	let mut record = Vec::new();
	record.push(PHASE_INITIALIZATION);
	record.push(ISMP_PALLET_INDEX);
	record.push(STATE_MACHINE_UPDATED_VARIANT);
	record.extend_from_slice(&state_machine_id.encode());
	record.extend_from_slice(&latest_height.encode());
	record.push(0x00); // topics: empty Vec<H256>

	let mut buf = Compact(1u32).encode();
	buf.extend_from_slice(&record);
	buf
}

/// Inject a synthetic `StateMachineUpdated` ISMP event into the next block.
/// Works by sudo-overwriting `System::Events` with a SCALE-encoded vec
/// containing one `pallet_ismp::Event::StateMachineUpdated` record. The
/// runtime API `pallet_ismp::block_events` reads this storage and surfaces
/// the event to the fisherman task's poll loop.
async fn inject_state_machine_updated_event(
	client: &subxt::OnlineClient<Hyperbridge>,
	rpc: &subxt::backend::rpc::RpcClient,
	state_machine_id: StateMachineId,
	latest_height: u64,
) -> Result<H256> {
	let kv = vec![(
		system_events_storage_key(),
		encode_state_machine_updated_events(state_machine_id, latest_height),
	)];
	let set_call = subxt::dynamic::tx("System", "set_storage", vec![storage_kv_list_to_value(&kv)]);
	submit_sudo_and_seal(client, rpc, set_call.into_value()).await
}

#[tokio::test]
#[ignore]
async fn fisherman_task_detects_disagreement_and_submits_veto() -> Result<()> {
	let port = env::var("PORT").unwrap_or(SIMNODE_PORT_DEFAULT.into());
	let url = format!("ws://127.0.0.1:{port}");
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&url, u32::MAX).await?;
	let _rpc = LegacyRpcMethods::<Hyperbridge>::new(rpc_client.clone());

	// Two providers returning *different* state roots. The byzantine handler's
	// "providers disagree" branch should fire on the first poll.
	let mock_a =
		spawn_mock_l2("0x1111111111111111111111111111111111111111111111111111111111111111").await;
	let mock_b =
		spawn_mock_l2("0x2222222222222222222222222222222222222222222222222222222222222222").await;

	// `EvmClient` resolves chain_id 42161 (= 0xa4b1) to consensus_state_id
	// "ETH0" via `consensus_state_id_for_chain_id`. The injected event must
	// match exactly, otherwise `state_machine_updates`'s filter drops it.
	let l2_state_machine_id =
		StateMachineId { state_id: StateMachine::Evm(42161), consensus_state_id: *b"ETH0" };
	let l2_latest_height: u64 = 0x100;
	let l2_height = StateMachineHeight { id: l2_state_machine_id, height: l2_latest_height };

	// Seed a recorded commitment so `query_state_machine_commitment` resolves.
	// The recorded root is irrelevant — disagreement among the two mocks
	// triggers the veto before recorded-vs-quorum is checked.
	seed_state_commitment(&client, &rpc_client, l2_height).await?;
	assert!(state_commitment_present(&client, l2_height).await?, "fixture must be seeded");

	// Spawn the fisherman BEFORE injecting the event so its initial
	// `query_finalized_height` baseline sits below the block we'll author.
	let (hyperbridge, coprocessor) = build_hyperbridge_provider(url).await?;
	let l2 = build_l2_provider(vec![mock_a.url(), mock_b.url()], hyperbridge.clone()).await?;

	let task_manager = TaskManager::new(tokio::runtime::Handle::current(), None)?;
	tesseract_fisherman::fish(hyperbridge, l2, &task_manager.spawn_essential_handle(), coprocessor)
		.await?;

	// Give the task a moment to subscribe and capture its baseline height.
	tokio::time::sleep(Duration::from_secs(2)).await;

	// Inject the StateMachineUpdated event into a freshly-authored block.
	inject_state_machine_updated_event(&client, &rpc_client, l2_state_machine_id, l2_latest_height)
		.await?;

	// The fisherman polls every 3s. Wait long enough for it to see the new
	// block, query both mocks, observe the disagreement, and push the veto
	// extrinsic into the txpool. With single-collator finalization the veto
	// deletes the commitment in one shot, so we watch for that.
	let mut deleted = false;
	for _ in 0..20 {
		// Drive a block so any queued veto extrinsic gets included.
		let block: CreatedBlock<H256> =
			rpc_client.request("engine_createBlock", rpc_params![true, false]).await?;
		let _: bool = rpc_client.request("engine_finalizeBlock", rpc_params![block.hash]).await?;
		if !state_commitment_present(&client, l2_height).await? {
			deleted = true;
			break;
		}
		tokio::time::sleep(Duration::from_secs(1)).await;
	}

	drop(task_manager);
	assert!(
		deleted,
		"fisherman task should have detected disagreement and submitted a veto that deleted the commitment",
	);
	Ok(())
}
