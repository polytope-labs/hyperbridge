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

//! End-to-end simnode integration test for the v1 → v2 ismp-parachain
//! `RelayChainStateCommitments` `SteppedMigration`.
//!
//! What it exercises:
//! 1. Spins up a simnode running the **previous** runtime (built from `main`).
//! 2. Pulls the live nexus mainnet `IsmpParachain::RelayChainStateCommitments` map (which contains
//!    the production backlog this work is about) and injects it into the simnode via `set_storage`.
//! 3. Submits a `set_code` runtime upgrade to the **current** runtime built from this branch (the
//!    one with `pallet_migrations` wired in and the `MigrationV2` `SteppedMigration` registered).
//! 4. Drives blocks until the multi-block migration drains the entire backlog.
//! 5. Asserts that the resulting cache is bounded by `MAX_RELAY_STATE_COMMITMENTS` and that the new
//!    on-chain storage version is 2.

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
	sp_core::{crypto::Ss58Codec, Bytes, H256},
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

/// Mirrors `ismp_parachain::MAX_RELAY_STATE_COMMITMENTS` — duplicated here so the
/// simtests crate doesn't need a direct dep on the pallet just for one constant.
const MAX_RELAY_STATE_COMMITMENTS: u32 = 256;

struct ProcessGuard(Child);

impl Drop for ProcessGuard {
	fn drop(&mut self) {
		self.0.kill().unwrap();
	}
}

async fn build_binary_from_main_branch() -> Result<String, anyhow::Error> {
	// Allow reusing a pre-built binary via env var. This dramatically speeds up
	// iteration on the test itself, since the from-scratch `cargo build -p
	// hyperbridge` from a clean clone takes ~30 minutes.
	if let Ok(prebuilt) = env::var("RELAY_MIGRATION_OLD_BINARY") {
		eprintln!("Using pre-built binary from RELAY_MIGRATION_OLD_BINARY={prebuilt}");
		if !std::path::Path::new(&prebuilt).exists() {
			return Err(anyhow!("RELAY_MIGRATION_OLD_BINARY does not exist: {prebuilt}"));
		}
		return Ok(prebuilt);
	}

	let current_dir = env::current_dir()?;
	let clone_dir = current_dir.join("hyperbridge-main-clone-relay");

	if clone_dir.exists() {
		eprintln!("Removing existing clone directory...");
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
	let binary_dest = current_dir.join("hyperbridge-main-binary-relay");

	eprintln!("Copying binary to {}...", binary_dest.display());
	fs::copy(&binary_source, &binary_dest)?;

	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mut perms = fs::metadata(&binary_dest)?.permissions();
		perms.set_mode(0o755);
		fs::set_permissions(&binary_dest, perms)?;
	}

	eprintln!("Cleaning up clone directory...");
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
	eprintln!("Repository root: {}", repo_root.display());

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
	let wasm_dest = current_dir.join("gargantua-runtime-relay-new.wasm");

	if !wasm_source.exists() {
		return Err(anyhow!("WASM file not found at expected location: {}", wasm_source.display()));
	}

	eprintln!("Copying runtime WASM to {}...", wasm_dest.display());
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
	Err(anyhow!("Timed out waiting for connection to port"))
}

