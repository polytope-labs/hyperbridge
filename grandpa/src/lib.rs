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

use hex_literal::hex;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use ismp_grandpa_prover::GrandpaProver;
use serde::{Deserialize, Serialize};
use tesseract_substrate::SubstrateConfig;
use subxt::{config::Header, ext::sp_runtime::traits::Zero, OnlineClient};

mod byzantine;
mod host;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrandpaConfig {
    /// RPC url for a standalone chain or relay chain
    pub chain: String,
    /// State machine Identifier for this client on it's counterparties.
    pub state_machine: StateMachine,
    /// Consensus state id on counterparty chain
    pub consensus_state_id: ConsensusStateId,
    /// substrate config options
    #[serde(flatten)]
    pub substrate: SubstrateConfig,
    /// grandpa prover config
    #[serde(flatten)]
    pub grandpa_prover_config: GrandpaProverConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrandpaProverConfig {
    /// para ids
    pub para_ids: Vec<u32>,
    /// Raw storage key for the babe epoch start storage value
    pub babe_epoch_start_key: Option<Vec<u8>>,
    /// Raw Storage key for the current set id in pallet grandpa
    pub current_set_id: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct GrandpaHost<T: subxt::Config> {
    /// Consensus state id on counterparty chain
    pub consensus_state_id: ConsensusStateId,
    /// State machine Identifier for this chain.
    pub state_machine: StateMachine,
    /// Subxt client for the chain.
    pub client: OnlineClient<T>,
    /// Grandpa prover
    pub prover: GrandpaProver<T>,
}

impl<T> GrandpaHost<T>
where
    T: subxt::Config + Send + Sync + Clone,
    <T::Header as Header>::Number: Ord + Zero,
    u32: From<<T::Header as Header>::Number>,
    sp_core::H256: From<T::Hash>,
    T::Header: codec::Decode,
{
    pub async fn new(config: &GrandpaConfig) -> Result<Self, anyhow::Error> {
        let client = OnlineClient::from_url(&config.chain).await?;
        let default_babe_epoch_start_key: [u8; 32] =
            hex!("1cb6f36e027abb2091cfb5110ab5087fe90e2fbf2d792cb324bffa9427fe1f0e");
        let default_current_set_id_key: [u8; 32] =
            hex!("5f9cc45b7a00c5899361e1c6099678dc8a2d09463effcc78a22d75b9cb87dffc");
        let prover = GrandpaProver::new(
            &config.chain,
            config.grandpa_prover_config.para_ids.clone(),
            config.state_machine,
            config
                .grandpa_prover_config
                .babe_epoch_start_key
                .clone()
                .unwrap_or(default_babe_epoch_start_key.to_vec()),
            config
                .grandpa_prover_config
                .current_set_id
                .clone()
                .unwrap_or(default_current_set_id_key.to_vec()),
        )
        .await?;
        Ok(GrandpaHost {
            consensus_state_id: config.consensus_state_id.clone(),
            state_machine: config.state_machine,
            client,
            prover,
        })
    }
}
