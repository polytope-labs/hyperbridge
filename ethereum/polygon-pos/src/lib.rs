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

pub use consensus_client::{ConsensusState, PolygonClientUpdate};
use ethers::providers::{Provider, Ws};
pub use geth_primitives::Header;
use ismp::{consensus::ConsensusStateId, host::StateMachine, util::Keccak256};
use jsonrpsee::ws_client::WsClientBuilder;
use primitive_types::H160;
use prover::PolygonPosProver;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::IsmpProvider;
pub use verifier::verify_polygon_header;

mod byzantine;
mod host;
mod notification;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonPosConfig {
	/// General ethereum config
	#[serde[flatten]]
	pub evm_config: EvmConfig,
}

impl PolygonPosConfig {
	/// Convert the config into a client.
	pub async fn into_client<C: IsmpProvider>(
		self,
		counterparty: &C,
	) -> anyhow::Result<EvmClient<PolygonPosHost>> {
		let host = PolygonPosHost::new(&self).await?;
		let client = EvmClient::new(host, self.evm_config, counterparty).await?;

		Ok(client)
	}
}

#[derive(Clone)]
pub struct PolygonPosHost {
	/// Consensus state id on counterparty chain
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this chain.
	pub state_machine: StateMachine,
	/// Consensus prover
	pub prover: PolygonPosProver,
	/// Config
	pub config: PolygonPosConfig,
	/// Jsonrpsee client for event susbscription, ethers does not expose a Send and Sync stream for
	/// susbcribing to contract logs
	pub rpc_client: Arc<jsonrpsee::ws_client::WsClient>,
}

impl PolygonPosHost {
	pub async fn new(config: &PolygonPosConfig) -> Result<Self, anyhow::Error> {
		let provider =
			Provider::<Ws>::connect_with_reconnects(config.evm_config.execution_ws.clone(), 1000)
				.await
				.unwrap();
		let prover = PolygonPosProver::new(provider);
		let rpc_client = WsClientBuilder::default().build(&config.evm_config.execution_ws).await?;
		Ok(Self {
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(config.evm_config.consensus_state_id.as_bytes());
				consensus_state_id
			},
			state_machine: config.evm_config.state_machine,
			prover,
			config: config.clone(),
			rpc_client: Arc::new(rpc_client),
		})
	}

	pub async fn get_initial_consensus_state(
		&self,
		ismp_contract_address: H160,
	) -> Result<ConsensusState, anyhow::Error> {
		let (header, validators) = self.prover.fetch_finalized_state().await?;

		let consensus_state = ConsensusState {
			frozen_height: None,
			finalized_hash: Header::from(&header).hash::<KeccakHasher>(),
			finalized_validators: validators,
			forks: Default::default(),
			ismp_contract_address,
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
