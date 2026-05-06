// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Parses the same toml the consolidated relayer consumes, but only the
//! pieces the collator-side fisherman needs: the `[hyperbridge]` table
//! (substrate connection + sr25519 signer) and each `[<chain>]` table
//! (deserialized into [`tesseract_config::AnyConfig`] via the canonical
//! tagged enum). `[<chain>.consensus]` sub-tables are stripped before
//! deserialization — this crate doesn't run consensus tasks, only the
//! byzantine watcher.
//!
//! Validation is non-negotiable: every Hyperbridge-supported L2 (per
//! [`tesseract_evm::registry::is_supported_l2`]) must be present and must
//! carry at least two `rpc_urls` so the byzantine handler has something to
//! quorum across.

use std::path::Path;

use anyhow::{anyhow, Context};
use ismp::host::StateMachine;
use tesseract_config::AnyConfig;
use tesseract_evm::registry::{
	is_supported_l2, SUPPORTED_L2_CHAIN_IDS_MAINNET, SUPPORTED_L2_CHAIN_IDS_TESTNET,
};
use tesseract_substrate::SubstrateConfig;
use toml::{Table, Value};

const HYPERBRIDGE: &str = "hyperbridge";
const RELAYER: &str = "relayer";
const CONSENSUS: &str = "consensus";
const MIN_RPC_URLS_PER_L2: usize = 2;

/// Subset of the relayer's `HyperbridgeConfig` used by the collator-side
/// fisherman. `chains` carries one [`AnyConfig`] per `[<chain>]` block in
/// the operator's toml — the same canonical type the relayer routes off of.
#[derive(Debug)]
pub struct FishermanConfig {
	pub hyperbridge: SubstrateConfig,
	pub chains: Vec<(String, AnyConfig)>,
}

impl FishermanConfig {
	/// Read the toml at `path`, parse out `[hyperbridge]` as a
	/// [`SubstrateConfig`] and each `[<chain>]` block as an [`AnyConfig`],
	/// and resolve every chain (`eth_chainId` / pallet lookups for
	/// `state_machine`, `ismp_host`, `consensus_state_id`).
	pub async fn parse(path: &Path) -> anyhow::Result<Self> {
		let toml_str = tokio::fs::read_to_string(path)
			.await
			.with_context(|| format!("reading tesseract config at {}", path.display()))?;
		let table: Table = toml_str.parse().context("parsing tesseract config as TOML")?;

		let hyperbridge = parse_hyperbridge_section(&table)?;

		let mut chains = Vec::new();
		for (name, raw) in &table {
			if name == HYPERBRIDGE || name == RELAYER {
				continue;
			}
			let Some(chain_table) = raw.as_table() else { continue };
			if !chain_table.contains_key("type") {
				continue;
			}
			// Strip the optional `[<chain>.consensus]` sub-table — fisherman
			// doesn't run consensus, and `AnyConfig` doesn't expect a
			// `consensus` field.
			let mut messaging_table = chain_table.clone();
			messaging_table.remove(CONSENSUS);

			let any: AnyConfig = Value::Table(messaging_table)
				.try_into()
				.with_context(|| format!("parsing chain section '{name}'"))?;
			let resolved = any
				.resolve()
				.await
				.with_context(|| format!("resolving chain section '{name}'"))?;
			chains.push((name.clone(), resolved));
		}

		Ok(Self { hyperbridge, chains })
	}

	/// Enforce the collator-side rules:
	///
	/// 1. `[hyperbridge].signer` is set (we need it to sign vetoes).
	/// 2. Every Hyperbridge-supported L2 has a configured chain section. Mainnet and testnet IDs
	///    are tracked separately; a config that contains *any* mainnet L2 must contain *all*
	///    mainnet L2s, and likewise for testnet — running a partial set is a deployment bug, not a
	///    knob.
	/// 3. Every L2 chain section has at least two `rpc_urls` so the byzantine handler has providers
	///    to quorum across.
	pub fn validate(&self) -> anyhow::Result<()> {
		if self.hyperbridge.signer.as_deref().unwrap_or("").trim().is_empty() {
			return Err(anyhow!(
				"[hyperbridge].signer is required (sr25519 seed of a registered collator account)"
			));
		}

		let mut configured_l2s: Vec<u64> = Vec::new();
		for (name, cfg) in &self.chains {
			let AnyConfig::Evm(evm) = cfg else { continue };
			let StateMachine::Evm(chain_id) = evm.state_machine() else { continue };
			if !is_supported_l2(chain_id as u64) {
				continue;
			}
			if evm.rpc_urls.len() < MIN_RPC_URLS_PER_L2 {
				return Err(anyhow!(
					"L2 chain '{name}' (chain_id {chain_id}) has only {} rpc_urls, need at least {} different RPC providers for quorum",
					evm.rpc_urls.len(),
					MIN_RPC_URLS_PER_L2,
				));
			}
			configured_l2s.push(chain_id as u64);
		}

		require_complete_set(&configured_l2s, SUPPORTED_L2_CHAIN_IDS_MAINNET, "mainnet")?;
		require_complete_set(&configured_l2s, SUPPORTED_L2_CHAIN_IDS_TESTNET, "testnet")?;
		Ok(())
	}
}

/// Parse `[hyperbridge]` directly into a `SubstrateConfig`. Mirrors the
/// relayer's logic but skips the consensus sub-table — the fisherman doesn't
/// run consensus.
fn parse_hyperbridge_section(table: &Table) -> anyhow::Result<SubstrateConfig> {
	let raw = table
		.get(HYPERBRIDGE)
		.cloned()
		.ok_or_else(|| anyhow!("missing [hyperbridge] section in tesseract config"))?;
	let Value::Table(mut hb_table) = raw else {
		return Err(anyhow!("[hyperbridge] must be a TOML table"));
	};
	hb_table.remove(CONSENSUS);
	let cfg: SubstrateConfig = Value::Table(hb_table)
		.try_into()
		.context("parsing [hyperbridge] section as SubstrateConfig")?;
	Ok(cfg)
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
