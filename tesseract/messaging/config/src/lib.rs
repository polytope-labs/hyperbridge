// Copyright (c) 2025 Polytope Labs.
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

/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "messaging-config";

use std::sync::Arc;
use substrate_state_machine::HashAlgorithm;
use tendermint_primitives::keys::{DefaultEvmKeys, SeiEvmKeys};
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_evm_tendermint::{TendermintEvmClient, TendermintEvmClientConfig};
use tesseract_pharos_evm::PharosEvmClient;
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::{
	config::{Blake2SubstrateChain, KeccakSubstrateChain},
	SubstrateClient, SubstrateConfig,
};
use tesseract_substrate_evm::{SubstrateEvmClient, SubstrateEvmClientConfig};
use tesseract_tron::{TronClient, TronConfig};

/// The AnyConfig wraps the configuration options for all supported chains
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnyConfig {
	/// Configuration for substrate-based chains
	Substrate(SubstrateConfig),
	/// Configuration for evm-based chains
	Evm(EvmConfig),
	/// Configuration for tendermint-based chains
	Tendermint(TendermintEvmClientConfig),
	/// Configuration for substrate-evm(revive) based chains
	SubstrateEvm(SubstrateEvmClientConfig),
	/// Configuration for Pharos EVM chains
	PharosEvm(EvmConfig),
	/// Configuration for Tron chains
	Tron(TronConfig),
}

impl AnyConfig {
	pub fn state_machine(&self) -> ismp::host::StateMachine {
		// `state_machine` is optional on the per-client configs so that
		// chains in `tesseract_evm::registry` / `tesseract_substrate::registry`
		// can be auto-derived. The relayer-level routing layer, however,
		// keys chains by `StateMachine` and can't proceed without one — so
		// require it here with a clear error.
		match self {
			Self::Substrate(config) => config
				.state_machine
				.expect("[<chain>] state_machine must be set explicitly for relayer routing"),
			Self::Evm(config) => config
				.state_machine
				.expect("[<chain>] state_machine must be set explicitly for relayer routing"),
			Self::Tendermint(tendermint_config) => tendermint_config
				.evm_config
				.state_machine
				.expect("[<chain>] state_machine must be set explicitly for relayer routing"),
			Self::SubstrateEvm(substrate_evm_config) => substrate_evm_config
				.evm
				.state_machine
				.expect("[<chain>] state_machine must be set explicitly for relayer routing"),
			Self::PharosEvm(config) => config
				.state_machine
				.expect("[<chain>] state_machine must be set explicitly for relayer routing"),
			Self::Tron(config) => config.state_machine(),
		}
	}

	/// The chain's signing key as configured. Returns the raw string from the
	/// per-chain TOML block, which is the secp256k1 private key (EVM family) or
	/// SR25519 seed (Substrate). `None` signals that this chain does not
	/// participate in roles that require signing (outbound delivery, fee
	/// withdrawal POSTs, fisherman vetoes), and the relayer skips it for
	/// those tasks.
	pub fn signer(&self) -> Option<&str> {
		match self {
			Self::Substrate(config) => config.signer.as_deref(),
			Self::Evm(config) => config.signer.as_deref(),
			Self::Tendermint(tendermint_config) => tendermint_config.evm_config.signer.as_deref(),
			Self::SubstrateEvm(substrate_evm_config) => substrate_evm_config.evm.signer.as_deref(),
			Self::PharosEvm(config) => config.signer.as_deref(),
			Self::Tron(config) => config.evm.signer.as_deref(),
		}
	}
}

impl AnyConfig {
	/// Convert the [`AnyConfig`] into an implementation of an [`IsmpProvider`]
	pub async fn into_client(
		self,
		hyperbridge: Arc<dyn IsmpProvider>,
	) -> Result<Arc<dyn IsmpProvider>, anyhow::Error> {
		let client = match self {
			AnyConfig::Substrate(config) => {
				match config.hashing.clone().unwrap_or(HashAlgorithm::Keccak) {
					HashAlgorithm::Keccak => {
						let mut client =
							SubstrateClient::<KeccakSubstrateChain>::new(config).await?;
						client.set_latest_finalized_height(hyperbridge).await?;
						Arc::new(client) as Arc<dyn IsmpProvider>
					},
					HashAlgorithm::Blake2 => {
						let mut client =
							SubstrateClient::<Blake2SubstrateChain>::new(config).await?;
						client.set_latest_finalized_height(hyperbridge).await?;
						Arc::new(client) as Arc<dyn IsmpProvider>
					},
				}
			},
			AnyConfig::Evm(config) => {
				let mut client = EvmClient::new(config).await?;
				client.set_latest_finalized_height(hyperbridge).await?;
				Arc::new(client) as Arc<dyn IsmpProvider>
			},
			AnyConfig::Tendermint(tendermint_config) => {
				let evm_inner = EvmClient::new(tendermint_config.evm_config).await?;
				match evm_inner.state_machine {
					ismp::host::StateMachine::Evm(chain_id)
						if chain_id == 1328 || chain_id == 1329 =>
					{
						let mut client = TendermintEvmClient::<SeiEvmKeys>::new(
							evm_inner,
							tendermint_config.rpc_url,
						)
						.await?;
						client.set_latest_finalized_height(hyperbridge).await?;
						Arc::new(client) as Arc<dyn IsmpProvider>
					},
					_ => {
						let mut client = TendermintEvmClient::<DefaultEvmKeys>::new(
							evm_inner,
							tendermint_config.rpc_url,
						)
						.await?;
						client.set_latest_finalized_height(hyperbridge).await?;
						Arc::new(client) as Arc<dyn IsmpProvider>
					},
				}
			},
			AnyConfig::SubstrateEvm(config) => {
				let mut client = SubstrateEvmClient::<Blake2SubstrateChain>::new(config).await?;
				client.set_latest_finalized_height(hyperbridge).await?;
				Arc::new(client) as Arc<dyn IsmpProvider>
			},
			AnyConfig::PharosEvm(config) => {
				let mut client = PharosEvmClient::new(config).await?;
				client.set_latest_finalized_height(hyperbridge).await?;
				Arc::new(client) as Arc<dyn IsmpProvider>
			},
			AnyConfig::Tron(config) => {
				let mut client = TronClient::new(config).await?;
				client.set_latest_finalized_height(hyperbridge).await?;
				Arc::new(client) as Arc<dyn IsmpProvider>
			},
		};

		Ok(client)
	}
}
