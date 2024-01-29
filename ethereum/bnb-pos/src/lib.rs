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

use bnb_pos_prover::BnbPosProver;
use bnb_pos_verifier::primitives::compute_epoch;
pub use bnb_pos_verifier::verify_bnb_header;
use ethers::providers::{Provider, Ws};
pub use geth_primitives::Header;
use ismp::{consensus::ConsensusStateId, host::StateMachine, util::Keccak256};
pub use ismp_bnb_pos::ConsensusState;
use primitive_types::H160;
use reconnecting_jsonrpsee_ws_client::{Client, ExponentialBackoff, PingConfig};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tesseract_evm::{EvmClient, EvmConfig};

mod byzantine;
mod host;
mod notification;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BnbPosConfig {
	/// General ethereum config
	#[serde[flatten]]
	pub evm_config: EvmConfig,
}

impl BnbPosConfig {
	/// Convert the config into a client.
	pub async fn into_client(self) -> anyhow::Result<EvmClient<BnbPosHost>> {
		let host = BnbPosHost::new(&self).await?;
		let client = EvmClient::new(Some(host), self.evm_config).await?;

		Ok(client)
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

#[derive(Clone)]
pub struct BnbPosHost {
	/// Consensus state id on counterparty chain
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this chain.
	pub state_machine: StateMachine,
	/// Consensus prover
	pub prover: BnbPosProver,
	/// Config
	pub config: BnbPosConfig,
	/// Jsonrpsee client for event susbscription, ethers does not expose a Send and Sync stream for
	/// susbcribing to contract logs
	pub rpc_client: Arc<Client>,
}

impl BnbPosHost {
	pub async fn new(config: &BnbPosConfig) -> Result<Self, anyhow::Error> {
		let provider =
			Provider::<Ws>::connect_with_reconnects(config.evm_config.execution_ws.clone(), 1000)
				.await
				.unwrap();
		let prover = BnbPosProver::new(provider);
		let rpc_client = Client::builder()
			.retry_policy(ExponentialBackoff::from_millis(100))
			.enable_ws_ping(
				PingConfig::new()
					.ping_interval(Duration::from_secs(6))
					.inactive_limit(Duration::from_secs(30)),
			)
			.build(config.evm_config.execution_ws.clone())
			.await?;
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

	pub async fn get_consensus_state<I: Keccak256>(
		&self,
		ismp_contract_address: H160,
	) -> Result<ConsensusState, anyhow::Error> {
		let (header, current_validators) =
			self.prover.fetch_finalized_state::<KeccakHasher>().await?;
		let latest_header = self.prover.latest_header().await?;
		if latest_header.number.low_u64() - header.number.low_u64() < 12 {
			// We want to ensure the current validators are signing before creating the consensus
			// state
			tokio::time::sleep(Duration::from_secs(
				(latest_header.number.low_u64() - header.number.low_u64()) * 12,
			))
			.await;
		}
		let consensus_state = ConsensusState {
			current_validators,
			next_validators: None,
			finalized_hash: Header::from(&header).hash::<KeccakHasher>(),
			finalized_height: header.number.as_u64(),
			ismp_contract_address,
			current_epoch: compute_epoch(header.number.low_u64()),
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
