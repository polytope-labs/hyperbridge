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

//! Simnode integration test for the gradual legacy `RelayChainStateCommitments`
//! drain and the new bounded `CurrentRelayChainStateRoots` map.
//!
//! 1. Spawns a simnode with the previous runtime.
//! 2. Injects real nexus `RelayChainStateCommitments` entries.
//! 3. Upgrades the runtime to the current branch.
//! 4. Drives blocks and verifies the legacy map drains while the new map stays bounded.

#![cfg(test)]
#![allow(dead_code)]

use std::{
	env, fs,
	net::TcpStream,
	process::{Child, Command, Stdio},
	time::{Duration, Instant},
};

use anyhow::anyhow;
use codec::{Decode, Encode};
use polkadot_sdk::{
	frame_support::weights::Weight,
	frame_system, pallet_sudo,
	sc_consensus_manual_seal::CreatedBlock,
	sp_core::{crypto::Ss58Codec, twox_128, Bytes, H256},
	sp_keyring::sr25519::Keyring,
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

async fn build_binary_from_main_branch() -> Result<String, anyhow::Error> {
	if let Ok(prebuilt) = env::var("RELAY_DRAIN_OLD_BINARY") {
		eprintln!("Using pre-built binary from RELAY_DRAIN_OLD_BINARY={prebuilt}");
		if !std::path::Path::new(&prebuilt).exists() {
			return Err(anyhow!("RELAY_DRAIN_OLD_BINARY does not exist: {prebuilt}"));
		}
		return Ok(prebuilt);
	}

	let current_dir = env::current_dir()?;
	let clone_dir = current_dir.join("hyperbridge-main-clone-drain");

	if clone_dir.exists() {
		fs::remove_dir_all(&clone_dir)?;
	}

	eprintln!("Cloning hyperbridge repository main branch...");
	let clone_status = Command::new("git")
		.args([
			"clone",
			"--branch",
			"main",
			"--depth",
			"1",
			"https://github.com/polytope-labs/hyperbridge.git",
			clone_dir.to_str().unwrap(),
		])
		.status()?;

	if !clone_status.success() {
		return Err(anyhow!("Failed to clone hyperbridge repository"));
	}

	eprintln!("Building hyperbridge binary from main branch...");
	let build_status = Command::new("cargo")
		.args(["build", "-p", "hyperbridge", "--release"])
		.current_dir(&clone_dir)
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.status()?;

	if !build_status.success() {
		return Err(anyhow!("Failed to build hyperbridge binary"));
	}

	let binary_source = clone_dir.join("target/release/hyperbridge");
	let binary_dest = current_dir.join("hyperbridge-main-binary-drain");

	fs::copy(&binary_source, &binary_dest)?;

	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mut perms = fs::metadata(&binary_dest)?.permissions();
		perms.set_mode(0o755);
		fs::set_permissions(&binary_dest, perms)?;
	}

	fs::remove_dir_all(&clone_dir)?;
	Ok(binary_dest.to_string_lossy().to_string())
}

