#![cfg(test)]

use std::{
	collections::HashMap,
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
	sp_core::{crypto::Ss58Codec, Bytes, H256, U256},
	sp_keyring::sr25519::Keyring,
};
use subxt::{
	ext::subxt_rpcs::{rpc_params, RpcClient},
	tx::SubmittableTransaction,
	OnlineClient,
};

use gargantua_runtime::RuntimeCall;
use ismp::host::StateMachine;
use subxt_utils::Hyperbridge;

const NEXUS_RPC: &str = "wss://nexus.ibp.network";

struct ProcessGuard(Child);

impl Drop for ProcessGuard {
	fn drop(&mut self) {
		self.0.kill().unwrap();
	}
}

async fn build_binary_from_main_branch() -> Result<String, anyhow::Error> {
	let current_dir = env::current_dir()?;
	let clone_dir = current_dir.join("hyperbridge-main-clone");

	// Clean up any existing clone
	if clone_dir.exists() {
		println!("Removing existing clone directory...");
		fs::remove_dir_all(&clone_dir)?;
	}

	println!("Cloning hyperbridge repository main branch...");
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

	println!("Building hyperbridge binary from main branch...");
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
	let binary_dest = current_dir.join("hyperbridge-main-binary");

	println!("Copying binary to {}...", binary_dest.display());
	fs::copy(&binary_source, &binary_dest)?;

	// Make executable
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mut perms = fs::metadata(&binary_dest)?.permissions();
		perms.set_mode(0o755);
		fs::set_permissions(&binary_dest, perms)?;
	}

	// Clean up clone directory
	println!("Cleaning up clone directory...");
	fs::remove_dir_all(&clone_dir)?;

	Ok(binary_dest.to_string_lossy().to_string())
}

