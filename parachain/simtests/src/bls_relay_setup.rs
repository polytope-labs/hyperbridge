//! One-off setup for the BLS BEEFY end-to-end test: register a parachain on the local relay.
//!
//! `pallet-beefy-consensus-proofs` only accepts a proof that finalizes a parachain head it has
//! not seen, and it reads the child trie root out of that head. A relay with no registered
//! parachains therefore cannot satisfy it, no matter how good the BLS proof is. Registering one
//! gives the relay's MMR leaves a non-empty parachain heads root and lets the whole path run.
//!
//! The para id must be 4009, which is what gargantua's `is_parachain_tracked` allows and what its
//! coprocessor state machine resolves to; a head under any other id is filtered out and the
//! commitment lookup misses.
//!
//! Generate the artifacts first:
//!
//! ```text
//! hyperbridge build-spec --chain gargantua-4009 --disable-default-bootnode > plain.json
//! # patch "relay_chain" to the relay's id, then
//! hyperbridge build-spec --chain plain.json --raw --disable-default-bootnode > raw.json
//! hyperbridge export-genesis-head --chain raw.json > head.hex
//! hyperbridge export-genesis-wasm --chain raw.json > wasm.hex
//! ```
//!
//! then:
//!
//! ```text
//! RELAY_WS_URL=ws://127.0.0.1:9979 PARA_HEAD_PATH=head.hex PARA_WASM_PATH=wasm.hex \
//!   cargo test -p simtests register_parachain -- --ignored --nocapture
//! ```

#![cfg(test)]

use std::{env, fs};

use anyhow::anyhow;
use polkadot_sdk::sp_core::Bytes;
use subxt::{dynamic::Value, ext::subxt_rpcs::rpc_params, PolkadotConfig};

/// Read a `0x`-prefixed hex blob written by `export-genesis-*`.
fn read_hex(path: &str) -> Result<Vec<u8>, anyhow::Error> {
	let raw = fs::read_to_string(path).map_err(|e| anyhow!("reading {path}: {e}"))?;
	let trimmed = raw.trim().trim_start_matches("0x");
	hex::decode(trimmed).map_err(|e| anyhow!("decoding {path}: {e}"))
}

#[tokio::test]
#[ignore]
async fn register_parachain_on_bls_relay() -> Result<(), anyhow::Error> {
	let relay_ws_url = env::var("RELAY_WS_URL")
		.map_err(|_| anyhow!("RELAY_WS_URL must point at the BLS BEEFY relay"))?;
	let head_path = env::var("PARA_HEAD_PATH").unwrap_or_else(|_| "/tmp/g4009-head.hex".into());
	let wasm_path = env::var("PARA_WASM_PATH").unwrap_or_else(|_| "/tmp/g4009-wasm.hex".into());
	let para_id: u32 = env::var("PARA_ID").unwrap_or_else(|_| "4009".into()).parse()?;

	let genesis_head = read_hex(&head_path)?;
	let validation_code = read_hex(&wasm_path)?;
	eprintln!(
		"[stage] para {para_id}: head {} bytes, validation code {} bytes",
		genesis_head.len(),
		validation_code.len(),
	);

	// The validation code alone is ~2MB, so the default payload ceiling is not enough.
	let (client, rpc_client) =
		subxt_utils::client::ws_client::<PolkadotConfig>(&relay_ws_url, u32::MAX).await?;

	let already: Vec<u32> = parachains(&rpc_client).await.unwrap_or_default();
	if already.contains(&para_id) {
		eprintln!("[ok] para {para_id} is already registered, nothing to do");
		return Ok(());
	}

	// `para_kind: true` registers a parachain rather than a parathread, so it gets a core and
	// its heads land in the relay's parachain heads root.
	let genesis = Value::named_composite(vec![
		("genesis_head", Value::from_bytes(&genesis_head)),
		("validation_code", Value::from_bytes(&validation_code)),
		("para_kind", Value::bool(true)),
	]);

	let inner = subxt::dynamic::tx(
		"ParasSudoWrapper",
		"sudo_schedule_para_initialize",
		vec![Value::u128(para_id as u128), genesis],
	);
	let sudo_call = subxt::dynamic::tx("Sudo", "sudo", vec![inner.into_value()]);

	// The relay's sudo key is Alice on a dev genesis; verified against `Sudo::Key` on chain.
	let signer = subxt_signer::sr25519::dev::alice();

	eprintln!("[stage] submitting sudoScheduleParaInitialize");
	let progress = client.tx().sign_and_submit_then_watch_default(&sudo_call, &signer).await?;
	progress.wait_for_finalized_success().await?;
	eprintln!("[stage] registration extrinsic finalized");

	// Onboarding takes effect at a session boundary, so the para list does not update immediately.
	for attempt in 1..=60 {
		let current = parachains(&rpc_client).await.unwrap_or_default();
		if current.contains(&para_id) {
			eprintln!("[ok] para {para_id} onboarded after {attempt} checks: {current:?}");
			return Ok(());
		}
		tokio::time::sleep(std::time::Duration::from_secs(3)).await;
	}

	Err(anyhow!("para {para_id} did not appear in Paras::Parachains within the timeout"))
}

/// `Paras::Parachains`, the ids currently onboarded as parachains.
async fn parachains(
	rpc_client: &subxt::ext::subxt_rpcs::RpcClient,
) -> Result<Vec<u32>, anyhow::Error> {
	let key = format!("0x{}", hex::encode(beefy_prover::PARAS_PARACHAINS));
	let raw: Option<Bytes> = rpc_client.request("state_getStorage", rpc_params![key]).await?;

	let Some(bytes) = raw else { return Ok(vec![]) };
	Ok(codec::Decode::decode(&mut &bytes.0[..])?)
}
