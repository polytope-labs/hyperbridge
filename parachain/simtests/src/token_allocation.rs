#![cfg(test)]
use anyhow::anyhow;
use codec::Encode;
use gargantua_runtime::RuntimeCall;
use pallet_bridge_airdrop::{pallet::Call as BridgeDropCall, EIGHTEEN_MONTHS};
use polkadot_sdk::{
	pallet_balances::pallet::Call as BalancesCall,
	pallet_sudo::pallet::Call as SudoCall,
	pallet_utility::pallet::Call as UtilityCall,
	pallet_vesting::{pallet::Call as VestingCall, VestingInfo},
	sc_consensus_manual_seal::CreatedBlock,
	sp_keyring::sr25519::Keyring,
};
use sp_core::{crypto::Ss58Codec, sr25519, Bytes, Pair, H256};
use std::{env, fs, str::FromStr};
use subxt::{
	ext::subxt_rpcs::{rpc_params, RpcClient},
	tx::SubmittableTransaction,
	OnlineClient,
};
use subxt_utils::Hyperbridge;

#[derive(serde::Deserialize, Debug)]
struct AllocationRecord {
	beneficiary: String,
	amount: u128,
	#[serde(default)]
	bonus_amount: Option<u128>,
}

#[tokio::test]
#[ignore]
async fn should_perform_batch_allocations() -> Result<(), anyhow::Error> {
	println!("in test");
	let port = env::var("PORT").unwrap_or_else(|_| "9990".to_string());
	let ws_url = format!("ws://127.0.0.1:{}", port);

	let iro_allocations_path = "./allocations/iro_allocations.json";
	let crowdloan_allocations_path = "./allocations/crowdloan_allocations.json";
	let manual_allocations_path = "./allocations/manual_allocations.json";

	let (client, rpc_client) =
		subxt_utils::client::ws_client::<Hyperbridge>(&ws_url, u32::MAX).await?;

	let sudo_account = match env::var("SUDO_SEED") {
		Ok(seed) => {
			let pair = sr25519::Pair::from_string(&seed, None)
				.map_err(|_| anyhow!("Invalid SUDO_SEED format"))?;
			println!("Using Sudo account from SUDO_SEED environment variable.");
			pair.public().into()
		},
		Err(_) => {
			println!("SUDO_SEED not found, defaulting to Alice.");
			Keyring::Alice.to_account_id()
		},
	};

	println!("Connecting to node at: {}", ws_url);
	println!("Using Sudo account: {}", sudo_account.to_ss58check());

	let storage_address = subxt::dynamic::storage("BridgeDrop", "StartingBlock", ());
	let maybe_starting_block = client.storage().at_latest().await?.fetch(&storage_address).await?;
	let starting_block_for_vesting = if let Some(encoded_block) = maybe_starting_block {
		let value = encoded_block.to_value()?;
		value
			.as_u128()
			.ok_or_else(|| anyhow!("Failed to decode StartingBlock as u128"))? as u32
	} else {
		0
	};
	println!("Using vesting starting block: {}", starting_block_for_vesting);

	let mut calls: Vec<RuntimeCall> = Vec::new();
	let mut records_to_check = Vec::new();

	if let Ok(data) = fs::read_to_string(iro_allocations_path) {
		let records: Vec<AllocationRecord> = serde_json::from_str(&data)?;
		for record in records {
			let dest = sp_core::crypto::AccountId32::from_str(&record.beneficiary)
				.map_err(|e| anyhow!(e))?;
			let call = RuntimeCall::BridgeDrop(BridgeDropCall::allocate_iro_tokens {
				beneficiary: dest.into(),
				amount: record.amount,
				bonus_amount: record.bonus_amount.unwrap(),
			});
			calls.push(call);
			records_to_check.push(record);
		}
	}

	if let Ok(data) = fs::read_to_string(crowdloan_allocations_path) {
		let records: Vec<AllocationRecord> = serde_json::from_str(&data)?;
		for record in records {
			let dest = sp_core::crypto::AccountId32::from_str(&record.beneficiary)
				.map_err(|e| anyhow!(e))?;
			let call = RuntimeCall::BridgeDrop(BridgeDropCall::allocate_crowdloan_tokens {
				beneficiary: dest.into(),
				amount: record.amount,
			});

			calls.push(call);
			records_to_check.push(record);
		}
	}

	if let Ok(data) = fs::read_to_string(manual_allocations_path) {
		let records: Vec<AllocationRecord> = serde_json::from_str(&data)?;
		for record in records {
			let total_amount = record.amount;
			let beneficiary = sp_core::crypto::AccountId32::from_str(&record.beneficiary)
				.map_err(|e| anyhow!(e))?;

			if total_amount > 0 {
				let locked_amount = total_amount;
				let per_block = locked_amount / EIGHTEEN_MONTHS as u128;
				let schedule =
					VestingInfo::new(total_amount, per_block, starting_block_for_vesting);

				let call = RuntimeCall::Vesting(VestingCall::force_vested_transfer {
					source: sudo_account.clone().into(),
					target: beneficiary.into(),
					schedule,
				});
				calls.push(call);
				records_to_check.push(record);
			}
		}
	}

	// pallet bridge drop address
	let recipient_account =
		sp_core::crypto::AccountId32::from_str("5EYCAe5ZVMbpbCGqFH6jAUWoWRdu9Rh3KSeUbSNvcbFZXkym")
			.map_err(|e| anyhow!(e))?;

	mint_tokens(
		client.clone(),
		rpc_client.clone(),
		&sudo_account,
		recipient_account.clone(),
		1_000_000_000 * 10u128.pow(12),
	)
	.await?;
	mint_tokens(
		client.clone(),
		rpc_client.clone(),
		&sudo_account,
		sudo_account.clone(),
		10_000_000_000_000 * 100u128.pow(12),
	)
	.await?;

	if calls.is_empty() {
		println!("No calls to execute. Exiting.");
		return Ok(());
	}

	println!("\nWrapping {} total calls in sudo(utility.batchAll(...))", calls.len());

	let batch_call = RuntimeCall::Utility(UtilityCall::batch_all { calls });
	let sudo_call = RuntimeCall::Sudo(SudoCall::sudo { call: Box::new(batch_call) });
	let final_call_data = sudo_call.encode();

	let extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(final_call_data), sudo_account.to_ss58check()],
		)
		.await?;

	let submittable = SubmittableTransaction::from_bytes(client.clone(), extrinsic.0);
	let progress = submittable.submit_and_watch().await?;

	let block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![block.hash])
		.await?;
	assert!(finalized, "Block was not finalized");

	let _events = progress.wait_for_finalized_success().await?;

	println!("\nVerifying vesting schedules for records...");
	for record in records_to_check {
		let beneficiary =
			sp_core::crypto::AccountId32::from_str(&record.beneficiary).map_err(|e| anyhow!(e))?;
		let vesting_storage_addr = subxt::dynamic::storage(
			"Vesting",
			"Vesting",
			vec![subxt::dynamic::Value::from_bytes(&beneficiary)],
		);

		let vesting_info = client.storage().at_latest().await?.fetch(&vesting_storage_addr).await?;

		assert!(vesting_info.is_some(), "Vesting schedule should exist for {}", record.beneficiary);
		println!("Vesting schedule found for {}", record.beneficiary);
	}

	Ok(())
}

