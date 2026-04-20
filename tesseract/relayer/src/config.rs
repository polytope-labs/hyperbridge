// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Consolidated relayer config. Shape:
//! - `[hyperbridge]`         — HB host (substrate RPC, prover)
//! - `[<chain-name>]`        — per-chain [`AnyConfig`]; one block per chain
//! - `[relayer]`             — operator knobs
//!
//! `delivery_endpoints` scopes inbound messaging; `consensus_chains` scopes
//! inbound consensus; `outbound` toggles the HB → chain fan-out.

use anyhow::anyhow;
use ismp::host::StateMachine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tesseract_consensus::any::{AnyConfig, HyperbridgeHostConfig};
use toml::Table;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
	// -- Messaging (inbound) --
	pub module_filter: Option<Vec<String>>,
	#[serde(default)]
	pub minimum_profit_percentage: u32,
	#[serde(default)]
	pub delivery_endpoints: Vec<String>,
	pub fisherman: Option<bool>,
	pub withdrawal_frequency: Option<u64>,
	pub minimum_withdrawal_amount: Option<u64>,
	pub unprofitable_retry_frequency: Option<u64>,
	pub deliver_failed: Option<bool>,
	pub disable_fee_accumulation: Option<bool>,

	/// Chains (by state machine string, e.g. `"EVM-1"`) to run inbound consensus
	/// for. `None` / empty → run for every configured chain. Use this to opt
	/// individual chains in or out of the consensus-relaying cost.
	pub consensus_chains: Option<Vec<String>>,

	/// If `true` (the default), spawn the outbound (HB → destination) task that
	/// drives off pallet-beefy-consensus-proofs `ProofAccepted` events.
	#[serde(default = "default_true")]
	pub outbound: bool,
}

fn default_true() -> bool {
	true
}

impl Default for RelayerConfig {
	fn default() -> Self {
		Self {
			module_filter: None,
			minimum_profit_percentage: 0,
			delivery_endpoints: Vec::new(),
			fisherman: None,
			withdrawal_frequency: None,
			minimum_withdrawal_amount: None,
			unprofitable_retry_frequency: None,
			deliver_failed: None,
			disable_fee_accumulation: None,
			consensus_chains: None,
			outbound: true,
		}
	}
}

impl RelayerConfig {
	/// Returns true iff inbound consensus should run for `state_machine`.
	pub fn inbound_consensus_enabled(&self, state_machine: &StateMachine) -> bool {
		match &self.consensus_chains {
			None => true,
			Some(list) if list.is_empty() => true,
			Some(list) => list.iter().any(|s| s == &state_machine.to_string()),
		}
	}
}

impl From<RelayerConfig> for tesseract_primitives::config::RelayerConfig {
	fn from(config: RelayerConfig) -> Self {
		tesseract_primitives::config::RelayerConfig {
			module_filter: config.module_filter,
			minimum_profit_percentage: config.minimum_profit_percentage,
			withdrawal_frequency: config.withdrawal_frequency,
			minimum_withdrawal_amount: config.minimum_withdrawal_amount,
			unprofitable_retry_frequency: config.unprofitable_retry_frequency,
			delivery_endpoints: config.delivery_endpoints,
			deliver_failed: config.deliver_failed,
			fisherman: config.fisherman,
			disable_fee_accumulation: config.disable_fee_accumulation,
		}
	}
}

pub struct HyperbridgeConfig {
	pub hyperbridge: HyperbridgeHostConfig,
	pub chains: HashMap<StateMachine, AnyConfig>,
	pub relayer: RelayerConfig,
}

const HYPERBRIDGE: &str = "hyperbridge";
const RELAYER: &str = "relayer";

impl HyperbridgeConfig {
	pub async fn parse_conf(config: &str) -> Result<Self, anyhow::Error> {
		let toml_str = tokio::fs::read_to_string(config)
			.await
			.map_err(|err| anyhow!("Error reading config file: {err:?}"))?;
		let table = toml_str.parse::<Table>()?;

		if !table.contains_key(HYPERBRIDGE) || !table.contains_key(RELAYER) {
			return Err(anyhow!("Missing [hyperbridge] or [relayer] section in config"));
		}

		let hyperbridge: HyperbridgeHostConfig =
			table.get(HYPERBRIDGE).cloned().expect("checked above").try_into()?;

		let relayer: RelayerConfig =
			table.get(RELAYER).cloned().expect("checked above").try_into()?;

		let mut chains = HashMap::new();
		for (key, val) in &table {
			if key != HYPERBRIDGE && key != RELAYER {
				let any_conf: AnyConfig = val.clone().try_into()?;
				chains.insert(any_conf.state_machine(), any_conf);
			}
		}

		Ok(Self { hyperbridge, chains, relayer })
	}

	/// Build the consensus relayer's config for create_client_map.
	pub fn consensus_config(&self) -> tesseract_consensus::config::HyperbridgeConfig {
		tesseract_consensus::config::HyperbridgeConfig {
			hyperbridge: self.hyperbridge.clone(),
			chains: self.chains.clone(),
			relayer: None,
		}
	}
}
