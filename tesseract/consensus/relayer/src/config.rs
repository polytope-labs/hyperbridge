use crate::any::{AnyConfig, HyperbridgeHostConfig};
use anyhow::anyhow;
use ismp::{consensus::StateMachineId, host::StateMachine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tesseract_consensus_config::HostKind;
use tesseract_evm::EvmConfig;
use tesseract_substrate::SubstrateConfig;

use toml::{Table, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerConfig {
	/// Challenge period to be used when creating consensus states
	pub challenge_period: Option<u64>,
	/// The maximum interval in seconds allowed between consensus updates for each state machine
	/// before the process should be restarted
	pub maximum_update_intervals: Option<Vec<(StateMachineId, u64)>>,
	/// Enables the hyperbridge host
	#[serde(default = "default_true")]
	pub enable_hyperbridge_consensus: bool,
}

fn default_true() -> bool {
	true
}

impl Default for RelayerConfig {
	fn default() -> Self {
		Self {
			challenge_period: None,
			maximum_update_intervals: None,
			enable_hyperbridge_consensus: true,
		}
	}
}

/// Defines the format of the tesseract config.toml file.
///
/// `hyperbridge` is optional: the consolidated `tesseract-relayer` doesn't
/// run the BEEFY prover/host (that's a separate binary) and leaves this
/// field as `None`. Callers that *do* need the full host config (like the
/// standalone prover binary) populate it.
///
/// Each chain entry is stored as a `(AnyConfig, HostKind)` pair because the
/// consensus config variants no longer embed the EVM / substrate host config
/// — callers supply it at construction time.
#[derive(Debug, Clone)]
pub struct HyperbridgeConfig {
	/// Configuration options for hyperbridge.
	pub hyperbridge: Option<HyperbridgeHostConfig>,
	/// Per-chain (consensus variant, host config) pairs.
	pub chains: HashMap<StateMachine, (AnyConfig, HostKind)>,
	/// Additional Relayer configuration.
	pub relayer: Option<RelayerConfig>,
}

const HYPERRIDGE: &'static str = "hyperbridge";
const RELAYER: &'static str = "relayer";

impl HyperbridgeConfig {
	pub async fn parse_conf(config: &str) -> Result<Self, anyhow::Error> {
		let toml = tokio::fs::read_to_string(config)
			.await
			.map_err(|err| anyhow!("Error occured while reading config file: {err:?}"))?;
		let table = toml.parse::<Table>()?;
		let mut chains: HashMap<StateMachine, (AnyConfig, HostKind)> = HashMap::new();
		if !table.contains_key(HYPERRIDGE) {
			Err(anyhow!("Missing Hyperbridge Config, Check your toml file"))?
		}

		let hyperbridge: HyperbridgeHostConfig = table
			.get(HYPERRIDGE)
			.cloned()
			.expect("Hyperbridge Config is Present")
			.try_into()
			.expect("Failed to parse hyperbridge config");

		let relayer: Option<RelayerConfig> = if let Some(value) = table.get(RELAYER).cloned() {
			let val = value.try_into().expect("Failed to parse relayer config");
			Some(val)
		} else {
			None
		};

		// Legacy TOML layout: consensus config (type + host fields) and the
		// host-side config (EvmConfig fields, or a `substrate = { ... }`
		// sub-table for grandpa) live side-by-side at the chain level. We
		// parse each half separately. The consensus variants no longer carry
		// a `#[serde(flatten)] evm_config`, so unrelated fields at the top
		// level are simply ignored by the AnyConfig deserializer.
		for (key, val) in table {
			if &key != HYPERRIDGE && &key != RELAYER {
				let any_conf: AnyConfig =
					val.clone().try_into().map_err(|err| anyhow!("[{key}] consensus: {err}"))?;
				let state_machine: StateMachine;
				let host: HostKind;

				if matches!(any_conf, AnyConfig::Grandpa(_)) {
					// Grandpa expects `[<chain>.substrate] { ... }`.
					let sub_val =
						val.as_table().and_then(|t| t.get("substrate")).cloned().ok_or_else(
							|| anyhow!("[{key}]: grandpa requires a [substrate] sub-table"),
						)?;
					let substrate: SubstrateConfig =
						sub_val.try_into().map_err(|err| anyhow!("[{key}.substrate]: {err}"))?;
					state_machine = substrate.state_machine;
					host = HostKind::Substrate(substrate);
				} else {
					let evm: EvmConfig =
						val.try_into().map_err(|err| anyhow!("[{key}] evm: {err}"))?;
					state_machine = evm.state_machine;
					host = HostKind::Evm(evm);
				}

				chains.insert(state_machine, (any_conf, host));
			}
		}
		Ok(Self { hyperbridge: Some(hyperbridge), chains, relayer })
	}
}

// keep `Value` import warning-free even if Value becomes unused after edits
#[allow(dead_code)]
fn _value_type_marker(_v: &Value) {}

#[tokio::test]
#[ignore]
async fn test_parsing() {
	let config = HyperbridgeConfig::parse_conf("../test-config.toml").await.unwrap();
	dbg!(config);
}
