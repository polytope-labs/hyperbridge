use crate::any::{AnyConfig, HyperbridgeHostConfig};
use anyhow::anyhow;
use ismp::{consensus::StateMachineId, host::StateMachine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use toml::Table;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperbridgeConfig {
	/// Configuration options for hyperbridge.
	pub hyperbridge: HyperbridgeHostConfig,
	/// Chains
	pub chains: HashMap<StateMachine, AnyConfig>,
	/// Additional Relayer configuration
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
		let mut chains: HashMap<StateMachine, AnyConfig> = HashMap::new();
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

		for (key, val) in table {
			if &key != HYPERRIDGE && &key != RELAYER {
				let any_conf: AnyConfig = val.try_into().unwrap();
				chains.insert(any_conf.state_machine(), any_conf);
			}
		}
		Ok(Self { hyperbridge, chains, relayer })
	}
}

#[tokio::test]
#[ignore]
async fn test_parsing() {
	let config = HyperbridgeConfig::parse_conf("../test-config.toml").await.unwrap();
	dbg!(config);
}
