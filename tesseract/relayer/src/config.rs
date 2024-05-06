// Copyright (C) 2023 Polytope Labs.
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
#![allow(dead_code)]
//! Tesseract config utilities

use anyhow::anyhow;
use ismp::host::StateMachine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tesseract_config::AnyConfig;
use tesseract_primitives::config::RelayerConfig;
use tesseract_substrate::SubstrateConfig;
use toml::Table;

/// Defines the format of the tesseract config.toml file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperbridgeConfig {
	/// Configuration options for hyperbridge.
	pub hyperbridge: SubstrateConfig,
	/// Other chains
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
			Err(anyhow!("Missing Hyperbridge or Relayer Config, Check your toml file"))?
		}

		let hyperbridge: SubstrateConfig = table
			.get(HYPERRIDGE)
			.cloned()
			.expect("Hyperbridge Config is Present")
			.try_into()
			.expect("Failed to parse hyperbridge config");
		let relayer: RelayerConfig = table
			.get(RELAYER)
			.cloned()
			.expect("Relayer Config is Present")
			.try_into()
			.expect("Failed to parse relayer config");
		for (key, val) in table {
			if &key != HYPERRIDGE && key != RELAYER {
				let any_conf: AnyConfig = val.try_into().unwrap();
				chains.insert(any_conf.state_machine(), any_conf);
			}
		}
		Ok(Self { hyperbridge, chains, relayer })
	}
}

#[tokio::test]
async fn test_parsing() {
	let config = HyperbridgeConfig::parse_conf("../test-config.toml").await.unwrap();
	dbg!(config);
}
