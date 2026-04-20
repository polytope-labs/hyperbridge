// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Simnode integration test for the on_idle-driven legacy storage drain.
//!
//! Spawns a simnode from the pre-upgrade (main) binary, injects real nexus data
//! for all three legacy maps, upgrades the runtime, drives blocks, and verifies
//! everything drains to zero via the `on_idle` hooks.

#![cfg(test)]

use anyhow::anyhow;
use codec::Encode;
use polkadot_sdk::{
	frame_support::weights::Weight,
	frame_system, pallet_sudo,
	sc_consensus_manual_seal::CreatedBlock,
	sp_core::{crypto::Ss58Codec, Bytes, H256},
	sp_keyring::sr25519::Keyring,
};
use std::{
	env, fs,
	net::TcpStream,
	process::{Child, Command, Stdio},
	time::{Duration, Instant},
};
use subxt::{
	ext::subxt_rpcs::{rpc_params, RpcClient},
	tx::SubmittableTransaction,
	OnlineClient,
};

use gargantua_runtime::RuntimeCall;
use subxt_utils::Hyperbridge;

const NEXUS_RPC: &str = "wss://nexus.ibp.network";

struct ProcessGuard(Child);

impl Drop for ProcessGuard {
	fn drop(&mut self) {
		self.0.kill().unwrap();
	}
}

fn old_binary_path() -> Result<String, anyhow::Error> {
	env::var("SIMNODE_OLD_BINARY").map_err(|_| {
		anyhow!(
			"SIMNODE_OLD_BINARY env var not set. \
			 Set it to the path of the pre-upgrade hyperbridge binary."
		)
	})
}

fn runtime_wasm_path() -> Result<String, anyhow::Error> {
	let current_dir = env::current_dir()?;
	let mut repo_root = current_dir.clone();
	while !repo_root.join("Cargo.toml").exists() || !repo_root.join("parachain").exists() {
		if !repo_root.pop() {
			return Err(anyhow!("Could not find repository root"));
		}
	}

	let wasm_path = repo_root
		.join("target/release/wbuild/gargantua-runtime/gargantua_runtime.compact.compressed.wasm");

	if !wasm_path.exists() {
		return Err(anyhow!(
			"Runtime WASM not found at {}. Run `cargo build -p gargantua-runtime --release` first.",
			wasm_path.display()
		));
	}

	Ok(wasm_path.to_string_lossy().to_string())
}

async fn wait_for_port(port: u16, timeout: Duration) -> Result<(), anyhow::Error> {
	let start = Instant::now();
	while start.elapsed() < timeout {
		if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
			return Ok(());
		}
		tokio::time::sleep(Duration::from_secs(1)).await;
	}
	Err(anyhow!("Timed out waiting for port {port}"))
}

