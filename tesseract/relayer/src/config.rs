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
//! [hyperbridge]                          # HB substrate RPC — just the client
//! state_machine = "KUSAMA-4009"          # essentials, no prover/backend config.
//! rpc_ws        = "ws://..."             # The BEEFY prover runs as a separate
//! signer        = "0x..."                # binary; this relayer only consumes
//! hashing       = "Keccak"               # its output from the chain.
//!
//! [relayer]                              # operator knobs
//!
//! [<chain-name>]                         # one block per chain, messaging/host config
//! type = "evm"
//! rpc_urls = ["https://..."]
//! # state_machine and ismp_host auto-derive at startup from eth_chainId plus
//! # the relayer's built-in registry; set either explicitly to override.
//! consensus_state_id = "ETH0"
//! signer = "${SIG}"                      # presence of `signer` toggles outbound:
//!                                        # set it to include this chain in the
//!                                        # HB->chain fan-out, omit it for inbound only.
//!
//! [<chain-name>.consensus]               # OPTIONAL. Presence opts into inbound consensus.
//! type = "ethereum"                      # consensus-side client type
//! host = { ... }                         # consensus-only knobs
//! # Host fields (rpc_urls, state_machine, ismp_host, signer, consensus_state_id, ...)
//! # are inherited from the parent and do not need to be re-specified.
//! ```
//!
//! Every chain with a `[<chain-name>]` block gets inbound messaging spawned
//! automatically. Presence of `[chain.consensus]` is the sole signal to spawn
//! inbound consensus for that chain. Presence of a non-empty `signer` is the
//! sole signal that this chain participates in the HB->chain outbound fan-out
//! (and the related fee-withdrawal and fisherman roles).

use anyhow::{anyhow, Context};
use ismp::host::StateMachine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tesseract_config::AnyConfig as MessagingConfig;
use tesseract_consensus::any::AnyConfig as ConsensusConfig;
use tesseract_substrate::SubstrateConfig;
use toml::{Table, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
	// -- Messaging (inbound) --
	pub module_filter: Option<Vec<String>>,
	#[serde(default)]
	pub minimum_profit_percentage: u32,
	pub fisherman: Option<bool>,
	pub withdrawal_frequency: Option<u64>,
	pub minimum_withdrawal_amount: Option<u64>,
	pub unprofitable_retry_frequency: Option<u64>,
	pub deliver_failed: Option<bool>,
	pub disable_fee_accumulation: Option<bool>,
}

impl Default for RelayerConfig {
	fn default() -> Self {
		Self {
			module_filter: None,
			minimum_profit_percentage: 0,
			fisherman: None,
			withdrawal_frequency: None,
			minimum_withdrawal_amount: None,
			unprofitable_retry_frequency: None,
			deliver_failed: None,
			disable_fee_accumulation: None,
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
			// Unused by the consolidated relayer — every chain in `[chains.*]`
			// gets inbound messaging spawned automatically.
			delivery_endpoints: Vec::new(),
			deliver_failed: config.deliver_failed,
			fisherman: config.fisherman,
			disable_fee_accumulation: config.disable_fee_accumulation,
		}
	}
}

/// Per-chain configuration in the consolidated relayer.
///
/// `messaging` is always present, it's the host config used for inbound
/// messaging and outbound submission. `consensus` is optional: when present,
/// that chain also runs an inbound-consensus task. Whether this chain is a
/// destination for the HB->chain outbound fan-out is derived from the messaging
/// config's signer: a non-empty `signer` opts the chain into outbound, an empty
/// or absent one keeps it inbound only.
#[derive(Debug, Clone)]
pub struct PerChainConfig {
	pub messaging: MessagingConfig,
	pub consensus: Option<ConsensusConfig>,
}

impl PerChainConfig {
	/// True when this chain participates in the HB->chain outbound fan-out
	/// (and the related fee-withdrawal and fisherman roles). The toggle is
	/// the messaging signer's presence: a configured non-empty signer means
	/// the operator has provisioned a key to submit transactions on this
	/// chain.
	pub fn outbound_enabled(&self) -> bool {
		self.messaging.signer().is_some_and(|s| !s.is_empty())
	}
}

