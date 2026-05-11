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
use ismp::{consensus::StateMachineId, host::StateMachine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tesseract_config::AnyConfig as MessagingConfig;
use tesseract_consensus_config::AnyConfig as ConsensusConfig;
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
	/// Per-`(state_machine_id, max_interval_secs)` entries enabling the
	/// passive liveness monitor. When set, a background task periodically
	/// checks how stale hyperbridge's view of each listed chain is (inbound
	/// consensus side), and for substrate chains, also checks how stale the
	/// chain's view of hyperbridge is (outbound HB → substrate consensus
	/// side). If any update lags by more than its `max_interval`, the
	/// relayer process exits so an external supervisor can restart it.
	pub maximum_update_intervals: Option<Vec<(StateMachineId, u64)>>,
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
			maximum_update_intervals: None,
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
	/// Hyperbridge's own substrate host config, optionally paired with a
	/// consensus sub-config when the operator wants this relayer to ship
	/// hyperbridge's own consensus updates (e.g. parachain proofs) to the
	/// counterparty chains it talks to.
	pub hyperbridge: HyperbridgeSection,
	pub chains: HashMap<StateMachine, PerChainConfig>,
	pub relayer: RelayerConfig,
}

/// Grouping of hyperbridge's substrate config and an optional consensus
/// pairing. Shaped like [`PerChainConfig`] but carries a plain
/// [`SubstrateConfig`] rather than the messaging-agnostic `AnyConfig`, since
/// hyperbridge is always substrate-native.
///
/// TOML surface: the `[hyperbridge]` table holds the substrate fields
/// directly; an optional `[hyperbridge.consensus]` sub-table carries a
/// [`ConsensusConfig`] (just like `[<chain>.consensus]` does for peers).
#[derive(Debug, Clone)]
pub struct HyperbridgeSection {
	/// Essentials only — substrate RPC + signer. The BEEFY prover/host is a
	/// separate binary that pushes accepted proofs into pallet storage; this
	/// relayer just consumes them via `offchain_localStorageGet`.
	pub substrate: SubstrateConfig,
	/// Optional inbound-consensus config for hyperbridge itself — e.g. a
	/// `parachain` variant that ships hyperbridge's own parachain-header
	/// proofs to other chains.
	pub consensus: Option<ConsensusConfig>,
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

		let hyperbridge =
			parse_hyperbridge_section(table.get(HYPERBRIDGE).cloned().expect("checked above"))
				.await?;

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

