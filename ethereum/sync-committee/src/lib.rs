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

use consensus_client::types::ConsensusState;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use primitive_types::H160;
use primitives::{types::LightClientState, util::compute_epoch_at_slot};
use prover::SyncCommitteeProver;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesseract_evm::{
    arbitrum::client::{ArbConfig, ArbHost},
    optimism::client::{OpConfig, OpHost},
    EvmConfig,
};

mod byzantine;
mod host;
mod notification;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCommitteeConfig {
    /// Http url for a beacon client
    pub beacon_node_url: String,
    /// State machine Identifier for this client on it's counterparties.
    pub state_machine: StateMachine,
    /// Arbitrum host config
    pub arbitrum_execution_client: Option<ArbConfig>,
    /// Optimism host config
    pub optimism_execution_client: Option<OpConfig>,
    /// Consensus state id on counterparty chain
    pub consensus_state_id: String,
    /// General ethereum config
    pub evm_config: Option<EvmConfig>,
}

#[derive(Clone)]
pub struct SyncCommitteeHost {
    /// Consensus state id on counterparty chain
    pub consensus_state_id: ConsensusStateId,
    /// State machine Identifier for this chain.
    pub state_machine: StateMachine,
    /// Arbitrum  client
    pub arbitrum_client: Option<ArbHost>,
    /// Optimism  client
    pub optimism_client: Option<OpHost>,
    /// Consensus prover
    pub prover: SyncCommitteeProver,
    /// Http URl beacon chain, required for subscribing to events SSE
    pub beacon_node_rpc: String,
}

impl SyncCommitteeHost {
    pub async fn new(config: SyncCommitteeConfig) -> Result<Self, anyhow::Error> {
        let prover = SyncCommitteeProver::new(config.beacon_node_url.clone());
        Ok(Self {
            consensus_state_id: {
                let mut consensus_state_id: ConsensusStateId = Default::default();
                consensus_state_id.copy_from_slice(config.consensus_state_id.as_bytes());
                consensus_state_id
            },
            state_machine: config.state_machine,
            arbitrum_client: if let Some(config) = config.arbitrum_execution_client.as_ref() {
                Some(ArbHost::new(config.clone()).await?)
            } else {
                None
            },
            optimism_client: if let Some(config) = config.optimism_execution_client.as_ref() {
                Some(OpHost::new(config.clone()).await?)
            } else {
                None
            },
            prover,
            beacon_node_rpc: config.beacon_node_url.clone(),
        })
    }

    pub async fn get_initial_consensus_state(
        &self,
        ismp_contract_addresses: BTreeMap<StateMachine, H160>,
        l2_oracle: BTreeMap<StateMachine, H160>,
        rollup_core: H160,
    ) -> Result<ConsensusState, anyhow::Error> {
        let block_id = "head";
        let block_header = self.prover.fetch_header(&block_id).await?;
        let state = self.prover.fetch_beacon_state(&block_header.slot.to_string()).await?;

        let client_state = LightClientState {
            finalized_header: block_header.clone(),
            latest_finalized_epoch: compute_epoch_at_slot(block_header.slot),
            current_sync_committee: state.current_sync_committee,
            next_sync_committee: state.next_sync_committee,
        };

        let consensus_state = ConsensusState {
            frozen_height: None,
            light_client_state: client_state,
            ismp_contract_addresses,
            l2_oracle_address: l2_oracle,
            rollup_core_address: rollup_core,
        };

        Ok(consensus_state)
    }
}
