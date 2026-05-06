// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Toml view of the relayer's `HyperbridgeConfig` that the fisherman task
//! consumes, plus validation rules and the L2-rollup discriminator.
//!
//! Only a subset of the relayer toml is read. Hand-mapped types are used
//! instead of importing `tesseract_consensus_config::AnyConfig` to keep the
//! collator's dep graph lean (the canonical enum drags in the entire
//! consensus-client family). The match list in [`is_l2`] is the manual
//! sync point; if a new rollup variant is added to the relayer registry,
//! it must also be added here.

use std::{collections::BTreeMap, path::Path};

use anyhow::{anyhow, Context};
use serde::Deserialize;

const MIN_RPC_URLS_PER_L2: usize = 2;

#[derive(Debug, Deserialize)]
pub struct FishermanConfig {
	pub hyperbridge: HyperbridgeSection,
	#[serde(skip)]
	pub chains: BTreeMap<String, ChainSection>,
}

#[derive(Debug, Deserialize)]
pub struct HyperbridgeSection {
	pub rpc_ws: String,
	/// Hex-encoded sr25519 seed of a registered collator account.
	pub signer: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChainSection {
	#[serde(rename = "type")]
	pub host_type: String,
	#[serde(default)]
	pub rpc_urls: Vec<String>,
	pub consensus: Option<ConsensusSection>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConsensusSection {
	#[serde(rename = "type")]
	pub consensus_type: String,
}

impl FishermanConfig {
	pub async fn parse(path: &Path) -> anyhow::Result<Self> {
		let toml_str = tokio::fs::read_to_string(path)
			.await
			.with_context(|| format!("reading tesseract config at {}", path.display()))?;

		let table: toml::Value = toml_str.parse().context("parsing tesseract config as TOML")?;
		let table = table
			.as_table()
			.ok_or_else(|| anyhow!("config must be a top-level TOML table"))?;

		let hyperbridge_value = table
			.get("hyperbridge")
			.ok_or_else(|| anyhow!("missing [hyperbridge] section in tesseract config"))?;
		let hyperbridge: HyperbridgeSection = hyperbridge_value.clone().try_into().context(
			"parsing [hyperbridge] section (need rpc_ws and signer for the fisherman task)",
		)?;

		let mut chains = BTreeMap::new();
		for (name, value) in table {
			if name == "hyperbridge" || name == "relayer" {
				continue;
			}
			let Some(chain_table) = value.as_table() else { continue };
			// Skip top-level tables that aren't chain sections (any TOML key the
			// relayer accepts but doesn't shape like `[<chain>]`).
			if !chain_table.contains_key("type") {
				continue;
			}
			let chain: ChainSection = value
				.clone()
				.try_into()
				.with_context(|| format!("parsing chain section '{name}'"))?;
			chains.insert(name.clone(), chain);
		}

		Ok(Self { hyperbridge, chains })
	}

	pub fn validate(&self) -> anyhow::Result<()> {
		if self.hyperbridge.signer.trim().is_empty() {
			return Err(anyhow!(
				"[hyperbridge].signer is required when running fisherman (sr25519 seed of a registered collator account)"
			));
		}
		for (name, chain) in &self.chains {
			if chain.host_type != "evm" {
				continue;
			}
			let Some(consensus) = &chain.consensus else { continue };
			if !is_l2(&consensus.consensus_type) {
				continue;
			}
			if chain.rpc_urls.len() < MIN_RPC_URLS_PER_L2 {
				return Err(anyhow!(
					"L2 chain '{name}' (consensus type '{}') has only {} rpc_urls, need at least {} different RPC providers for quorum",
					consensus.consensus_type,
					chain.rpc_urls.len(),
					MIN_RPC_URLS_PER_L2,
				));
			}
		}
		Ok(())
	}
}

/// True for Ethereum L2 rollups. L1s and sidechains are excluded since the
/// L2-quorum semantics only apply to rollups. Discriminating by toml type
/// rather than chain-id keeps adding new OP Stack / Orbit chains in the
/// relayer registry from touching this crate.
pub fn is_l2(consensus_type: &str) -> bool {
	matches!(consensus_type, "op_stack" | "arbitrum_orbit")
}
