// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0.

//! Auto-derivation helpers for substrate chains.
//!
//! Given just a WebSocket RPC URL, asks the chain's runtime for its canonical
//! ISMP [`StateMachine`] via the `IsmpRuntimeApi::host_state_machine` runtime
//! API. Used by the consolidated relayer to spare operators from writing
//! `state_machine = "POLKADOT-1000"` when the runtime already knows.
//!
//! Requires the chain to expose `pallet-ismp` with its runtime API. Chains
//! without ISMP cannot be auto-derived and must set `state_machine` explicitly.

use anyhow::{anyhow, Context};
use codec::Decode;
use ismp::host::StateMachine;
use subxt::{
	backend::rpc::RpcClient,
	ext::subxt_rpcs::{client::reconnecting_rpc_client::RpcClientBuilder, rpc_params},
};

/// Runtime API method name. Substrate's `state_call` convention is
/// `<TraitName>_<method_name>` (no parameters for this call).
const HOST_STATE_MACHINE_CALL: &str = "IsmpRuntimeApi_host_state_machine";

/// Resolves the ISMP [`StateMachine`] for a substrate chain by invoking
/// `IsmpRuntimeApi::host_state_machine` over the chain's JSON-RPC `state_call`.
/// This is the canonical source — the runtime itself declares which state
/// machine it represents — so it sidesteps all the ambiguities of
/// chain-name-based heuristics (e.g. parachains that run on both Polkadot and
/// Kusama under the same name).
pub async fn fetch_state_machine(rpc_ws: &str) -> anyhow::Result<StateMachine> {
	let reconnecting = RpcClientBuilder::new()
		.max_request_size(4 * 1024 * 1024)
		.max_response_size(4 * 1024 * 1024)
		.build(rpc_ws.to_string())
		.await
		.with_context(|| format!("failed to connect to substrate RPC {rpc_ws}"))?;
	let rpc = RpcClient::new(reconnecting);

	// `state_call(method, data, block_hash_opt)`:
	//   - method = "<Trait>_<fn>"
	//   - data   = hex-encoded SCALE-encoded args ("0x" for no args)
	//   - block  = None → use the latest known block
	let result_hex: String = rpc
		.request("state_call", rpc_params![HOST_STATE_MACHINE_CALL, "0x"])
		.await
		.with_context(|| {
			format!(
				"state_call({HOST_STATE_MACHINE_CALL}) failed — does this chain expose pallet-ismp's runtime API?"
			)
		})?;

	decode_state_machine(&result_hex).with_context(|| {
		format!("failed to decode StateMachine from runtime API response `{result_hex}`")
	})
}

fn decode_state_machine(hex_bytes: &str) -> anyhow::Result<StateMachine> {
	let stripped = hex_bytes.strip_prefix("0x").unwrap_or(hex_bytes);
	let bytes = hex::decode(stripped).with_context(|| "runtime API result was not valid hex")?;
	StateMachine::decode(&mut &bytes[..])
		.map_err(|err| anyhow!("SCALE decode StateMachine: {err:?}"))
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;

	#[test]
	fn decodes_polkadot_parachain() {
		let encoded = StateMachine::Polkadot(1000).encode();
		let hex = format!("0x{}", hex::encode(encoded));
		assert_eq!(decode_state_machine(&hex).unwrap(), StateMachine::Polkadot(1000));
	}

	#[test]
	fn decodes_kusama_parachain() {
		let encoded = StateMachine::Kusama(4009).encode();
		let hex = format!("0x{}", hex::encode(encoded));
		assert_eq!(decode_state_machine(&hex).unwrap(), StateMachine::Kusama(4009));
	}

	#[test]
	fn decodes_relay_variant() {
		let sm = StateMachine::Relay { relay: *b"PAS0", para_id: 1000 };
		let hex = format!("0x{}", hex::encode(sm.encode()));
		assert_eq!(decode_state_machine(&hex).unwrap(), sm);
	}

	#[test]
	fn decode_tolerates_missing_0x_prefix() {
		let encoded = StateMachine::Polkadot(1000).encode();
		let hex = hex::encode(encoded);
		assert_eq!(decode_state_machine(&hex).unwrap(), StateMachine::Polkadot(1000));
	}

	#[test]
	fn decode_rejects_invalid_hex() {
		assert!(decode_state_machine("0xnothex").is_err());
	}

	#[test]
	fn decode_rejects_short_input() {
		assert!(decode_state_machine("0x00").is_err());
	}
}