#[derive(Debug)]
pub struct HyperbridgeConfig {
	/// Essentials only — substrate RPC + signer. The BEEFY prover/host is a
	/// separate binary that pushes accepted proofs into pallet storage; this
	/// relayer just consumes them via `offchain_localStorageGet`.
	pub hyperbridge: SubstrateConfig,
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

		let hyperbridge: SubstrateConfig =
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
			let per_chain = parse_chain(name, chain_table).await?;
			let state_machine = per_chain.messaging.state_machine();
			if chains.insert(state_machine, per_chain).is_some() {
				return Err(anyhow!("duplicate chain configured for state machine {state_machine}"));
			}
		}

		Ok(Self { hyperbridge, chains, relayer })
	}

	/// Build the consensus relayer's per-chain pairings for the subset of
	/// chains that opted into inbound consensus via `[<chain>.consensus]`.
	/// Each pairing extracts the host config (EVM or substrate) from the
	/// messaging side since consensus variants no longer embed it.
	pub fn consensus_chains(
		&self,
	) -> HashMap<
		StateMachine,
		(tesseract_consensus::any::AnyConfig, tesseract_consensus::cli::HostKind),
	> {
		use tesseract_config::AnyConfig as Msg;
		use tesseract_consensus::cli::HostKind;

		self.chains
			.iter()
			.filter_map(|(sm, pc)| {
				let consensus = pc.consensus.clone()?;
				let host = match &pc.messaging {
					Msg::Evm(e) => HostKind::Evm(e.clone()),
					Msg::PharosEvm(e) => HostKind::Evm(e.clone()),
					Msg::Tendermint(t) => HostKind::Evm(t.evm_config.clone()),
					Msg::SubstrateEvm(se) => HostKind::Evm(se.evm.clone()),
					Msg::Substrate(s) => HostKind::Substrate(s.clone()),
					Msg::Tron(_) => return None, // tron is not a supported consensus host
				};
				Some((*sm, (consensus, host)))
			})
			.collect()
	}
}

