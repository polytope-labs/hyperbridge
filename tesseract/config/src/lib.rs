// Copyright (c) 2024 Polytope Labs.
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

use tesseract_primitives::IsmpProvider;
use std::sync::Arc;
use substrate_state_machine::HashAlgorithm;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_substrate::{
	config::{Blake2SubstrateChain, KeccakSubstrateChain},
	SubstrateClient, SubstrateConfig,
};

/// The AnyConfig wraps the configuration options for all supported chains
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(tag = "untagged", rename_all = "snake_case")]
pub enum AnyConfig {
	/// Configuration for substrate-based chains
	Substrate(SubstrateConfig),
	/// Configuration for evn-based chains
	Ethereum(EvmConfig),
}

impl AnyConfig {
	pub fn state_machine(&self) -> ismp::host::StateMachine {
		match self {
			Self::Substrate(config) => config.state_machine,
			Self::Ethereum(config) => config.state_machine,
		}
	}
}

impl AnyConfig {
	/// Convert the [`AnyConfig`] into an implementation of an [`IsmpProvider`]
	pub async fn into_client(self) -> Result<Arc<dyn IsmpProvider>, anyhow::Error> {
		let client = match self {
			AnyConfig::Substrate(config) => {
				match config.hashing.clone().unwrap_or(HashAlgorithm::Keccak) {
					HashAlgorithm::Keccak =>
						Arc::new(SubstrateClient::<KeccakSubstrateChain>::new(config).await?)
							as Arc<dyn IsmpProvider>,
					HashAlgorithm::Blake2 =>
						Arc::new(SubstrateClient::<Blake2SubstrateChain>::new(config).await?)
							as Arc<dyn IsmpProvider>,
				}
			},
			AnyConfig::Ethereum(config) => {
				let client = EvmClient::new(config).await?;
				Arc::new(client) as Arc<dyn IsmpProvider>
			},
		};

		Ok(client)
	}
}