#[tokio::test]
#[ignore]
async fn test_runtime_upgrade_and_relay_state_commitments_migration() -> Result<(), anyhow::Error> {
	eprintln!("Connecting to Nexus at: {}", NEXUS_RPC);
	let (nexus_client, _) =
		subxt_utils::client::ws_client::<Hyperbridge>(NEXUS_RPC, u32::MAX).await?;
	let runtime_version = nexus_client.runtime_version();
	let spec_version = runtime_version.spec_version;
	eprintln!("Nexus runtime spec_version: {}", spec_version);

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
			"--rpc-port=1943",
			"--port=40057",
			"--rpc-cors=all",
			"--unsafe-rpc-external",
			"--rpc-methods=unsafe",
		])
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.spawn()?;

	let _guard = ProcessGuard(child);
	let port = 1943;

	eprintln!("Waiting for Simnode RPC port {port}...");
	wait_for_port(port, Duration::from_secs(60)).await?;

	let local_ws_url = format!("ws://127.0.0.1:{port}");

	eprintln!("Reconnecting to Nexus at: {}", NEXUS_RPC);
	let nexus_client = subxt_utils::client::ws_client::<Hyperbridge>(NEXUS_RPC, u32::MAX).await?.0;

	eprintln!("Connecting to Local Simnode at: {}", local_ws_url);
	let (local_client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&local_ws_url, u32::MAX).await?;

	let sudo_account = Keyring::Alice.to_account_id();
	eprintln!("Using Sudo account: {}", sudo_account.to_ss58check());

	// ----------------------------------------------------------------------
	// Pull the live nexus `IsmpParachain::RelayChainStateCommitments` map.
	// ----------------------------------------------------------------------

	// Pull only enough RelayChainStateCommitments entries from nexus to meaningfully
	// exercise the v1 → v2 drain. Iterating the entire production map over WSS would
	// take hours and isn't needed — we just want to seed the simnode with more entries
	// than `MAX_RELAY_STATE_COMMITMENTS` so the drain has real work to do and the
	// post-migration count assertion is meaningful.
	let fetch_limit: u32 = env::var("RELAY_MIGRATION_FETCH_LIMIT")
		.ok()
		.and_then(|v| v.parse().ok())
		.unwrap_or(2 * MAX_RELAY_STATE_COMMITMENTS);

	eprintln!(
		"Fetching up to {} IsmpParachain::RelayChainStateCommitments entries from Nexus...",
		fetch_limit,
	);
	let commitments_addr =
		subxt::dynamic::storage("IsmpParachain", "RelayChainStateCommitments", ());
	let mut iter = nexus_client.storage().at_latest().await?.iter(commitments_addr).await?;

	let mut storage_data: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
	let mut commitment_count = 0u32;
	let mut sentinel_block_numbers: Vec<u32> = Vec::new();

	while let Some(Ok(kv)) = iter.next().await {
		commitment_count += 1;
		// Decode the relay block number from the storage key. The key layout for
		// `Blake2_128Concat<u32, _>` is `pallet_prefix(32) ++ blake2_128(16) ++ u32(4)`.
		if kv.key_bytes.len() >= 52 && sentinel_block_numbers.len() < 5 {
			let mut suffix = &kv.key_bytes[48..52];
			if let Ok(n) = u32::decode(&mut suffix) {
				sentinel_block_numbers.push(n);
			}
		}
		storage_data.push((kv.key_bytes, kv.value.encoded().to_vec()));
		if commitment_count >= fetch_limit {
			eprintln!("Reached fetch limit of {} entries — stopping iteration.", fetch_limit);
			break;
		}
	}
	eprintln!("Sentinel keys to probe: {:?}", sentinel_block_numbers);

	eprintln!("Fetched {} relay-chain commitment entries from Nexus.", commitment_count);
	if commitment_count == 0 {
		return Err(anyhow!(
			"No RelayChainStateCommitments entries found on Nexus — \
			 nothing to migrate, test cannot validate the drain."
		));
	}
	if commitment_count <= MAX_RELAY_STATE_COMMITMENTS {
		return Err(anyhow!(
			"Fetched only {} entries; need at least {} to verify the v1 → v2 drain. \
			 Increase RELAY_MIGRATION_FETCH_LIMIT.",
			commitment_count,
			MAX_RELAY_STATE_COMMITMENTS + 1,
		));
	}

	// Inject the captured state into the simnode and snapshot the
	// pre-upgrade count for sanity.
	eprintln!("Injecting state into Simnode...");
	batch_set_storage(&local_client, &rpc_client, &sudo_account, storage_data).await?;

	let pre_upgrade_count = count_relay_state_commitments(&local_client).await?;
	eprintln!("Pre-upgrade simnode count: {}", pre_upgrade_count);
	assert!(
		pre_upgrade_count >= commitment_count,
		"set_storage should have written at least {} entries (found {})",
		commitment_count,
		pre_upgrade_count,
	);

	// Submit the runtime upgrade.
	eprintln!("Reading WASM file from: {}", wasm_path);
	let wasm_code = fs::read(wasm_path).map_err(|e| anyhow!("Failed to read WASM: {}", e))?;
	eprintln!("WASM size: {} bytes", wasm_code.len());

	eprintln!("Submitting Runtime Upgrade...");
	let set_code_call = RuntimeCall::System(frame_system::Call::set_code { code: wasm_code });
	let sudo_call = RuntimeCall::Sudo(pallet_sudo::Call::sudo_unchecked_weight {
		call: Box::new(set_code_call),
		weight: Weight::from_parts(0, 0),
	});
	submit_sudo(&local_client, &rpc_client, &sudo_account, sudo_call).await?;

	eprintln!("Signaling Simnode Upgrade...");
	let _: () = rpc_client.request("simnode_upgradeSignal", rpc_params![true]).await?;

	// Drive blocks. The migration drains one entry per `step` and pallet_migrations
	// runs as many steps per block as fit in `MaxServiceWeight`. Even at very small
	// per-step budgets, the test backlog (≤ a few thousand entries from nexus) drains
	// well within `blocks_to_produce`.
	let blocks_to_produce: u32 = std::env::var("RELAY_MIGRATION_BLOCKS")
		.ok()
		.and_then(|v| v.parse().ok())
		.unwrap_or(2_000);
	eprintln!("Producing {} blocks to drive migration...", blocks_to_produce);

	for i in 0..blocks_to_produce {
		let _ = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, true])
			.await?;

		if i % 100 == 0 {
			let n = count_relay_state_commitments(&local_client).await?;
			let counter = read_relay_state_commitments_counter(&local_client).await?;
			let storage_version =
				read_pallet_storage_version(&local_client, "IsmpParachain").await?;
			eprintln!(
				"Block {}: iter-count = {}, CounterFor = {:?}, IsmpParachain version = {}",
				i, n, counter, storage_version,
			);
			// Stop only once the migration has actually completed (storage version
			// has advanced to 2). Breaking early on `n <= MAX` would cut the test
			// off mid-drain since `MigrationV2::step` only returns `Ok(None)` after
			// `iter_keys()` returns nothing.
			if storage_version >= 2 {
				eprintln!("Migration completed after {} blocks.", i + 1);
				break;
			}
		}
	}

	// Verify post-migration state.
	let post_upgrade_count = count_relay_state_commitments(&local_client).await?;
	eprintln!("Post-upgrade simnode count: {}", post_upgrade_count);

	assert!(
		post_upgrade_count <= MAX_RELAY_STATE_COMMITMENTS,
		"after the v1 → v2 drain, RelayChainStateCommitments must be bounded by \
		 MAX_RELAY_STATE_COMMITMENTS ({}); found {}",
		MAX_RELAY_STATE_COMMITMENTS,
		post_upgrade_count,
	);

	let storage_version = read_pallet_storage_version(&local_client, "IsmpParachain").await?;
	assert_eq!(
		storage_version, 2,
		"IsmpParachain on-chain storage version should advance to 2 after the drain",
	);

	eprintln!(
		"Migration verified — pre={}, post={}, max_allowed={}, storage_version={}",
		pre_upgrade_count, post_upgrade_count, MAX_RELAY_STATE_COMMITMENTS, storage_version,
	);

	Ok(())
}

