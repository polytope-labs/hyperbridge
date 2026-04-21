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
//!
//! ```toml
//! [hyperbridge]                          # HB host (substrate RPC, prover)
//! ...
//!
//! [relayer]                              # operator knobs + outbound toggle
//! delivery_endpoints = ["EVM-1"]
//! outbound = true
//!
//! [<chain-name>]                         # one block per chain — messaging/host config
//! type = "evm"
//! rpc_urls = ["https://..."]
//! state_machine = "EVM-1"
//! ismp_host = "0x..."
//! signer = "${SIG}"
//! consensus_state_id = "ETH0"
//!
//! [<chain-name>.consensus]               # OPTIONAL — presence opts into inbound consensus.
//! type = "ethereum"                      # consensus-side client type
//! host = { ... }                         # consensus-only knobs
//! # Host fields (rpc_urls, state_machine, ismp_host, signer, consensus_state_id, ...)
//! # are inherited from the parent and do not need to be re-specified.
//! ```
//!
//! `delivery_endpoints` scopes inbound messaging; presence of `[chain.consensus]`
//! is the sole signal to spawn inbound consensus for that chain; `outbound`
//! toggles the HB → chain fan-out.

use anyhow::{anyhow, Context};
use ismp::host::StateMachine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tesseract_config::AnyConfig as MessagingConfig;
use tesseract_consensus::any::{AnyConfig as ConsensusConfig, HyperbridgeHostConfig};
use toml::{Table, Value};

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
			outbound: true,
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

/// Per-chain configuration in the consolidated relayer.
///
/// `messaging` is always present — it's the host config used for inbound
/// messaging and outbound submission. `consensus` is optional: when present,
/// that chain also runs an inbound-consensus task.
#[derive(Debug, Clone)]
pub struct PerChainConfig {
	pub messaging: MessagingConfig,
	pub consensus: Option<ConsensusConfig>,
}

pub struct HyperbridgeConfig {
	pub hyperbridge: HyperbridgeHostConfig,
	pub chains: HashMap<StateMachine, PerChainConfig>,
	pub relayer: RelayerConfig,
}

const HYPERBRIDGE: &str = "hyperbridge";
const RELAYER: &str = "relayer";
const CONSENSUS: &str = "consensus";

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
		for (name, raw) in &table {
			if name == HYPERBRIDGE || name == RELAYER {
				continue;
			}
			let chain_table = raw.as_table().ok_or_else(|| {
				anyhow!("chain '{name}' must be a TOML table, got {}", raw.type_str())
			})?;
			let per_chain = parse_chain(name, chain_table)?;
			let state_machine = per_chain.messaging.state_machine();
			if chains.insert(state_machine, per_chain).is_some() {
				return Err(anyhow!("duplicate chain configured for state machine {state_machine}"));
			}
		}

		Ok(Self { hyperbridge, chains, relayer })
	}

	/// Build the consensus relayer's config view, containing only the subset of
	/// chains that opted into inbound consensus via `[<chain>.consensus]`.
	pub fn consensus_config(&self) -> tesseract_consensus::config::HyperbridgeConfig {
		let chains = self
			.chains
			.iter()
			.filter_map(|(sm, pc)| pc.consensus.clone().map(|c| (*sm, c)))
			.collect();

		tesseract_consensus::config::HyperbridgeConfig {
			hyperbridge: self.hyperbridge.clone(),
			chains,
			relayer: None,
		}
	}
}

