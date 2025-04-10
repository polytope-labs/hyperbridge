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

use bsc_prover::BscPosProver;
pub use bsc_verifier::{
	primitives::{compute_epoch, parse_extra, BscClientUpdate, Config},
	verify_bsc_header,
};
use ethers::providers::{Http, Middleware, Provider};
pub use geth_primitives::Header;
use ismp::{consensus::ConsensusStateId, host::StateMachine, messaging::Keccak256};
pub use ismp_bsc::ConsensusState;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod host;
mod notification;

pub use bsc_verifier::primitives::{Mainnet, Testnet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BscPosConfig {
	/// Host configuration options
	pub host: HostConfig,
	/// General ethereum config
	#[serde[flatten]]
	pub evm_config: EvmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
	pub consensus_update_frequency: Option<u64>,
	pub epoch_length: u64,
}

impl BscPosConfig {
	/// Convert the config into a client.
	pub async fn into_client<C: Config + 'static>(self) -> anyhow::Result<Arc<dyn IsmpHost>> {
		Ok(Arc::new(BscPosHost::<C>::new(&self.host, &self.evm_config).await?))
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

#[derive(Clone)]
pub struct BscPosHost<C: Config> {
	/// Consensus state id on counterparty chain
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this chain.
	pub state_machine: StateMachine,
	/// Consensus prover
	pub prover: BscPosProver<C>,
	/// Host config options
	pub host: HostConfig,
	/// Evm config options
	pub evm: EvmConfig,
	/// Ismp provider
	pub provider: Arc<dyn IsmpProvider>,
}

impl<C: Config> BscPosHost<C> {
	pub async fn new(host: &HostConfig, evm: &EvmConfig) -> Result<Self, anyhow::Error> {
		let provider = Provider::new(Http::new_client_with_chain_middleware(
			evm.rpc_urls.iter().map(|url| url.parse()).collect::<Result<_, _>>()?,
			None,
		));
		let prover = BscPosProver::new(provider);
		let ismp_provider = EvmClient::new(evm.clone()).await?;

		Ok(Self {
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(evm.consensus_state_id.as_bytes());
				consensus_state_id
			},
			state_machine: evm.state_machine,
			prover,
			host: host.clone(),
			evm: evm.clone(),
			provider: Arc::new(ismp_provider),
		})
	}

	pub async fn get_consensus_state<I: Keccak256>(&self) -> Result<ConsensusState, anyhow::Error> {
		let (header, current_validators) = self
			.prover
			.fetch_finalized_state::<KeccakHasher>(self.host.epoch_length)
			.await?;

		let chain_id = self.prover.client.get_chainid().await?;
		let consensus_state = ConsensusState {
			current_validators,
			next_validators: None,
			finalized_hash: Header::from(&header).hash::<KeccakHasher>(),
			finalized_height: header.number.as_u64(),
			current_epoch: compute_epoch(header.number.low_u64(), self.host.epoch_length),
			chain_id: chain_id.low_u32(),
		};

		Ok(consensus_state)
	}
}

pub struct KeccakHasher;

impl Keccak256 for KeccakHasher {
	fn keccak256(bytes: &[u8]) -> primitive_types::H256
	where
		Self: Sized,
	{
		sp_core::keccak_256(bytes).into()
	}
}
