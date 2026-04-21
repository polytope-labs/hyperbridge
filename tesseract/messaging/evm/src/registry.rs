// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0.

//! Registry of known Hyperbridge EVM deployments, mirroring the source-of-truth
//! `chainConfigs` table in the TS SDK (`sdk/packages/sdk/src/configs/chain.ts`).
//!
//! Used by the consolidated relayer to auto-derive `ismp_host` from the chain's
//! `eth_chainId` when the user hasn't specified one explicitly.

use alloy::providers::{Provider, RootProvider};
use anyhow::{anyhow, Context};
use primitive_types::H160;
use std::str::FromStr;

/// Returns the canonical Hyperbridge `IsmpHost` contract address for the given
/// EVM chain ID, or `None` if the chain isn't a known Hyperbridge deployment.
///
/// Addresses here must stay in sync with the TS SDK at
/// `sdk/packages/sdk/src/configs/chain.ts`.
pub fn ismp_host_for_chain_id(chain_id: u64) -> Option<H160> {
	let addr = match chain_id {
		// Testnets
		97 => "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7", // BSC Chapel
		10200 => "0x58a41b89f4871725e5d898d98ef4bf917601c5eb", // Gnosis Chiado
		11155111 => "0x2EdB74C269948b60ec1000040E104cef0eABaae8", // Sepolia
		80002 => "0x9a2840D050e64Db89c90Ac5857536E4ec66641DE", // Polygon Amoy
		421614 => "0x3435bD7e5895356535459D6087D1eB982DAd90e7", // Arbitrum Sepolia
		11155420 => "0x6d51b678836d8060d980605d2999eF211809f3C2", // Optimism Sepolia
		84532 => "0xD198c01839dd4843918617AfD1e4DDf44Cc3BB4a", // Base Sepolia
		420420417 => "0xbb26e04a71e7c12093e82b83ba310163eac186fa", // Polkadot Asset Hub Paseo (Revive)
		688689 => "0xED54E9b64043c389173316B6351Bd25491060eA8", // Pharos Atlantic

		// Mainnets
		1 => "0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20", // Ethereum
		56 => "0x24B5d421Ec373FcA57325dd2F0C074009Af021F7", // BSC
		42161 => "0xE05AFD4Eb2ce6d65c40e1048381BD0Ef8b4B299e", // Arbitrum
		8453 => "0x6FFe92e4d7a9D589549644544780e6725E84b248", // Base
		137 => "0xD8d3db17C1dF65b301D45C84405CcAC1395C559a", // Polygon
		130 => "0x2A17C1c3616Bbc33FCe5aF5B965F166ba76cEDAf", // Unichain
		10 => "0x78c8A5F27C06757EA0e30bEa682f1FD5C8d7645d", // Optimism
		100 => "0x50c236247447B9d4Ee0561054ee596fbDa7791b1", // Gnosis
		1868 => "0x7F0165140D0f3251c8f6465e94E9d12C7FD40711", // Soneium

		_ => return None,
	};
	H160::from_str(addr).ok()
}

/// Fetches the chain's numeric ID via `eth_chainId` against the first RPC URL
/// in the list. Used by the consolidated relayer to auto-derive a chain's
/// `state_machine` identifier.
pub async fn fetch_chain_id(rpc_url: &str) -> anyhow::Result<u64> {
	let url = rpc_url
		.parse::<alloy::transports::http::reqwest::Url>()
		.with_context(|| format!("invalid RPC URL: {rpc_url}"))?;
	let provider: RootProvider = RootProvider::new_http(url);
	provider
		.get_chain_id()
		.await
		.map_err(|err| anyhow!("eth_chainId({rpc_url}) failed: {err}"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn known_mainnets_resolve() {
		assert!(ismp_host_for_chain_id(1).is_some(), "ethereum mainnet");
		assert!(ismp_host_for_chain_id(56).is_some(), "bsc");
		assert!(ismp_host_for_chain_id(8453).is_some(), "base");
	}

	#[test]
	fn known_testnets_resolve() {
		assert!(ismp_host_for_chain_id(97).is_some(), "bsc chapel");
		assert!(ismp_host_for_chain_id(11155111).is_some(), "sepolia");
		assert!(ismp_host_for_chain_id(84532).is_some(), "base sepolia");
	}

	#[test]
	fn unknown_chain_returns_none() {
		assert!(ismp_host_for_chain_id(9999).is_none());
		assert!(ismp_host_for_chain_id(0).is_none());
	}
}