/// Parse one `[<chain>]` block into a [`PerChainConfig`]. The base fields feed
/// the messaging [`MessagingConfig`]; if a `[<chain>.consensus]` sub-table is
/// present, the base fields are inherited into the consensus table before it's
/// deserialized as a [`ConsensusConfig`] (so host essentials are specified once).
fn parse_chain(name: &str, chain_table: &Table) -> Result<PerChainConfig, anyhow::Error> {
	let mut messaging_table = chain_table.clone();
	let consensus_value = messaging_table.remove(CONSENSUS);

	let messaging: MessagingConfig = Value::Table(messaging_table.clone())
		.try_into()
		.with_context(|| format!("failed to parse messaging config for chain '{name}'"))?;

	let consensus = match consensus_value {
		None => None,
		Some(Value::Table(mut cons_table)) => {
			// Inherit every base field (except `type`, which the consensus
			// variant discriminates on its own) that the consensus sub-table
			// hasn't explicitly overridden. This lets users specify `rpc_urls`,
			// `state_machine`, `ismp_host`, `signer`, `consensus_state_id`, etc.
			// once at the chain level.
			for (key, val) in messaging_table.iter() {
				if key == "type" {
					continue;
				}
				cons_table.entry(key.clone()).or_insert_with(|| val.clone());
			}

			let cfg: ConsensusConfig = Value::Table(cons_table)
				.try_into()
				.with_context(|| format!("failed to parse [{name}.consensus] sub-table"))?;
			Some(cfg)
		},
		Some(other) => {
			return Err(anyhow!("[{name}.consensus] must be a table, got {}", other.type_str()));
		},
	};

	Ok(PerChainConfig { messaging, consensus })
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Write;

	/// Minimal hyperbridge section so `parse_conf`'s structural checks pass.
	/// The substrate/beefy bits don't need to be syntactically valid clients
	/// since parsing stops at `try_into()` for `HyperbridgeHostConfig`.
	const HB_HEADER: &str = r#"
[hyperbridge]
type = "beefy"

[hyperbridge.substrate]
state_machine = "KUSAMA-4009"
hashing = "Keccak"
rpc_ws = "ws://127.0.0.1:9001"
signer = "0x00"

[hyperbridge.prover]
relay_rpc_ws = "ws://127.0.0.1:9944"
para_rpc_ws = "ws://127.0.0.1:9001"
para_ids = [4009]
proof_variant = "naive"

[hyperbridge.beefy]
consensus_state_id = [80, 65, 82, 65]

[hyperbridge.redis]
url = "127.0.0.1"
port = 6379
db = 0
ns = "rsmq"
realtime = false
mandatory_queue = "m"
messages_queue = "q"

[relayer]
minimum_profit_percentage = 0
delivery_endpoints = ["EVM-97"]
"#;

	async fn parse(body: &str) -> Result<HyperbridgeConfig, anyhow::Error> {
		let mut f = tempfile::NamedTempFile::new()?;
		write!(f, "{HB_HEADER}{body}")?;
		let path = f.path().to_owned();
		HyperbridgeConfig::parse_conf(path.to_str().unwrap()).await
	}

	#[tokio::test]
	async fn messaging_only_chain_has_no_consensus() {
		let cfg = parse(
			r#"
[bsc_chapel]
type = "evm"
state_machine = "EVM-97"
rpc_urls = ["https://example.invalid"]
consensus_state_id = "BSC0"
ismp_host = "0xFE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"
signer = "0x00"
"#,
		)
		.await
		.expect("parse should succeed");

		let pc = cfg.chains.get(&StateMachine::Evm(97)).expect("EVM-97 chain present");
		assert!(pc.consensus.is_none(), "no [.consensus] sub-table => consensus None");
		assert!(matches!(pc.messaging, MessagingConfig::Evm(_)));
	}

	#[tokio::test]
	async fn consensus_sub_table_opts_chain_in() {
		let cfg = parse(
			r#"
[bsc_chapel]
type = "evm"
state_machine = "EVM-97"
rpc_urls = ["https://example.invalid"]
consensus_state_id = "BSC0"
ismp_host = "0xFE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"
signer = "0x00"

[bsc_chapel.consensus]
type = "bsc_testnet"

[bsc_chapel.consensus.host]
consensus_update_frequency = 60
epoch_length = 200
"#,
		)
		.await
		.expect("parse should succeed");

		let pc = cfg.chains.get(&StateMachine::Evm(97)).expect("EVM-97 chain present");
		assert!(pc.consensus.is_some(), "[.consensus] present => consensus Some");

		// The consensus variant must receive the inherited host fields
		// (rpc_urls/state_machine/ismp_host/signer/consensus_state_id).
		match pc.consensus.as_ref().unwrap() {
			ConsensusConfig::BscTestnet(bsc) => {
				assert_eq!(bsc.evm_config.state_machine, StateMachine::Evm(97));
				assert_eq!(bsc.evm_config.consensus_state_id, "BSC0");
				assert_eq!(bsc.evm_config.rpc_urls, vec!["https://example.invalid".to_string()]);
				assert_eq!(bsc.host.epoch_length, 200);
			},
			other => panic!("expected BscTestnet variant, got {other:?}"),
		}
	}

	#[tokio::test]
	async fn consensus_config_view_filters_out_messaging_only_chains() {
		let cfg = parse(
			r#"
[chapel]
type = "evm"
state_machine = "EVM-97"
rpc_urls = ["https://example.invalid"]
consensus_state_id = "BSC0"
ismp_host = "0xFE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"
signer = "0x00"

[chapel.consensus]
type = "bsc_testnet"

[chapel.consensus.host]
consensus_update_frequency = 60
epoch_length = 200

[base_sepolia]
type = "evm"
state_machine = "EVM-84532"
rpc_urls = ["https://example2.invalid"]
consensus_state_id = "ETH0"
ismp_host = "0xFE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"
signer = "0x00"
"#,
		)
		.await
		.expect("parse should succeed");

		let consensus_view = cfg.consensus_config();
		assert!(consensus_view.chains.contains_key(&StateMachine::Evm(97)));
		assert!(!consensus_view.chains.contains_key(&StateMachine::Evm(84532)));
	}

	#[tokio::test]
	async fn relayer_outbound_defaults_to_true() {
		let cfg = parse(r#""#).await.expect("parse should succeed");
		assert!(cfg.relayer.outbound, "outbound defaults to true");
	}
}