#[tokio::test]
#[ignore]
async fn test_legacy_storage_drain() -> Result<(), anyhow::Error> {
	let binary = old_binary_path()?;
	let wasm_path = runtime_wasm_path()?;

	eprintln!("Spawning Simnode from old binary...");
	let child = Command::new(&binary)
		.args([
			"simnode",
			"--chain=gargantua-2000",
			"--name=alice",
			"--tmp",
			"--state-pruning=archive",
			"--blocks-pruning=archive",
			"--rpc-port=1944",
			"--port=40058",
			"--rpc-cors=all",
			"--unsafe-rpc-external",
			"--rpc-methods=unsafe",
		])
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.spawn()?;

	let _guard = ProcessGuard(child);
	wait_for_port(1944, Duration::from_secs(60)).await?;

	let local_ws = "ws://127.0.0.1:1944";
	eprintln!("Connecting to Nexus at: {NEXUS_RPC}");
	let (nexus_client, _) =
		subxt_utils::client::ws_client::<Hyperbridge>(NEXUS_RPC, u32::MAX).await?;

	eprintln!("Connecting to Simnode at: {local_ws}");
	let (local_client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(local_ws, u32::MAX).await?;

	let sudo_account = Keyring::Alice.to_account_id();

	// ── Fetch all legacy data from Nexus ────────────────────────────
	let fetch_limit: u32 =
		env::var("DRAIN_FETCH_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(512);

	let mut storage_data: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

	eprintln!("Fetching up to {fetch_limit} RelayChainStateCommitments from Nexus...");
	let addr = subxt::dynamic::storage("IsmpParachain", "RelayChainStateCommitments", ());
	let mut iter = nexus_client.storage().at_latest().await?.iter(addr).await?;
	let mut relay_count = 0u32;
	while let Some(Ok(kv)) = iter.next().await {
		relay_count += 1;
		storage_data.push((kv.key_bytes, kv.value.encoded().to_vec()));
		if relay_count >= fetch_limit {
			break;
		}
	}
	eprintln!("Fetched {relay_count} RelayChainStateCommitments.");

	eprintln!("Fetching up to {fetch_limit} StateCommitments from Nexus...");
	let addr = subxt::dynamic::storage("Ismp", "StateCommitments", ());
	let mut iter = nexus_client.storage().at_latest().await?.iter(addr).await?;
	let mut sc_count = 0u32;
	while let Some(Ok(kv)) = iter.next().await {
		sc_count += 1;
		storage_data.push((kv.key_bytes, kv.value.encoded().to_vec()));
		if sc_count >= fetch_limit {
			break;
		}
	}
	eprintln!("Fetched {sc_count} StateCommitments.");

	eprintln!("Fetching up to {fetch_limit} StateMachineUpdateTime from Nexus...");
	let addr = subxt::dynamic::storage("Ismp", "StateMachineUpdateTime", ());
	let mut iter = nexus_client.storage().at_latest().await?.iter(addr).await?;
	let mut smu_count = 0u32;
	while let Some(Ok(kv)) = iter.next().await {
		smu_count += 1;
		storage_data.push((kv.key_bytes, kv.value.encoded().to_vec()));
		if smu_count >= fetch_limit {
			break;
		}
	}
	eprintln!("Fetched {smu_count} StateMachineUpdateTime.");

	// ── Inject all data into simnode ────────────────────────────────
	eprintln!("Injecting {} total entries into Simnode...", storage_data.len());
	batch_set_storage(&local_client, &rpc_client, &sudo_account, storage_data).await?;

	let pre_relay =
		count_storage(&local_client, "IsmpParachain", "RelayChainStateCommitments").await?;
	let pre_sc = count_storage(&local_client, "Ismp", "StateCommitments").await?;
	let pre_smu = count_storage(&local_client, "Ismp", "StateMachineUpdateTime").await?;
	eprintln!(
		"Pre-upgrade: RelayChain={pre_relay}, StateCommitments={pre_sc}, StateMachineUpdateTime={pre_smu}"
	);

	// ── Runtime upgrade ─────────────────────────────────────────────
	let wasm_code = fs::read(&wasm_path)?;
	eprintln!("Submitting runtime upgrade ({} bytes)...", wasm_code.len());
	let set_code_call = RuntimeCall::System(frame_system::Call::set_code { code: wasm_code });
	let sudo_call = RuntimeCall::Sudo(pallet_sudo::Call::sudo_unchecked_weight {
		call: Box::new(set_code_call),
		weight: Weight::from_parts(0, 0),
	});
	submit_sudo(&local_client, &rpc_client, &sudo_account, sudo_call).await?;

	eprintln!("Signaling Simnode Upgrade...");
	let _: () = rpc_client.request("simnode_upgradeSignal", rpc_params![true]).await?;

	// Reconnect to pick up new metadata
	let (local_client, _) =
		subxt_utils::client::ws_client::<Hyperbridge>(local_ws, u32::MAX).await?;
	eprintln!("Reconnected with new metadata.");

	// ── Drive blocks; on_idle will drain legacy maps each block ─────
	let blocks: u32 = env::var("DRAIN_BLOCKS").ok().and_then(|v| v.parse().ok()).unwrap_or(800);
	// Number of transfers bundled into one utility.batch_all per block (simulates load).
	let transfers_per_block: u32 =
		env::var("TRANSFERS_PER_BLOCK").ok().and_then(|v| v.parse().ok()).unwrap_or(200);
	eprintln!(
		"Producing {blocks} blocks with {transfers_per_block} transfers batched per block..."
	);

	// Use a fresh Charlie account so we can track balance growth from zero.
	let charlie = Keyring::Charlie.to_account_id();
	let initial_charlie = query_balance(&local_client, &charlie).await?;
	eprintln!("Charlie's initial balance: {initial_charlie}");
	let mut successful_batches = 0u64;

	for i in 0..blocks {
		// Build a utility.batch_all with many transfers FROM Alice TO Charlie.
		// This is ONE extrinsic (single nonce) but consumes weight proportional to
		// the number of inner calls, simulating production load on the block.
		let transfers: Vec<RuntimeCall> = (0..transfers_per_block)
			.map(|_| {
				RuntimeCall::Balances(polkadot_sdk::pallet_balances::Call::transfer_keep_alive {
					dest: charlie.clone().into(),
					value: 1_000_000,
				})
			})
			.collect();
		let batch_call =
			RuntimeCall::Utility(polkadot_sdk::pallet_utility::Call::batch { calls: transfers });
		// NOTE: submit signed as Alice (not wrapped in sudo). The batch runs with Alice's
		// Signed origin, so transfers actually execute from Alice's account.
		if submit_extrinsic_no_wait(&rpc_client, &sudo_account, batch_call).await.is_ok() {
			successful_batches += 1;
		}

		let _ = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, true])
			.await?;

		if i % 50 == 0 {
			let relay =
				count_storage(&local_client, "IsmpParachain", "RelayChainStateCommitments").await?;
			let sc = count_storage(&local_client, "Ismp", "StateCommitments").await?;
			let smu = count_storage(&local_client, "Ismp", "StateMachineUpdateTime").await?;
			let charlie_balance = query_balance(&local_client, &charlie).await?;
			let delta = charlie_balance.saturating_sub(initial_charlie);
			eprintln!(
				"Block {i}: RelayChain={relay}, SC={sc}, SMU={smu}, batches={successful_batches}, charlie_gain={delta}"
			);

			if relay == 0 && sc == 0 && smu == 0 {
				eprintln!("All legacy maps fully drained after {i} blocks.");
				break;
			}
		}
	}

	let final_charlie = query_balance(&local_client, &charlie).await?;
	let gain = final_charlie.saturating_sub(initial_charlie);
	eprintln!("Total batches submitted: {successful_batches}, Charlie received: {gain} planks");
	assert!(
		gain > 0,
		"Charlie received nothing — user transactions aren't executing during drain!"
	);

	let final_relay =
		count_storage(&local_client, "IsmpParachain", "RelayChainStateCommitments").await?;
	let final_sc = count_storage(&local_client, "Ismp", "StateCommitments").await?;
	let final_smu = count_storage(&local_client, "Ismp", "StateMachineUpdateTime").await?;

	eprintln!("Final: RelayChain={final_relay}, SC={final_sc}, SMU={final_smu}");

	assert_eq!(
		final_relay, 0,
		"legacy RelayChainStateCommitments should be fully drained (found {final_relay})"
	);
	assert_eq!(final_sc, 0, "legacy StateCommitments should be fully drained (found {final_sc})");
	assert_eq!(
		final_smu, 0,
		"legacy StateMachineUpdateTime should be fully drained (found {final_smu})"
	);

	eprintln!("All legacy storage drain tests passed.");
	Ok(())
}

