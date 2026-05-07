// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Validation of the consolidated relayer's [`HyperbridgeConfig`] for use by
//! the collator-side fisherman. The toml is parsed by
//! [`tesseract::config::HyperbridgeConfig::parse_conf`] — there's no
//! parallel schema here. This module only encodes the rules that the
//! collator path layers on top: a signer must be set, every supported L2 in
//! [`tesseract_evm::registry`] must be present, and every L2 needs at least
//! two `rpc_urls` so the byzantine handler has providers to quorum across.
//!
//! Rules are deliberately stricter than the relayer's: running fisherman is
//! not optional for collators.

use anyhow::anyhow;
use ismp::host::StateMachine;
use tesseract::config::HyperbridgeConfig;
use tesseract_config::AnyConfig;
use tesseract_evm::registry::{
	is_supported_l2, SUPPORTED_L2_CHAIN_IDS_MAINNET, SUPPORTED_L2_CHAIN_IDS_TESTNET,
};

const MIN_RPC_URLS_PER_L2: usize = 2;

/// Enforce the collator-side rules. Returns the first violation as `Err`.
pub fn validate(config: &HyperbridgeConfig) -> anyhow::Result<()> {
	if config.hyperbridge.substrate.signer.as_deref().unwrap_or("").trim().is_empty() {
		return Err(anyhow!(
			"[hyperbridge].signer is required (sr25519 seed of a registered collator account)"
		));
	}

	let mut configured_l2s: Vec<u64> = Vec::new();
	for (state_machine, per_chain) in &config.chains {
		let AnyConfig::Evm(evm) = &per_chain.messaging else { continue };
		let StateMachine::Evm(chain_id) = state_machine else { continue };
		if !is_supported_l2(*chain_id as u64) {
			continue;
		}
		if evm.rpc_urls.len() < MIN_RPC_URLS_PER_L2 {
			return Err(anyhow!(
				"L2 chain (chain_id {chain_id}) has only {} rpc_urls, need at least {} different RPC providers for quorum",
				evm.rpc_urls.len(),
				MIN_RPC_URLS_PER_L2,
			));
		}
		configured_l2s.push(*chain_id as u64);
	}

	require_complete_set(&configured_l2s, SUPPORTED_L2_CHAIN_IDS_MAINNET, "mainnet")?;
	require_complete_set(&configured_l2s, SUPPORTED_L2_CHAIN_IDS_TESTNET, "testnet")?;
	Ok(())
}

/// If any chain in `set` is configured, all of them must be configured. A
/// completely empty intersection is fine (operator runs the other set).
fn require_complete_set(configured: &[u64], set: &[u64], label: &str) -> anyhow::Result<()> {
	let any_present = set.iter().any(|c| configured.contains(c));
	if !any_present {
		return Ok(());
	}
	let missing: Vec<u64> = set.iter().copied().filter(|c| !configured.contains(c)).collect();
	if !missing.is_empty() {
		return Err(anyhow!(
			"{label} L2 coverage is partial — missing chain_ids {missing:?}. Running fisherman requires all supported L2s to be configured"
		));
	}
	Ok(())
}