	/// Build the consensus relayer's per-chain pairings for every chain (and
	/// hyperbridge itself) that opted into inbound consensus via a
	/// `[<chain>.consensus]` / `[hyperbridge.consensus]` sub-table. Each
	/// pairing extracts the host config (EVM or substrate) from the
	/// messaging side since consensus variants no longer embed it.
	pub fn consensus_chains(
		&self,
	) -> HashMap<
		StateMachine,
		(tesseract_consensus_config::AnyConfig, tesseract_consensus_config::HostKind),
	> {
		use tesseract_config::AnyConfig as Msg;
		use tesseract_consensus_config::{AnyConfig as Consensus, HostKind};

		let mut out: HashMap<
			StateMachine,
			(tesseract_consensus_config::AnyConfig, tesseract_consensus_config::HostKind),
		> = self
			.chains
			.iter()
			.filter_map(|(sm, pc)| {
				let consensus = pc.consensus.clone()?;
				let host = match &pc.messaging {
					Msg::Evm(e) => HostKind::Evm(e.clone()),
					Msg::PharosEvm(e) => HostKind::Evm(e.clone()),
					Msg::Tendermint(t) => HostKind::Evm(t.evm_config.clone()),
					// Substrate-EVM chains can run with either parachain
					// consensus (where the consensus client needs the substrate
					// half to read `Paras::Heads` storage proofs and so wants
					// the full SubstrateEvmClientConfig) or with a pure EVM
					// consensus client (sync-committee L2, etc.) which only
					// needs the EVM half. Pick the variant based on what the
					// consensus side actually expects.
					Msg::SubstrateEvm(se) => match &consensus {
						Consensus::Parachain { .. } => HostKind::SubstrateEvm(se.clone()),
						_ => return None,
					},
					Msg::Substrate(s) => HostKind::Substrate(s.clone()),
					Msg::Tron(_) => return None, // tron is not a supported consensus host
				};
				Some((*sm, (consensus, host)))
			})
			.collect();

		// Hyperbridge itself — include it only if the operator supplied a
		// consensus sub-table under `[hyperbridge.consensus]`. Hyperbridge is
		// always substrate-native so the host kind is unconditionally
		// `HostKind::Substrate`.
		if let Some(consensus) = self.hyperbridge.consensus.clone() {
			out.insert(
				self.hyperbridge.substrate.state_machine(),
				(consensus, HostKind::Substrate(self.hyperbridge.substrate.clone())),
			);
		}

		out
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
/// For EVM chains, `state_machine`, `ismp_host`, and `consensus_state_id`
/// are derived from the chain via [`AnyConfig::resolve`] right after the
/// TOML deserialises — see [`tesseract_evm::registry`] for the lookup
/// tables. Substrate chains derive `state_machine` (and the
/// Kusama/Polkadot consensus state id) the same way. After this call,
/// every consumer that reads `state_machine()` sees a concrete value
/// without touching the chain.
async fn parse_chain(name: &str, chain_table: &Table) -> Result<PerChainConfig, anyhow::Error> {
	let mut messaging_table = chain_table.clone();
	let consensus_value = messaging_table.remove(CONSENSUS);

	let messaging: MessagingConfig = Value::Table(messaging_table.clone())
		.try_into()
		.with_context(|| format!("failed to parse messaging config for chain '{name}'"))?;
	let messaging = messaging
		.resolve()
		.await
		.with_context(|| format!("failed to resolve config for chain '{name}'"))?;

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

/// Parse the top-level `[hyperbridge]` TOML section into a
/// [`HyperbridgeSection`]. Mirrors [`parse_chain`]: strips any
/// `[hyperbridge.consensus]` sub-table before deserialising the remaining
/// fields as a [`SubstrateConfig`], then resolves so `state_machine` and
/// `consensus_state_id` are guaranteed before any consumer reads them.
async fn parse_hyperbridge_section(raw: Value) -> Result<HyperbridgeSection, anyhow::Error> {
	let Value::Table(mut table) = raw else {
		return Err(anyhow!("[hyperbridge] must be a table, got {}", raw.type_str(),));
	};
	let consensus_value = table.remove(CONSENSUS);

	let substrate: SubstrateConfig = Value::Table(table)
		.try_into()
		.with_context(|| "failed to parse [hyperbridge] substrate fields")?;
	let substrate = substrate
		.resolve()
		.await
		.with_context(|| "failed to resolve [hyperbridge] substrate fields")?;

	let consensus = match consensus_value {
		None => None,
		Some(Value::Table(cons_table)) => {
			let cfg: ConsensusConfig = Value::Table(cons_table)
				.try_into()
				.with_context(|| "failed to parse [hyperbridge.consensus] sub-table")?;
			Some(cfg)
		},
		Some(other) => {
			return Err(anyhow!(
				"[hyperbridge.consensus] must be a table, got {}",
				other.type_str(),
			));
		},
	};

	Ok(HyperbridgeSection { substrate, consensus })
}

pub fn setup_logging() -> Result<(), anyhow::Error> {
	use tracing_subscriber::{fmt, prelude::*, EnvFilter};

	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
	// `fmt::layer` defaults to auto-detect: ANSI on only if stdout is a
	// TTY. That breaks under systemd / `nohup` / docker where stdout is
	// piped to journald or a file and colors get silently dropped. Force
	// ANSI on; the standard `NO_COLOR=1` opt-out (https://no-color.org)
	// disables it for users who pipe through non-rendering tools. The
	// raw bytes survive into the journal, and `journalctl` pages through
	// `less -R` by default so colors render on read-out.
	let use_ansi = std::env::var_os("NO_COLOR").is_none();
	tracing_subscriber::registry()
		.with(fmt::layer().with_ansi(use_ansi))
		.with(filter)
		.init();

	Ok(())
}
