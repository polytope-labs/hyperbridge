use crate::any::AnyConfig;
use anyhow::anyhow;
use ismp::host::StateMachine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tesseract_beefy::BeefyConfig;
use tesseract_primitives::config::RelayerConfig;

use toml::Table;

/// Defines the format of the tesseract config.toml file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperbridgeConfig {
    /// Configuration options for hyperbridge.
    pub hyperbridge: BeefyConfig,
    /// Chains
    pub chains: HashMap<StateMachine, AnyConfig>,
    /// Relayer config
    pub relayer: RelayerConfig,
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
        if !table.contains_key(HYPERRIDGE) || !table.contains_key(RELAYER) {
            Err(anyhow!(
                "Missing Hyperbridge or Relayer in config, Check your toml file"
            ))?
        }

        let hyperbridge: BeefyConfig = table
            .get(HYPERRIDGE)
            .cloned()
            .expect("Hyperbridge Config is Present")
            .try_into()
            .expect("Failed to parse hyperbridge config");

        let relayer: RelayerConfig = table
            .get(RELAYER)
            .cloned()
            .expect("Hyperbridge Config is Present")
            .try_into()
            .expect("Failed to parse hyperbridge config");
        for (key, val) in table {
            if &key != HYPERRIDGE && &key != RELAYER {
                let any_conf: AnyConfig = val.try_into().unwrap();
                chains.insert(any_conf.state_machine(), any_conf);
            }
        }
        Ok(Self {
            hyperbridge,
            chains,
            relayer,
        })
    }
}