async fn build_runtime_from_current_branch() -> Result<String, anyhow::Error> {
	let current_dir = env::current_dir()?;

	// Navigate to the repository root (assuming we're in parachain/simtests or similar)
	let mut repo_root = current_dir.clone();
	while !repo_root.join("Cargo.toml").exists() || !repo_root.join("parachain").exists() {
		if !repo_root.pop() {
			return Err(anyhow!("Could not find repository root"));
		}
	}

	println!("Building gargantua runtime from current branch...");
	println!("Repository root: {}", repo_root.display());

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
	let wasm_dest = current_dir.join("gargantua-runtime-new.wasm");

	if !wasm_source.exists() {
		return Err(anyhow!("WASM file not found at expected location: {}", wasm_source.display()));
	}

	println!("Copying runtime WASM to {}...", wasm_dest.display());
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

// #[tokio::test]
// #[ignore]
async fn test_runtime_upgrade_and_fee_migration() -> Result<(), anyhow::Error> {
	// Check nexus runtime version - only run test if version is below 6000
	println!("Connecting to Nexus at: {} to check runtime version...", NEXUS_RPC);
	let (nexus_client, _) =
		subxt_utils::client::ws_client::<Hyperbridge>(NEXUS_RPC, u32::MAX).await?;
	let runtime_version = nexus_client.runtime_version();
	let spec_version = runtime_version.spec_version;

	println!("Nexus runtime spec_version: {}", spec_version);

	if spec_version >= 6000 {
		println!("Skipping migration test - nexus runtime version {} is >= 6000", spec_version);
		return Ok(());
	}

	println!("Running migration test - nexus runtime version {} is < 6000", spec_version);

	// Build hyperbridge binary from main branch
	let binary_path = build_binary_from_main_branch().await?;

	// Build gargantua runtime from current branch
	let wasm_path = build_runtime_from_current_branch().await?;

	println!("Spawning Simnode...");
	let child = Command::new(binary_path)
		.args([
			"simnode",
			"--chain=gargantua-2000",
			"--name=alice",
			"--tmp",
			"--state-pruning=archive",
			"--blocks-pruning=archive",
			"--rpc-port=1942",
			"--port=40056",
			"--rpc-cors=all",
			"--unsafe-rpc-external",
			"--rpc-methods=unsafe",
		])
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.spawn()?;

	let _guard = ProcessGuard(child);
	let port = 1942;

	println!("Waiting for Simnode RPC port {port}...");
	wait_for_port(port, Duration::from_secs(60)).await?;


	let local_ws_url = format!("ws://127.0.0.1:{port}");

	// nexus_client already connected earlier for version check, reconnect for consistency
	println!("Reconnecting to Nexus at: {}", NEXUS_RPC);
	let nexus_client = subxt_utils::client::ws_client::<Hyperbridge>(NEXUS_RPC, u32::MAX).await?.0;

	println!("Connecting to Local Simnode at: {}", local_ws_url);
	let (local_client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&local_ws_url, u32::MAX).await?;

	let sudo_account = Keyring::Alice.to_account_id();
	println!("Using Sudo account: {}", sudo_account.to_ss58check());

	println!("Fetching Fee Token Decimals from Nexus...");
	let decimals_addr = subxt::dynamic::storage("HostExecutive", "FeeTokenDecimals", ());
	let mut decimals_iter = nexus_client.storage().at_latest().await?.iter(decimals_addr).await?;

	let mut chain_decimals: HashMap<StateMachine, u8> = HashMap::new();
	let mut storage_data: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

	while let Some(Ok(kv)) = decimals_iter.next().await {
		let key_bytes = kv.key_bytes;
		let value_thunk = kv.value;

		// offset is 32 bytes (pallet prefix + storage prefix) + 16 bytes (key hash) = 40 bytes.
		let mut key_slice = &key_bytes[48..];
		if let Ok(chain) = StateMachine::decode(&mut key_slice) {
			let mut value_slice = value_thunk.encoded();
			if let Ok(decimals) = u8::decode(&mut value_slice) {
				chain_decimals.insert(chain, decimals);
			}
		}

		storage_data.push((key_bytes, value_thunk.encoded().to_vec()));
	}
	println!("Fetched {} decimal entries.", chain_decimals.len());

	println!("Fetching Relayer Fees from Nexus...");
	let fees_addr = subxt::dynamic::storage("Relayer", "Fees", ());
	let mut fees_iter = nexus_client.storage().at_latest().await?.iter(fees_addr).await?;

	// (rawKey, stateMachine, original value)
	let mut fees_to_verify: Vec<(Vec<u8>, StateMachine, U256)> = Vec::new();

	while let Some(Ok(kv)) = fees_iter.next().await {
		let key_bytes = kv.key_bytes;
		let value_thunk = kv.value;

		// offset is 32 bytes (pallet prefix + storage prefix) + 16 bytes (key hash) = 48 bytes.
		let mut key_slice = &key_bytes[48..];
		if let Ok(chain) = StateMachine::decode(&mut key_slice) {
			let mut value_slice = value_thunk.encoded();
			if let Ok(fee_value) = U256::decode(&mut value_slice) {
				fees_to_verify.push((key_bytes.clone(), chain, fee_value));
			}
		}

		storage_data.push((key_bytes, value_thunk.encoded().to_vec()));
	}
	println!("Fetched {} fee entries to verify.", fees_to_verify.len());

	if fees_to_verify.is_empty() {
		return Err(anyhow!("No fee entries found."));
	}

	println!("Injecting state into Simnode...");
	batch_set_storage(&local_client, &rpc_client, &sudo_account, storage_data).await?;

	println!("Reading WASM file from: {}", wasm_path);
	let wasm_code = fs::read(wasm_path).map_err(|e| anyhow!("Failed to read WASM: {}", e))?;
	println!("WASM size: {} bytes", wasm_code.len());

	println!("Submitting Runtime Upgrade...");
	let set_code_call = RuntimeCall::System(frame_system::Call::set_code { code: wasm_code });
	let sudo_call = RuntimeCall::Sudo(pallet_sudo::Call::sudo_unchecked_weight {
		call: Box::new(set_code_call),
		weight: Weight::from_parts(0, 0),
	});

	submit_sudo(&local_client, &rpc_client, &sudo_account, sudo_call).await?;

	println!("Signaling Simnode Upgrade...");
	let _: () = rpc_client.request("simnode_upgradeSignal", rpc_params![true]).await?;

	println!("Producing blocks to drive migration...");
	let blocks_to_produce = 250;
	for i in 0..blocks_to_produce {
		let _ = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, true])
			.await?;

		if i % 100 == 0 {
			println!("Produced {}/{} blocks...", i, blocks_to_produce);
		}
	}

	println!("Verifying Migrations...");
	let mut changed_count = 0;
	let mut unchanged_count = 0;

	for (raw_key, chain, old_val) in fees_to_verify {
		let fetch_res =
			local_client.storage().at_latest().await?.fetch_raw(raw_key.clone()).await?;

		if chain.is_substrate() {
			continue;
		}

		if let Some(data) = fetch_res {
			let new_val = U256::decode(&mut &data[..])?;
			let msg = format!("Expected decimals for {chain:?}");
			let decimals = *chain_decimals.get(&chain).expect(&msg);
			let scaling_power = 18u32.saturating_sub(decimals as u32);
			let divisor = U256::from(10).pow(U256::from(scaling_power));
			let expected = old_val / divisor;
			if new_val != old_val && new_val == expected {
				changed_count += 1;
				println!(" Successful storage change {chain:?} {decimals} decimals: Old {old_val}, New {new_val}");
			} else if new_val != old_val && new_val != expected {
				panic!("Error in migration Expected {expected} for Key(0x{}), found {new_val}, chain: {chain:?}, decimals {decimals}", hex::encode(raw_key.clone()))
			} else {
				unchanged_count += 1;
				println!(" Storage Unchanged for Key(0x{}), chain: {chain:?}, decimals {decimals}, Value: {old_val:?}", hex::encode(raw_key));
			}
		}
	}

	println!("Changed Count: {}", changed_count);
	println!("Unchanged Count: {}", unchanged_count);

	Ok(())
}

async fn batch_set_storage(
	client: &OnlineClient<Hyperbridge>,
	rpc_client: &RpcClient,
	sudo_account: &sp_core::crypto::AccountId32,
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

		println!("Injecting batch {}/{}...", i + 1, (data.len() + BATCH_SIZE - 1) / BATCH_SIZE);
		submit_sudo(client, rpc_client, sudo_account, sudo_call).await?;
	}
	Ok(())
}

async fn submit_sudo(
	client: &OnlineClient<Hyperbridge>,
	rpc_client: &RpcClient,
	sudo_account: &sp_core::crypto::AccountId32,
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

	let events = progress.wait_for_finalized_success().await?;
	Ok(())
}