async fn mint_tokens(
	client: OnlineClient<Hyperbridge>,
	rpc_client: RpcClient,
	sudo_account: &sp_core::crypto::AccountId32,
	recipient_account: sp_core::crypto::AccountId32,
	_amount: u128,
) -> Result<(), anyhow::Error> {
	let mint_amount = 1_000_000_000 * 10u128.pow(12);

	println!("\nMinting {} tokens into {}", mint_amount, recipient_account);

	let mint_call = RuntimeCall::Balances(BalancesCall::force_set_balance {
		who: recipient_account.into(),
		new_free: mint_amount,
	});
	let sudo_mint_call = RuntimeCall::Sudo(SudoCall::sudo { call: Box::new(mint_call) });
	let mint_call_data = sudo_mint_call.encode();

	let mint_extrinsic: Bytes = rpc_client
		.request(
			"simnode_authorExtrinsic",
			rpc_params![Bytes::from(mint_call_data), sudo_account.to_ss58check()],
		)
		.await?;

	let mint_submittable = SubmittableTransaction::from_bytes(client.clone(), mint_extrinsic.0);
	let mint_progress = mint_submittable.submit_and_watch().await?;

	let mint_block = rpc_client
		.request::<CreatedBlock<H256>>("engine_createBlock", rpc_params![true, false])
		.await?;
	let mint_finalized = rpc_client
		.request::<bool>("engine_finalizeBlock", rpc_params![mint_block.hash])
		.await?;
	assert!(mint_finalized, "Minting block was not finalized");
	mint_progress.wait_for_finalized_success().await?;
	println!("Minting successful!");

	Ok(())
}
