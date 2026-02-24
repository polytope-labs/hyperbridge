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

use ismp::{consensus::ConsensusStateId, host::StateMachine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod host;

/// Configuration for the EVM Host consensus client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmHostConfig {
	/// General EVM config
	#[serde(flatten)]
	pub evm_config: EvmConfig,
}

impl EvmHostConfig {
	/// Convert the config into a client.
	pub async fn into_client(self) -> anyhow::Result<Arc<dyn IsmpHost>> {
		Ok(Arc::new(EvmHost::new(&self.evm_config).await?))
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

/// EVM Host consensus client
///
/// This is a dummy consensus client that never produces consensus updates.
/// It holds an EVM client that can be used to submit updates to other chains.
/// This is useful for chains whose consensus is managed elsewhere (e.g., by a relay chain)
/// and only need to provide an EVM interface for submitting messages.
#[derive(Clone)]
pub struct EvmHost {
	/// Consensus state id on counterparty chain
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this chain.
	pub state_machine: StateMachine,
	/// Evm config options
	pub evm: EvmConfig,
	/// Ismp provider (EVM client)
	pub provider: Arc<dyn IsmpProvider>,
}

impl EvmHost {
	pub async fn new(evm: &EvmConfig) -> Result<Self, anyhow::Error> {
		let ismp_provider = EvmClient::new(evm.clone()).await?;

		Ok(Self {
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(evm.consensus_state_id.as_bytes());
				consensus_state_id
			},
			state_machine: evm.state_machine,
			evm: evm.clone(),
			provider: Arc::new(ismp_provider),
		})
	}
}