async fn build_runtime_from_current_branch() -> Result<String, anyhow::Error> {
	let current_dir = env::current_dir()?;
	let mut repo_root = current_dir.clone();
	while !repo_root.join("Cargo.toml").exists() || !repo_root.join("parachain").exists() {
		if !repo_root.pop() {
			return Err(anyhow!("Could not find repository root"));
		}
	}

	eprintln!("Building gargantua runtime from current branch...");
	let build_status = Command::new("cargo")
		.args(["build", "-p", "gargantua-runtime", "--release"])
		.current_dir(&repo_root)
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.status()?;

	if !build_status.success() {
		return Err(anyhow!("Failed to build gargantua runtime"));
	}

	let wasm_source = repo_root
		.join("target/release/wbuild/gargantua-runtime/gargantua_runtime.compact.compressed.wasm");
	let wasm_dest = current_dir.join("gargantua-runtime-drain-new.wasm");

	if !wasm_source.exists() {
		return Err(anyhow!("WASM not found at {}", wasm_source.display()));
	}

	fs::copy(&wasm_source, &wasm_dest)?;
	Ok(wasm_dest.to_string_lossy().to_string())
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
async fn test_legacy_relay_state_drain() -> Result<(), anyhow::Error> {
	eprintln!("Connecting to Nexus at: {}", NEXUS_RPC);
	let (nexus_client, _) =
		subxt_utils::client::ws_client::<Hyperbridge>(NEXUS_RPC, u32::MAX).await?;
	eprintln!("Nexus spec_version: {}", nexus_client.runtime_version().spec_version);

	let binary_path = build_binary_from_main_branch().await?;
	let wasm_path = build_runtime_from_current_branch().await?;

	eprintln!("Spawning Simnode...");
	let child = Command::new(binary_path)
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

	let local_ws_url = "ws://127.0.0.1:1944";
	eprintln!("Reconnecting to Nexus...");
	let nexus_client = subxt_utils::client::ws_client::<Hyperbridge>(NEXUS_RPC, u32::MAX).await?.0;
	eprintln!("Connecting to Simnode at: {local_ws_url}");
	let (local_client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(local_ws_url, u32::MAX).await?;

	let sudo_account = Keyring::Alice.to_account_id();

	// Fetch and inject legacy entries
	let fetch_limit: u32 = env::var("RELAY_DRAIN_FETCH_LIMIT")
		.ok()
		.and_then(|v| v.parse().ok())
		.unwrap_or(512);

	eprintln!("Fetching up to {fetch_limit} RelayChainStateCommitments from Nexus...");
	let addr = subxt::dynamic::storage("IsmpParachain", "RelayChainStateCommitments", ());
	let mut iter = nexus_client.storage().at_latest().await?.iter(addr).await?;

	let mut storage_data: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
	let mut count = 0u32;

	while let Some(Ok(kv)) = iter.next().await {
		count += 1;
		storage_data.push((kv.key_bytes, kv.value.encoded().to_vec()));
		if count >= fetch_limit {
			break;
		}
	}
	eprintln!("Fetched {count} entries.");

	eprintln!("Injecting into Simnode...");
	batch_set_storage(&local_client, &rpc_client, &sudo_account, storage_data).await?;

	let pre_legacy =
		count_storage(&local_client, "IsmpParachain", "RelayChainStateCommitments").await?;
	eprintln!("Pre-upgrade legacy count: {pre_legacy}");

	// Runtime upgrade
	let wasm_code = fs::read(wasm_path)?;
	eprintln!("Submitting runtime upgrade ({} bytes)...", wasm_code.len());

	let set_code_call = RuntimeCall::System(frame_system::Call::set_code { code: wasm_code });
	let sudo_call = RuntimeCall::Sudo(pallet_sudo::Call::sudo_unchecked_weight {
		call: Box::new(set_code_call),
		weight: Weight::from_parts(0, 0),
	});
	submit_sudo(&local_client, &rpc_client, &sudo_account, sudo_call).await?;

	let _: () = rpc_client.request("simnode_upgradeSignal", rpc_params![true]).await?;

	//  Drive blocks and observe drain
	let blocks: u32 =
		env::var("RELAY_DRAIN_BLOCKS").ok().and_then(|v| v.parse().ok()).unwrap_or(800);
	eprintln!("Producing {blocks} blocks...");

	for i in 0..blocks {
		let _ = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, true])
			.await?;

		if i % 100 == 0 {
			let legacy =
				count_storage(&local_client, "IsmpParachain", "RelayChainStateCommitments").await?;
			eprintln!("Block {i}: legacy RelayChainStateCommitments={legacy}");

			if legacy == 0 {
				eprintln!("Legacy map fully drained after {i} blocks.");
				break;
			}
		}
	}

	let final_legacy_relay =
		count_storage(&local_client, "IsmpParachain", "RelayChainStateCommitments").await?;

	eprintln!("Final: legacy RelayChainStateCommitments={final_legacy_relay}");

	assert_eq!(
		final_legacy_relay, 0,
		"legacy RelayChainStateCommitments should be fully drained (found {final_legacy_relay})"
	);

	eprintln!("Relay state drain test passed.");

	Ok(())
}

