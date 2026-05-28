#![cfg(test)]

use std::{
	process::{Command, Stdio},
	time::Duration,
};

use anyhow::anyhow;
use codec::{Decode, Encode};
use polkadot_sdk::{
	frame_support::weights::Weight,
	frame_system, pallet_balances, pallet_sudo,
	sc_consensus_manual_seal::CreatedBlock,
	sp_core::H256,
	sp_io::hashing::{blake2_128, twox_128},
	sp_keyring::sr25519::Keyring,
};
use subxt::ext::subxt_rpcs::rpc_params;

use gargantua_runtime::RuntimeCall;
use subxt_utils::Hyperbridge;

use crate::migration_test::{
	batch_set_storage, build_binary_from_main_branch, build_runtime_from_current_branch,
	submit_sudo, wait_for_port, ProcessGuard,
};

/// Storage key for a `Blake2_128Concat` map: `twox_128(pallet) ++ twox_128(item) ++
/// blake2_128(key) ++ key`.
fn blake2_128_concat_key(pallet: &[u8], item: &[u8], key: &[u8]) -> Vec<u8> {
	[twox_128(pallet).as_slice(), twox_128(item).as_slice(), blake2_128(key).as_slice(), key]
		.concat()
}

/// Drives `pallet_collator_manager::migrations::MigrateBondsToReserves` on a live simnode.
///
/// Seeds one bonded collator the way nexus held them before this change (a `collbond` lock plus
/// a `Bonded` ledger entry), upgrades to the current runtime, then checks the bond moved to a
/// collator-selection reserve and the legacy lock and ledger are gone. Bob is endowed in the
/// gargantua dev genesis, so the reserve has free balance to draw from.
///
/// Set `OLD_BINARY` to a pre-built node running the pre-migration runtime to skip the
/// main-branch build. Run with `--ignored`.
#[tokio::test]
#[ignore]
async fn collator_bond_lock_migration() -> Result<(), anyhow::Error> {
	let binary_path = match std::env::var("OLD_BINARY") {
		Ok(path) => path,
		Err(_) => build_binary_from_main_branch().await?,
	};
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

	println!("Waiting for Simnode RPC port {port}...");
	wait_for_port(port, Duration::from_secs(60)).await?;

	let local_ws_url = format!("ws://127.0.0.1:{port}");
	let (local_client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&local_ws_url, u32::MAX).await?;
	let sudo_account = Keyring::Alice.to_account_id();

	let collator = Keyring::Bob.to_account_id();
	let bond: u128 = 100_000_000_000_000;
	let collator_key = collator.encode();
	let bonded_key = blake2_128_concat_key(b"CollatorManager", b"Bonded", &collator_key);
	let locks_key = blake2_128_concat_key(b"Balances", b"Locks", &collator_key);

	let locks = vec![pallet_balances::BalanceLock {
		id: *b"collbond",
		amount: bond,
		reasons: pallet_balances::Reasons::All,
	}];

	println!("Seeding legacy bond state...");
	let storage_data =
		vec![(bonded_key.clone(), bond.encode()), (locks_key.clone(), locks.encode())];
	batch_set_storage(&local_client, &rpc_client, &sudo_account, storage_data).await?;

	println!("Reading WASM file from: {}", wasm_path);
	let wasm_code = std::fs::read(&wasm_path).map_err(|e| anyhow!("Failed to read WASM: {}", e))?;

	println!("Submitting Runtime Upgrade...");
	let set_code_call = RuntimeCall::System(frame_system::Call::set_code { code: wasm_code });
	let sudo_call = RuntimeCall::Sudo(pallet_sudo::Call::sudo_unchecked_weight {
		call: Box::new(set_code_call),
		weight: Weight::from_parts(0, 0),
	});
	submit_sudo(&local_client, &rpc_client, &sudo_account, sudo_call).await?;

	let _: () = rpc_client.request("simnode_upgradeSignal", rpc_params![true]).await?;

	println!("Producing blocks to drive migration...");
	for _ in 0..3 {
		let _ = rpc_client
			.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, true])
			.await?;
	}

	println!("Verifying migration...");
	let storage = local_client.storage().at_latest().await?;

	let bonded_after = storage.fetch_raw(bonded_key).await?;
	assert!(bonded_after.is_none(), "legacy Bonded ledger entry should be drained");

	if let Some(bytes) = storage.fetch_raw(locks_key).await? {
		let remaining: Vec<pallet_balances::BalanceLock<u128>> = Decode::decode(&mut &bytes[..])?;
		assert!(
			remaining.iter().all(|lock| lock.id != *b"collbond"),
			"the collbond lock should be removed",
		);
	}

	let account_key = blake2_128_concat_key(b"System", b"Account", &collator_key);
	let account_bytes = storage
		.fetch_raw(account_key)
		.await?
		.ok_or_else(|| anyhow!("collator account should exist"))?;
	let account: frame_system::AccountInfo<u32, pallet_balances::AccountData<u128>> =
		Decode::decode(&mut &account_bytes[..])?;
	assert_eq!(account.data.reserved, bond, "the bond should be reserved after the migration");

	Ok(())
}
