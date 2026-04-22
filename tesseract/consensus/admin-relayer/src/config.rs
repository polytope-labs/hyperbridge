// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use anyhow::anyhow;
use ismp::consensus::ConsensusStateId;
use serde::{Deserialize, Serialize};
use tesseract_beefy::backend::RedisConfig;
use tesseract_evm::EvmConfig;

/// How a mandatory consensus update should be delivered on a given chain.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionMode {
	/// EIP-7702 delegate the signer to a per-chain ERC-7821 Executor and then
	/// submit a single batched transaction `[unfreeze, handleConsensus, freeze]`.
	#[default]
	Batched,
	/// Submit three sequential transactions from a plain EOA: `setFrozenState(None)`,
	/// `handleConsensus`, then `setFrozenState(All)`. Use for chains whose RPC/VM
	/// does not yet support EIP-7702 (e.g. Soneium today).
	Sequential,
}

/// Per-chain config: inner `EvmConfig` fields plus the submission mode.
///
/// The EVM fields are flattened so an existing `[chains.<label>]` block continues
/// to work; `submission_mode` is optional and defaults to `batched`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
	#[serde(flatten)]
	pub evm: EvmConfig,
	#[serde(default)]
	pub submission_mode: SubmissionMode,
}

/// Top-level config for the admin-driven mandatory-consensus relayer.
///
/// Expected TOML layout:
/// ```toml
/// [hyperbridge]
/// consensus_state_id = [0x32, 0x29, 0x34, 0x29]  # "2A4A"
///
/// [hyperbridge.redis]
/// # RedisConfig fields
///
/// [chains.soneium]
/// # EvmConfig fields plus optionally:
/// submission_mode = "sequential"  # or "batched" (default)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminRelayerConfig {
	pub hyperbridge: HyperbridgeConfig,
	#[serde(default)]
	pub chains: HashMap<String, ChainConfig>,
}

/// Minimal Hyperbridge config for the admin relayer — only what's needed to
/// consume proofs out of the shared Redis queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperbridgeConfig {
	pub consensus_state_id: ConsensusStateId,
	pub redis: RedisConfig,
}

impl AdminRelayerConfig {
	pub async fn load(path: &str) -> anyhow::Result<Self> {
		let contents = tokio::fs::read_to_string(path)
			.await
			.map_err(|e| anyhow!("failed to read config {path}: {e}"))?;
		let cfg: Self =
			toml::from_str(&contents).map_err(|e| anyhow!("failed to parse config: {e}"))?;
		if cfg.chains.is_empty() {
			return Err(anyhow!("config must declare at least one chain under [chains.*]"));
		}
		Ok(cfg)
	}
}
