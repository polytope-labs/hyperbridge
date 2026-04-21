// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0.

//! Auto-derivation helpers for substrate chains.
//!
//! Given just a WebSocket RPC URL, queries `system_chain` + ParachainInfo
//! storage and produces the canonical ISMP [`StateMachine`] variant. Used by
//! the consolidated relayer to spare operators from writing `state_machine =
//! "POLKADOT-1000"` when the RPC already knows.

use anyhow::{anyhow, Context};
use ismp::host::StateMachine;
use subxt::{
	backend::rpc::RpcClient,
	ext::subxt_rpcs::{client::reconnecting_rpc_client::RpcClientBuilder, rpc_params},
};

/// twox_128("ParachainInfo") || twox_128("ParachainId") — the standard cumulus
/// parachain id storage key. Constant across every parachain that carries the
/// `parachain-info` pallet (i.e. every cumulus parachain).
const PARACHAIN_ID_STORAGE_KEY: &str =
	"0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f";

/// Resolves the ISMP [`StateMachine`] for a substrate chain by querying its
/// WebSocket RPC. Works for cumulus parachains whose chain name starts with
/// the relay chain's name (Polkadot / Kusama / Paseo) and for the known
/// Hyperbridge testnet. Other chains should set `state_machine` explicitly.
pub async fn fetch_state_machine(rpc_ws: &str) -> anyhow::Result<StateMachine> {
	let reconnecting = RpcClientBuilder::new()
		.max_request_size(4 * 1024 * 1024)
		.max_response_size(4 * 1024 * 1024)
		.build(rpc_ws.to_string())
		.await
		.with_context(|| format!("failed to connect to substrate RPC {rpc_ws}"))?;
	let rpc = RpcClient::new(reconnecting);

	let chain: String = rpc
		.request("system_chain", rpc_params![])
		.await
		.with_context(|| format!("system_chain({rpc_ws})"))?;

	// Parachain id is absent on relay chains; ok() folds the "storage key
	// doesn't exist on this chain" case into `None`.
	let para_bytes: Option<String> = rpc
		.request("state_getStorage", rpc_params![PARACHAIN_ID_STORAGE_KEY])
		.await
		.ok()
		.flatten();
	let para_id = para_bytes.as_deref().and_then(parse_para_id);

	state_machine_from_chain_name(&chain, para_id).ok_or_else(|| {
		anyhow!(
			"cannot auto-derive StateMachine for substrate chain '{chain}' (para_id={para_id:?}); \
			 set `state_machine` explicitly in the chain's config block"
		)
	})
}

fn parse_para_id(hex_str: &str) -> Option<u32> {
	let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
	let bytes = hex::decode(stripped).ok()?;
	if bytes.len() < 4 {
		return None;
	}
	let mut arr = [0u8; 4];
	arr.copy_from_slice(&bytes[..4]);
	Some(u32::from_le_bytes(arr))
}

/// Map `system_chain` name → [`StateMachine`] variant.
///
/// Heuristic based on the chain name's prefix. Ambiguous cases (e.g. a
/// parachain called "Bifrost" that runs on both Polkadot and Kusama) can't
/// be resolved from the parachain's own RPC alone, so those return `None`
/// and the caller errors out with a suggestion to set `state_machine`
/// explicitly.
fn state_machine_from_chain_name(name: &str, para_id: Option<u32>) -> Option<StateMachine> {
	let lower = name.to_lowercase();
	let pid = para_id?; // no para_id → relay/standalone chain; needs explicit config

	if lower.starts_with("polkadot") {
		return Some(StateMachine::Polkadot(pid));
	}
	if lower.starts_with("kusama") {
		return Some(StateMachine::Kusama(pid));
	}
	if lower.starts_with("paseo") {
		return Some(StateMachine::Relay { relay: *b"PAS0", para_id: pid });
	}
	// Hyperbridge testnet is hosted on Kusama (Gargantua); mainnet on Polkadot.
	if lower.starts_with("hyperbridge gargantua") {
		return Some(StateMachine::Kusama(pid));
	}
	if lower.starts_with("hyperbridge") {
		return Some(StateMachine::Polkadot(pid));
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn para_id_parses_little_endian_u32() {
		// 1000 in LE bytes = [0xe8, 0x03, 0x00, 0x00]
		assert_eq!(parse_para_id("0xe803000000"), Some(1000));
		// Extra bytes ignored
		assert_eq!(parse_para_id("0xe8030000deadbeef"), Some(1000));
		// Without 0x prefix
		assert_eq!(parse_para_id("e8030000"), Some(1000));
	}

	#[test]
	fn para_id_rejects_short_input() {
		assert_eq!(parse_para_id("0xe8"), None);
		assert_eq!(parse_para_id(""), None);
	}

	#[test]
	fn chain_name_maps_polkadot_parachain() {
		assert!(matches!(
			state_machine_from_chain_name("Polkadot Asset Hub", Some(1000)),
			Some(StateMachine::Polkadot(1000))
		));
		assert!(matches!(
			state_machine_from_chain_name("polkadot bridge hub", Some(1002)),
			Some(StateMachine::Polkadot(1002))
		));
	}

	#[test]
	fn chain_name_maps_kusama_parachain() {
		assert!(matches!(
			state_machine_from_chain_name("Kusama Asset Hub", Some(1000)),
			Some(StateMachine::Kusama(1000))
		));
	}

	#[test]
	fn chain_name_maps_paseo_parachain_to_relay_variant() {
		match state_machine_from_chain_name("Paseo Asset Hub", Some(1000)) {
			Some(StateMachine::Relay { relay, para_id }) => {
				assert_eq!(&relay, b"PAS0");
				assert_eq!(para_id, 1000);
			},
			other => panic!("expected Relay variant, got {other:?}"),
		}
	}

	#[test]
	fn chain_name_maps_hyperbridge_gargantua_to_kusama() {
		assert!(matches!(
			state_machine_from_chain_name("Hyperbridge Gargantua", Some(4009)),
			Some(StateMachine::Kusama(4009))
		));
	}

	#[test]
	fn chain_name_maps_hyperbridge_mainnet_to_polkadot() {
		assert!(matches!(
			state_machine_from_chain_name("Hyperbridge", Some(3367)),
			Some(StateMachine::Polkadot(3367))
		));
	}

	#[test]
	fn missing_para_id_returns_none() {
		// Relay chains or standalone chains can't be auto-derived by this
		// heuristic — user must set `state_machine` explicitly.
		assert!(state_machine_from_chain_name("Polkadot", None).is_none());
		assert!(state_machine_from_chain_name("Kusama", None).is_none());
	}

	#[test]
	fn unknown_prefix_returns_none() {
		assert!(state_machine_from_chain_name("Random Parachain", Some(2000)).is_none());
	}
}