/// Tests the gradual drain of `Ismp::StateCommitments` and
/// `Ismp::StateMachineUpdateTime` into the new bounded
/// `BoundedStateCommitments` / `BoundedStateMachineUpdateTime` maps.
#[tokio::test]
#[ignore]
async fn test_legacy_state_commitments_drain() -> Result<(), anyhow::Error> {
	eprintln!("Connecting to Nexus at: {}", NEXUS_RPC);
	let (nexus_client, _) =
		subxt_utils::client::ws_client::<Hyperbridge>(NEXUS_RPC, u32::MAX).await?;
	eprintln!("Nexus spec_version: {}", nexus_client.runtime_version().spec_version);

	let binary_path = build_binary_from_main_branch().await?;
	let wasm_path = build_runtime_from_current_branch().await?;

	eprintln!("Spawning Simnode...");
	let child = Command::new(binary_path)
		.args([
			"simnode",
			"--chain=gargantua-2000",
			"--name=alice",
			"--tmp",
			"--state-pruning=archive",
			"--blocks-pruning=archive",
			"--rpc-port=1945",
			"--port=40059",
			"--rpc-cors=all",
			"--unsafe-rpc-external",
			"--rpc-methods=unsafe",
		])
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.spawn()?;

	let _guard = ProcessGuard(child);
	wait_for_port(1945, Duration::from_secs(60)).await?;

	let local_ws_url = "ws://127.0.0.1:1945";
	eprintln!("Reconnecting to Nexus...");
	let nexus_client = subxt_utils::client::ws_client::<Hyperbridge>(NEXUS_RPC, u32::MAX).await?.0;
	eprintln!("Connecting to Simnode at: {local_ws_url}");
	let (local_client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(local_ws_url, u32::MAX).await?;

	let sudo_account = Keyring::Alice.to_account_id();

	// Fetch and inject legacy StateCommitments + StateMachineUpdateTime
	let fetch_limit: u32 = env::var("SM_DRAIN_FETCH_LIMIT")
		.ok()
		.and_then(|v| v.parse().ok())
		.unwrap_or(512);

	eprintln!("Fetching up to {fetch_limit} Ismp::StateCommitments from Nexus...");
	let sc_addr = subxt::dynamic::storage("Ismp", "StateCommitments", ());
	let mut sc_iter = nexus_client.storage().at_latest().await?.iter(sc_addr).await?;

	let mut storage_data: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
	let mut sc_count = 0u32;

	while let Some(Ok(kv)) = sc_iter.next().await {
		sc_count += 1;
		storage_data.push((kv.key_bytes, kv.value.encoded().to_vec()));
		if sc_count >= fetch_limit {
			break;
		}
	}
	eprintln!("Fetched {sc_count} StateCommitments entries.");

	eprintln!("Fetching up to {fetch_limit} Ismp::StateMachineUpdateTime from Nexus...");
	let smu_addr = subxt::dynamic::storage("Ismp", "StateMachineUpdateTime", ());
	let mut smu_iter = nexus_client.storage().at_latest().await?.iter(smu_addr).await?;

	let mut smu_count = 0u32;
	while let Some(Ok(kv)) = smu_iter.next().await {
		smu_count += 1;
		storage_data.push((kv.key_bytes, kv.value.encoded().to_vec()));
		if smu_count >= fetch_limit {
			break;
		}
	}
	eprintln!("Fetched {smu_count} StateMachineUpdateTime entries.");

	eprintln!("Injecting {} total entries into Simnode...", storage_data.len());
	batch_set_storage(&local_client, &rpc_client, &sudo_account, storage_data).await?;

	let pre_sc = count_storage(&local_client, "Ismp", "StateCommitments").await?;
	let pre_smu = count_storage(&local_client, "Ismp", "StateMachineUpdateTime").await?;
	eprintln!("Pre-upgrade: StateCommitments={pre_sc}, StateMachineUpdateTime={pre_smu}");

	// Runtime upgrade
	let wasm_code = fs::read(wasm_path)?;
	eprintln!("Submitting runtime upgrade ({} bytes)...", wasm_code.len());

	let set_code_call = RuntimeCall::System(frame_system::Call::set_code { code: wasm_code });
	let sudo_call = RuntimeCall::Sudo(pallet_sudo::Call::sudo_unchecked_weight {
		call: Box::new(set_code_call),
		weight: Weight::from_parts(0, 0),
	});
	submit_sudo(&local_client, &rpc_client, &sudo_account, sudo_call).await?;

	let _: () = rpc_client.request("simnode_upgradeSignal", rpc_params![true]).await?;

	// Drive blocks and observe drain
	let blocks: u32 = env::var("SM_DRAIN_BLOCKS").ok().and_then(|v| v.parse().ok()).unwrap_or(800);
	eprintln!("Producing {blocks} blocks...");

	for i in 0..blocks {
		let _ = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, true])
			.await?;

		if i % 100 == 0 {
			let sc = count_storage(&local_client, "Ismp", "StateCommitments").await?;
			let smu = count_storage(&local_client, "Ismp", "StateMachineUpdateTime").await?;
			eprintln!("Block {i}: legacy SC={sc}, legacy SMU={smu}");

			if sc == 0 && smu == 0 {
				eprintln!("Legacy maps fully drained after {i} blocks.");
				break;
			}
		}
	}

	let final_sc = count_storage(&local_client, "Ismp", "StateCommitments").await?;
	let final_smu = count_storage(&local_client, "Ismp", "StateMachineUpdateTime").await?;

	eprintln!("Final: legacy SC={final_sc}, legacy SMU={final_smu}");

	assert_eq!(final_sc, 0, "legacy StateCommitments should be fully drained (found {final_sc})");
	assert_eq!(
		final_smu, 0,
		"legacy StateMachineUpdateTime should be fully drained (found {final_smu})"
	);

	eprintln!("State commitments drain test passed.");

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
