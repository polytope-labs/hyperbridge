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

/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "consensus-evm-host";

use ismp::{consensus::ConsensusStateId, host::StateMachine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod host;

/// Configuration for the EVM Host consensus client. Empty — this variant
/// is a marker that the chain uses the generic EVM consensus-less path;
/// the caller supplies the `EvmConfig` at construction time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvmHostConfig {}

impl EvmHostConfig {
	/// Convert the config into a client. Caller supplies the chain's EVM host
	/// config.
	pub async fn into_client(self, evm_config: EvmConfig) -> anyhow::Result<Arc<dyn IsmpHost>> {
		Ok(Arc::new(EvmHost::new(&evm_config).await?))
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
		let evm_resolved = ismp_provider.resolved_config();

		Ok(Self {
			consensus_state_id: ismp_provider.consensus_state_id,
			state_machine: ismp_provider.state_machine,
			evm: evm_resolved,
			provider: Arc::new(ismp_provider),
		})
	}
}