async fn count_storage(
	client: &OnlineClient<Hyperbridge>,
	pallet: &str,
	storage: &str,
) -> Result<u32, anyhow::Error> {
	let addr = subxt::dynamic::storage(pallet, storage, ());
	let mut iter = client.storage().at_latest().await?.iter(addr).await?;
	let mut count = 0u32;
	while let Some(Ok(_)) = iter.next().await {
		count += 1;
	}
	Ok(count)
}

async fn batch_set_storage(
	client: &OnlineClient<Hyperbridge>,
	rpc_client: &RpcClient,
	sudo_account: &polkadot_sdk::sp_core::crypto::AccountId32,
	data: Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<(), anyhow::Error> {
	const BATCH_SIZE: usize = 500;
	for (i, chunk) in data.chunks(BATCH_SIZE).enumerate() {
		let items: Vec<(Vec<u8>, Vec<u8>)> = chunk.to_vec();
		let call = RuntimeCall::System(frame_system::Call::set_storage { items });
		let sudo_call = RuntimeCall::Sudo(pallet_sudo::Call::sudo_unchecked_weight {
			call: Box::new(call),
			weight: Weight::from_parts(0, 0),
		});
		eprintln!("Injecting batch {}/{}...", i + 1, (data.len() + BATCH_SIZE - 1) / BATCH_SIZE);
		submit_sudo(client, rpc_client, sudo_account, sudo_call).await?;
	}
	Ok(())
}

/// Queries the free balance of an account via the System::Account storage.
async fn query_balance(
	client: &OnlineClient<Hyperbridge>,
	account: &polkadot_sdk::sp_core::crypto::AccountId32,
) -> Result<u128, anyhow::Error> {
	use subxt::ext::scale_value::At;

	let account_bytes: &[u8] = account.as_ref();
	let addr = subxt::dynamic::storage(
		"System",
		"Account",
		vec![subxt::dynamic::Value::from_bytes(account_bytes)],
	);
	let value = client.storage().at_latest().await?.fetch(&addr).await?;
	let Some(v) = value else { return Ok(0) };
	let decoded = v.to_value()?;
	// AccountInfo { nonce, consumers, providers, sufficients, data: { free, ... } }
	let Some(free_value) = decoded.at("data").and_then(|d| d.at("free")) else {
		return Ok(0);
	};
	Ok(free_value.as_u128().unwrap_or(0))
}

/// Submits an extrinsic to the txpool without waiting for it to be included.
/// Used to fill the block with user transactions.
async fn submit_extrinsic_no_wait(
	rpc_client: &RpcClient,
	sudo_account: &polkadot_sdk::sp_core::crypto::AccountId32,
	call: RuntimeCall,
) -> Result<(), anyhow::Error> {
	let call_data = call.encode();
	let extrinsic_bytes: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), sudo_account.to_ss58check()],
		)
		.await?;

	rpc_client
		.request::<String>("author_submitExtrinsic", rpc_params![extrinsic_bytes])
		.await?;
	Ok(())
}

async fn submit_sudo(
	client: &OnlineClient<Hyperbridge>,
	rpc_client: &RpcClient,
	sudo_account: &polkadot_sdk::sp_core::crypto::AccountId32,
	call: RuntimeCall,
) -> Result<(), anyhow::Error> {
	let call_data = call.encode();
	let extrinsic_bytes: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(call_data), sudo_account.to_ss58check()],
		)
		.await?;

	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic_bytes.0);
	let progress = submittable.submit_and_watch().await?;

	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let _ = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;

	let _events = progress.wait_for_finalized_success().await?;
	Ok(())
}
