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
use pallet_ismp::primitives::HashAlgorithm;
use std::collections::HashMap;
// use grandpa::{GrandpaConfig, GrandpaHost};
use ismp::host::StateMachine;
use ismp_sync_committee::constants::{mainnet::Mainnet, sepolia::Sepolia};
use parachain::ParachainHost;
use primitives::config::RelayerConfig;
use serde::{Deserialize, Serialize};
use tesseract_beefy::BeefyConfig;
use tesseract_bnb_pos::{BnbPosConfig, BnbPosHost};
use tesseract_evm::{
	arbitrum::client::{ArbConfig, ArbHost},
	optimism::client::{OpConfig, OpHost},
	EvmClient,
};
use tesseract_polygon_pos::{PolygonPosConfig, PolygonPosHost};
use tesseract_substrate::{
	config::{Blake2SubstrateChain, KeccakSubstrateChain},
	SubstrateClient, SubstrateConfig,
};
use tesseract_sync_committee::{SyncCommitteeConfig, SyncCommitteeHost};
use toml::Table;

type Parachain<T> = SubstrateClient<ParachainHost, T>;
// type Grandpa<T> = SubstrateClient<GrandpaHost<T>, T>;

crate::chain! {
	KeccakParachain(SubstrateConfig, Parachain<KeccakSubstrateChain>),
	Parachain(SubstrateConfig, Parachain<Blake2SubstrateChain>),
	EthereumSepolia(SyncCommitteeConfig, EvmClient<SyncCommitteeHost<Sepolia>>),
	EthereumMainnet(SyncCommitteeConfig, EvmClient<SyncCommitteeHost<Mainnet>>),
	Arbitrum(ArbConfig, EvmClient<ArbHost>),
	Optimism(OpConfig, EvmClient<OpHost>),
	Base(OpConfig, EvmClient<OpHost>),
	Polygon(PolygonPosConfig, EvmClient<PolygonPosHost>),
	Bsc(BnbPosConfig, EvmClient<BnbPosHost>),
	// Polkadot(GrandpaConfig, Grandpa<Blake2SubstrateChain>),
	// Kusama(GrandpaConfig, Grandpa<Blake2SubstrateChain>),
}

/// Defines the format of the tesseract config.toml file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperbridgeConfig {
	/// Configuration options for hyperbridge.
	pub hyperbridge: BeefyConfig,
	/// Other chains
	pub chains: HashMap<StateMachine, AnyConfig>,
	/// Relayer config
	pub relayer: RelayerConfig,
}
const HYPERRIDGE: &'static str = "hyperbridge";
const RELAYER: &'static str = "relayer";

impl HyperbridgeConfig {
	pub async fn parse_conf(config: String) -> Result<Self, anyhow::Error> {
		let toml = tokio::fs::read_to_string(&config).await?;
		let table = toml.parse::<Table>()?;
		let mut chains: HashMap<StateMachine, AnyConfig> = HashMap::new();
		if !table.contains_key(HYPERRIDGE) || !table.contains_key(RELAYER) {
			Err(anyhow!("Missing Hyperbridge or Relayer Config, Check your toml file"))?
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

impl AnyConfig {
	/// Convert the [`HyperbridgeConfig`] into an implementation of an [`IsmpHost`]
	pub async fn into_client(self) -> Result<AnyClient, anyhow::Error> {
		let client = match self {
			AnyConfig::KeccakParachain(config) | AnyConfig::Parachain(config) => {
				match config.hashing {
					HashAlgorithm::Keccak => {
						let host = ParachainHost::default();
						AnyClient::KeccakParachain(Parachain::new(host, config).await?)
					},
					HashAlgorithm::Blake2 => {
						let host = ParachainHost::default();
						AnyClient::Parachain(Parachain::new(host, config).await?)
					},
				}
			},
			AnyConfig::EthereumSepolia(config) => {
				let host = SyncCommitteeHost::new(&config).await?;
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::EthereumSepolia(client)
			},
			AnyConfig::EthereumMainnet(config) => {
				let host = SyncCommitteeHost::new(&config).await?;
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::EthereumMainnet(client)
			},
			AnyConfig::Arbitrum(config) => {
				let host = ArbHost::new(&config).await?;
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::Arbitrum(client)
			},
			AnyConfig::Optimism(config) => {
				let host = OpHost::new(&config).await?;
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::Optimism(client)
			},
			AnyConfig::Base(config) => {
				let host = OpHost::new(&config).await?;
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::Base(client)
			},
			AnyConfig::Polygon(config) => {
				let host = PolygonPosHost::new(&config).await?;
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::Polygon(client)
			},
			AnyConfig::Bsc(config) => {
				let host = BnbPosHost::new(&config).await?;
				let client = EvmClient::new(host, config.evm_config).await?;
				AnyClient::Bsc(client)
			}, /* AnyConfig::Polkadot(config) => {
			    *     let naive = GrandpaHost::new(&config).await?;
			    *     AnyClient::Grandpa(Grandpa::new(naive, config.substrate).await?)
			    * } */
		};

		Ok(client)
	}
}

#[tokio::test]
async fn test_parsing() {
	let config = HyperbridgeConfig::parse_conf("../test-config.toml".to_string()).await.unwrap();
	dbg!(config);
}