/// Count entries under `IsmpParachain::RelayChainStateCommitments` on the simnode.
async fn count_relay_state_commitments(
	client: &OnlineClient<Hyperbridge>,
) -> Result<u32, anyhow::Error> {
	let addr = subxt::dynamic::storage("IsmpParachain", "RelayChainStateCommitments", ());
	let mut iter = client.storage().at_latest().await?.iter(addr).await?;
	let mut count = 0u32;
	let mut sample_block_numbers: Vec<u32> = Vec::new();
	while let Some(Ok(kv)) = iter.next().await {
		if sample_block_numbers.len() < 5 && kv.key_bytes.len() >= 52 {
			let mut suffix = &kv.key_bytes[48..52];
			if let Ok(n) = u32::decode(&mut suffix) {
				sample_block_numbers.push(n);
			}
		}
		count += 1;
	}
	if !sample_block_numbers.is_empty() {
		eprintln!("    sample of iter() keys: {:?}", sample_block_numbers);
	}
	Ok(count)
}

/// Read the `CounterFor` value of `IsmpParachain::RelayChainStateCommitments`. Returns
/// `None` if the key isn't set (which is the case immediately after `set_storage`-based
/// injection — the counter is only maintained by `CountedStorageMap::insert`/`remove`).
async fn read_relay_state_commitments_counter(
	client: &OnlineClient<Hyperbridge>,
) -> Result<Option<u32>, anyhow::Error> {
	use polkadot_sdk::sp_core::twox_128;

	let mut key = Vec::with_capacity(48);
	key.extend_from_slice(&twox_128(b"IsmpParachain"));
	key.extend_from_slice(&twox_128(b"CounterForRelayChainStateCommitments"));

	let raw = client.storage().at_latest().await?.fetch_raw(key).await?;
	match raw {
		None => Ok(None),
		Some(bytes) => {
			let value = u32::decode(&mut bytes.as_slice())
				.map_err(|e| anyhow!("failed to decode counter: {e:?}"))?;
			Ok(Some(value))
		},
	}
}

/// Returns `true` if `IsmpParachain::RelayChainStateCommitments[block_number]` exists
/// in storage. Used to definitively check whether `Migration::step` removals are
/// being persisted to the database.
async fn relay_commitment_exists(
	client: &OnlineClient<Hyperbridge>,
	block_number: u32,
) -> Result<bool, anyhow::Error> {
	use polkadot_sdk::sp_core::{blake2_128, twox_128};

	let encoded_key = block_number.encode();
	let mut hashed = blake2_128(&encoded_key).to_vec();
	hashed.extend_from_slice(&encoded_key);

	let mut full_key = Vec::with_capacity(48 + hashed.len());
	full_key.extend_from_slice(&twox_128(b"IsmpParachain"));
	full_key.extend_from_slice(&twox_128(b"RelayChainStateCommitments"));
	full_key.extend_from_slice(&hashed);

	let raw = client.storage().at_latest().await?.fetch_raw(full_key).await?;
	Ok(raw.is_some())
}

/// Reads the on-chain storage version of a pallet via its `:__STORAGE_VERSION__:` key.
async fn read_pallet_storage_version(
	client: &OnlineClient<Hyperbridge>,
	pallet_name: &str,
) -> Result<u16, anyhow::Error> {
	use polkadot_sdk::sp_core::twox_128;

	let mut key = Vec::with_capacity(48);
	key.extend_from_slice(&twox_128(pallet_name.as_bytes()));
	key.extend_from_slice(&twox_128(b":__STORAGE_VERSION__:"));

	let raw = client
		.storage()
		.at_latest()
		.await?
		.fetch_raw(key)
		.await?
		.ok_or_else(|| anyhow!("storage version key not found for pallet {pallet_name}"))?;

	let version = u16::decode(&mut raw.as_slice())
		.map_err(|e| anyhow!("failed to decode storage version: {e:?}"))?;
	Ok(version)
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