/// Parse one `[<chain>]` block into a [`PerChainConfig`]. The base fields feed
/// the messaging [`MessagingConfig`]; if a `[<chain>.consensus]` sub-table is
/// present, the base fields are inherited into the consensus table before it's
/// deserialized as a [`ConsensusConfig`] (so host essentials are specified once).
///
/// Outbound participation is derived from the messaging config's `signer`
/// (see [`PerChainConfig::outbound_enabled`]); there is no separate
/// `outbound` toggle.
///
/// For EVM chains, `state_machine` and `ismp_host` are auto-derived from the
/// RPC (`eth_chainId` + the [`tesseract_evm::registry`] table) when the user
/// omits them; explicit values win over derivation.
async fn parse_chain(name: &str, chain_table: &Table) -> Result<PerChainConfig, anyhow::Error> {
	let mut messaging_table = chain_table.clone();
	let consensus_value = messaging_table.remove(CONSENSUS);

	autofill_missing_fields(name, &mut messaging_table).await?;

	let messaging: MessagingConfig = Value::Table(messaging_table.clone())
		.try_into()
		.with_context(|| format!("failed to parse messaging config for chain '{name}'"))?;

	let consensus = match consensus_value {
		None => None,
		Some(Value::Table(cons_table)) => {
			// Consensus variants no longer embed EvmConfig/SubstrateConfig.
			// The host config is threaded in separately at construction time
			// (see `HyperbridgeConfig::consensus_chains`). So the consensus
			// sub-table only needs the variant's own fields: `type` plus
			// whatever nested tables that variant declares (`host`, `grandpa`,
			// `layer_twos`, etc.).
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

/// Fills in fields the user omitted, using the chain's own RPC as the source
/// of truth:
///
/// - **EVM family** (`type = "evm"` | `"pharos_evm"`): derives `state_machine` from `eth_chainId`
///   and `ismp_host` from the [`tesseract_evm::registry`] table.
/// - **Substrate** (`type = "substrate"`): derives `state_machine` from `system_chain` +
///   `ParachainInfo::parachainId` storage (see [`tesseract_substrate::registry`]).
///
/// Explicit values always win over derivation; this helper only fills in
/// keys that are absent. No-op for chain types without derivation support
/// (e.g. tron).
async fn autofill_missing_fields(name: &str, chain_table: &mut Table) -> Result<(), anyhow::Error> {
	let chain_type = match chain_table.get("type").and_then(Value::as_str) {
		Some(t) => t.to_string(),
		None => return Ok(()), // deserializer will reject later with a clearer error
	};

	match chain_type.as_str() {
		"evm" | "pharos_evm" => autofill_evm(name, chain_table).await,
		"substrate" => autofill_substrate(name, chain_table).await,
		_ => Ok(()),
	}
}

async fn autofill_evm(name: &str, chain_table: &mut Table) -> Result<(), anyhow::Error> {
	let has_state_machine = chain_table.contains_key("state_machine");
	let has_ismp_host = chain_table.contains_key("ismp_host");
	if has_state_machine && has_ismp_host {
		tracing::debug!(target: crate::LOG_TARGET, chain = name, "autofill skipped — state_machine + ismp_host already set");
		return Ok(());
	}

	let rpc_url = chain_table
		.get("rpc_urls")
		.and_then(Value::as_array)
		.and_then(|arr| arr.first())
		.and_then(Value::as_str)
		.ok_or_else(|| {
			anyhow!(
				"[{name}]: cannot auto-derive state_machine/ismp_host without at least one \
				 entry in `rpc_urls`"
			)
		})?
		.to_string();

	let chain_id = tesseract_evm::registry::fetch_chain_id(&rpc_url)
		.await
		.with_context(|| format!("[{name}]: auto-derive via eth_chainId"))?;

	if !has_state_machine {
		let value = format!("EVM-{chain_id}");
		tracing::info!(target: crate::LOG_TARGET, chain = name, state_machine = %value, "auto-derived state_machine");
		chain_table.insert("state_machine".to_string(), Value::String(value));
	}

	if !has_ismp_host {
		let host = tesseract_evm::registry::ismp_host_for_chain_id(chain_id).ok_or_else(|| {
			anyhow!(
				"[{name}]: no known IsmpHost for chain_id={chain_id}. Set `ismp_host` explicitly \
				 or add the chain to tesseract_evm::registry."
			)
		})?;
		let hex = format!("0x{}", hex::encode(host.0));
		tracing::info!(target: crate::LOG_TARGET, chain = name, ismp_host = %hex, "auto-derived ismp_host");
		chain_table.insert("ismp_host".to_string(), Value::String(hex));
	}

	Ok(())
}

async fn autofill_substrate(name: &str, chain_table: &mut Table) -> Result<(), anyhow::Error> {
	if chain_table.contains_key("state_machine") {
		tracing::debug!(target: crate::LOG_TARGET, chain = name, "autofill skipped — state_machine already set");
		return Ok(());
	}

	let rpc_ws = chain_table
		.get("rpc_ws")
		.and_then(Value::as_str)
		.ok_or_else(|| anyhow!("[{name}]: cannot auto-derive state_machine without `rpc_ws`"))?
		.to_string();

	let state_machine = tesseract_substrate::registry::fetch_state_machine(&rpc_ws)
		.await
		.with_context(|| format!("[{name}]: auto-derive via system_chain + ParachainInfo"))?;

	let rendered = state_machine.to_string();
	tracing::info!(target: crate::LOG_TARGET, chain = name, state_machine = %rendered, "auto-derived state_machine");
	chain_table.insert("state_machine".to_string(), Value::String(rendered));
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Write;

	/// Minimal hyperbridge section — just a SubstrateConfig, no prover/backend.
	const HB_HEADER: &str = r#"
[hyperbridge]
state_machine = "KUSAMA-4009"
hashing = "Keccak"
rpc_ws = "ws://127.0.0.1:9001"
signer = "0x00"

[relayer]
minimum_profit_percentage = 0
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

		// Consensus variants no longer carry the EVM host config; the reviewer's
		// refactor moved it to a constructor argument. Here we only check the
		// consensus-specific fields arrived correctly.
		match pc.consensus.as_ref().unwrap() {
			ConsensusConfig::BscTestnet(bsc) => {
				assert_eq!(bsc.host.epoch_length, 200);
				assert_eq!(bsc.host.consensus_update_frequency, Some(60));
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

		let consensus_view = cfg.consensus_chains();
		assert!(consensus_view.contains_key(&StateMachine::Evm(97)));
		assert!(!consensus_view.contains_key(&StateMachine::Evm(84532)));
	}

	#[tokio::test]
	async fn outbound_enabled_when_signer_is_set() {
		let cfg = parse(
			r#"
[chapel]
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

		let pc = cfg.chains.get(&StateMachine::Evm(97)).expect("chain present");
		assert!(pc.outbound_enabled(), "non-empty signer opts the chain into outbound");
	}

	#[tokio::test]
	async fn outbound_disabled_when_signer_is_empty() {
		// Empty signer keeps the chain inbound only. Note: the underlying
		// chain client may still reject construction with an empty signer
		// today, this test just covers the relayer-level toggle semantics.
		let cfg = parse(
			r#"
[chapel]
type = "evm"
state_machine = "EVM-97"
rpc_urls = ["https://example.invalid"]
consensus_state_id = "BSC0"
ismp_host = "0xFE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"
signer = ""

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

		let chapel = cfg.chains.get(&StateMachine::Evm(97)).expect("chapel present");
		assert!(!chapel.outbound_enabled(), "empty signer keeps the chain inbound only");

		let base = cfg.chains.get(&StateMachine::Evm(84532)).expect("base present");
		assert!(base.outbound_enabled(), "non-empty signer opts the chain into outbound");
	}

	#[tokio::test]
	async fn autofill_skipped_when_both_fields_present() {
		// Both fields explicit → no RPC call, parse succeeds even though the
		// URL is bogus (proves we short-circuited before network access).
		let cfg = parse(
			r#"
[chapel]
type = "evm"
state_machine = "EVM-97"
rpc_urls = ["https://definitely-not-a-real-host.invalid"]
consensus_state_id = "BSC0"
ismp_host = "0xFE9f23F0F2fE83b8B9576d3FC94e9a7458DdDD35"
signer = "0x00"
"#,
		)
		.await
		.expect("parse should succeed without hitting the network");

		assert!(cfg.chains.contains_key(&StateMachine::Evm(97)));
	}

	#[tokio::test]
	async fn autofill_requires_rpc_urls_when_fields_missing() {
		// Missing state_machine AND ismp_host with empty rpc_urls → clear error,
		// not a network timeout.
		let err = parse(
			r#"
[chapel]
type = "evm"
rpc_urls = []
consensus_state_id = "BSC0"
signer = "0x00"
"#,
		)
		.await
		.expect_err("should fail: no rpc url to query");

		let msg = format!("{err:?}");
		assert!(
			msg.contains("rpc_urls") || msg.contains("auto-derive"),
			"error should mention missing rpc_urls, got: {msg}"
		);
	}

	#[tokio::test]
	async fn substrate_autofill_skipped_when_state_machine_present() {
		// Explicit state_machine → no RPC call (proven by the bogus rpc_ws).
		let cfg = parse(
			r#"
[asset_hub]
type = "substrate"
state_machine = "POLKADOT-1000"
rpc_ws = "ws://definitely-not-a-real-host.invalid"
signer = "0x00"
"#,
		)
		.await
		.expect("parse should succeed without hitting the network");

		assert!(cfg.chains.contains_key(&StateMachine::Polkadot(1000)));
	}

	#[tokio::test]
	async fn substrate_autofill_requires_rpc_ws_when_missing() {
		let err = parse(
			r#"
[asset_hub]
type = "substrate"
signer = "0x00"
"#,
		)
		.await
		.err()
		.expect("should fail: no rpc_ws to query");

		let msg = format!("{err:?}");
		assert!(
			msg.contains("rpc_ws") || msg.contains("auto-derive"),
			"error should mention missing rpc_ws, got: {msg}"
		);
	}
}
