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

/// Returns the messaging-side `consensus_state_id` for the given EVM
/// chain on Hyperbridge, or `None` if the chain isn't known.
///
/// This is the id messaging uses to locate the chain's state on
/// Hyperbridge. For any chain finalized through Ethereum (every L2
/// plus chains that "track Ethereum") that id is `"ETH0"`, not the
/// chain's own consensus-task id. The chain-specific ids (`"ARB0"`,
/// `"OPT0"`, `"BASE"`, `"UNI0"`, `"SON0"`) belong to the consensus
/// client and are sourced from the `[<chain>.consensus]` host config,
/// never from this registry.
pub fn consensus_state_id_for_chain_id(chain_id: u64) -> Option<&'static str> {
	let id = match chain_id {
		// Testnets.
		97 => "BSC0",        // BSC Chapel
		10200 => "GNO0",     // Gnosis Chiado
		11155111 => "ETH0",  // Sepolia
		80002 => "POLY",     // Polygon Amoy
		421614 => "ETH0",    // Arbitrum Sepolia (L2 of Sepolia)
		11155420 => "ETH0",  // Optimism Sepolia (L2 of Sepolia)
		84532 => "ETH0",     // Base Sepolia (L2 of Sepolia)
		420420417 => "PAS0", // Polkadot Asset Hub Paseo (Revive), finalised by Paseo relay
		688689 => "PHAR",    // Pharos Atlantic

		// Mainnets.
		1 => "ETH0",     // Ethereum
		56 => "BSC0",    // BSC
		42161 => "ETH0", // Arbitrum (L2 of Ethereum)
		8453 => "ETH0",  // Base (L2 of Ethereum)
		137 => "POLY",   // Polygon
		130 => "ETH0",   // Unichain (L2 of Ethereum)
		10 => "ETH0",    // Optimism (L2 of Ethereum)
		100 => "GNO0",   // Gnosis
		1868 => "ETH0",  // Soneium (L2 of Ethereum)

		_ => return None,
	};
	Some(id)
}

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

/// EVM chain IDs that Hyperbridge treats as L2 rollups of Ethereum. These are
/// the chains the collator-side fisherman task is required to monitor, and the
/// canonical source of truth used by the wrapper to enforce coverage of the
/// `[<chain>]` sections in the operator's tesseract toml.
///
/// Excludes Ethereum L1 itself, plus chains not finalized through Ethereum
/// (BSC, Gnosis, Polygon, Pharos), and Polkadot-finalized chains.
pub const SUPPORTED_L2_CHAIN_IDS_MAINNET: &[u64] = &[
	42161, // Arbitrum
	8453,  // Base
	10,    // Optimism
	1868,  // Soneium
];

/// Testnet counterparts of [`SUPPORTED_L2_CHAIN_IDS_MAINNET`]. Listed
/// separately so a collator pointed at a testnet deployment doesn't need
/// mainnet entries (and vice-versa). The collator-side fisherman currently
/// supports only Arbitrum Sepolia and Base Sepolia on the L2 side
/// (Optimism Sepolia / Unichain Sepolia / Soneium testnet aren't covered).
pub const SUPPORTED_L2_CHAIN_IDS_TESTNET: &[u64] = &[
	421614, // Arbitrum Sepolia
	84532,  // Base Sepolia
];

/// Non-L2 EVM chain IDs that Hyperbridge tracks directly (each has its own consensus client,
/// not rolled up to an L1). The collator-side fisherman config validation requires every one
/// of these to be present alongside the L2 set so the messaging path has counterparties on
/// each chain we settle commitments against.
pub const SUPPORTED_NON_L2_CHAIN_IDS_MAINNET: &[u64] = &[
	1,   // Ethereum
	56,  // BNB Smart Chain
	100, // Gnosis
	137, // Polygon
];

/// Testnet counterparts of [`SUPPORTED_NON_L2_CHAIN_IDS_MAINNET`]. The collator-side
/// fisherman covers only Sepolia on testnet — BSC Chapel, Gnosis Chiado and Polygon Amoy
/// aren't required because Hyperbridge's testnet deployment settles those rollups against
/// Sepolia only.
pub const SUPPORTED_NON_L2_CHAIN_IDS_TESTNET: &[u64] = &[
	11155111, // Sepolia
];

/// True when `chain_id` is a Hyperbridge-supported L2 (mainnet or testnet).
pub fn is_supported_l2(chain_id: u64) -> bool {
	SUPPORTED_L2_CHAIN_IDS_MAINNET.contains(&chain_id) ||
		SUPPORTED_L2_CHAIN_IDS_TESTNET.contains(&chain_id)
}

/// True when `chain_id` is a Hyperbridge-supported non-L2 EVM chain (mainnet or testnet).
pub fn is_supported_non_l2(chain_id: u64) -> bool {
	SUPPORTED_NON_L2_CHAIN_IDS_MAINNET.contains(&chain_id) ||
		SUPPORTED_NON_L2_CHAIN_IDS_TESTNET.contains(&chain_id)
}

/// Union of [`is_supported_l2`] and [`is_supported_non_l2`].
pub fn is_supported_chain(chain_id: u64) -> bool {
	is_supported_l2(chain_id) || is_supported_non_l2(chain_id)
}

/// True for Arbitrum-family L2s (`arbitrum_orbit` consensus). Used by the collator-side
/// fisherman to validate that the operator wired the expected consensus client kind per chain.
pub fn is_arbitrum_l2(chain_id: u64) -> bool {
	matches!(chain_id, 42161 | 421614)
}

/// True for OP-Stack-family L2s (`op_stack` consensus): Base, Optimism, Soneium, and their
/// testnets.
pub fn is_opstack_l2(chain_id: u64) -> bool {
	is_supported_l2(chain_id) && !is_arbitrum_l2(chain_id)
}

/// The expected tesseract consensus client kind for a supported L2. Returns `None` for any
/// other chain — non-L2 chains don't require a `[<chain>.consensus]` block on the collator
/// side, only the L2s do (the on-chain rollup-claim fisherman needs the rollup-core /
/// dispute-game factory addresses that live in those L2 consensus configs).
pub fn expected_consensus_kind(chain_id: u64) -> Option<&'static str> {
	if is_arbitrum_l2(chain_id) {
		Some("arbitrum_orbit")
	} else if is_opstack_l2(chain_id) {
		Some("op_stack")
	} else {
		None
	}
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
