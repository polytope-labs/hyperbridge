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
		// can be auto-derived. The relayer-level routing layer keys chains
		// by `StateMachine` so it must be set before this is called: the
		// config loader resolves any missing value via
		// [`AnyConfig::resolve_state_machine`] at parse time.
		match self {
			Self::Substrate(config) => config
				.state_machine
				.expect("state_machine should have been resolved at config-parse time"),
			Self::Evm(config) => config
				.state_machine
				.expect("state_machine should have been resolved at config-parse time"),
			Self::Tendermint(tendermint_config) => tendermint_config
				.evm_config
				.state_machine
				.expect("state_machine should have been resolved at config-parse time"),
			Self::SubstrateEvm(substrate_evm_config) => substrate_evm_config
				.evm
				.state_machine
				.expect("state_machine should have been resolved at config-parse time"),
			Self::PharosEvm(config) => config
				.state_machine
				.expect("state_machine should have been resolved at config-parse time"),
			Self::Tron(config) => config.state_machine(),
		}
	}

	/// Backfill `state_machine` on the inner per-client config if the
	/// operator did not set it explicitly. EVM-family configs derive it
	/// from `eth_chainId` against the first configured RPC URL; substrate
	/// configs derive it via the chain's `system_chain` and
	/// `ParachainInfo::parachainId` runtime calls. This is the routing
	/// layer's only parse-time autofill: the libraries
	/// (`EvmClient::new`, `SubstrateClient::new`) still own the
	/// resolution of `ismp_host` and `consensus_state_id` at client
	/// construction.
	pub async fn resolve_state_machine(&mut self) -> anyhow::Result<()> {
		async fn resolve_evm(evm: &mut EvmConfig) -> anyhow::Result<()> {
			if evm.state_machine.is_some() {
				return Ok(());
			}
			let url = evm.rpc_urls.first().ok_or_else(|| {
				anyhow::anyhow!("evm chain config requires at least one rpc_urls entry")
			})?;
			let chain_id = tesseract_evm::registry::fetch_chain_id(url).await?;
			evm.state_machine = Some(ismp::host::StateMachine::Evm(chain_id as u32));
			Ok(())
		}
		async fn resolve_substrate(sub: &mut SubstrateConfig) -> anyhow::Result<()> {
			if sub.state_machine.is_some() {
				return Ok(());
			}
			let sm = tesseract_substrate::registry::fetch_state_machine(&sub.rpc_ws).await?;
			sub.state_machine = Some(sm);
			Ok(())
		}
		match self {
			Self::Substrate(config) => resolve_substrate(config).await,
			Self::Evm(config) => resolve_evm(config).await,
			Self::Tendermint(tendermint_config) =>
				resolve_evm(&mut tendermint_config.evm_config).await,
			Self::SubstrateEvm(substrate_evm_config) =>
				resolve_evm(&mut substrate_evm_config.evm).await,
			Self::PharosEvm(config) => resolve_evm(config).await,
			Self::Tron(config) => resolve_evm(&mut config.evm).await,
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
