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
use ismp_primitives::HashAlgorithm;
use parking_lot::Mutex;
use primitives::IsmpHost;
use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, sr25519, Pair};
use std::sync::Arc;
use subxt::{config::Header, OnlineClient};

mod calls;
pub mod config;
mod extrinsic;
mod host;
mod provider;
#[cfg(feature = "testing")]
mod testing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateConfig {
    /// State machine Identifier for this client.
    pub state_machine: StateMachine,
    /// The hashing algorithm that substrate chain uses.
    pub hashing: HashAlgorithm,
    /// Consensus state id
    pub consensus_state_id: String,
    /// RPC url for the parachain
    pub ws_url: String,
    /// Relayer account seed
    pub signer: String,
    /// Latest state machine height
    pub latest_state_machine_height: Option<u64>,
}

/// Core substrate client.
pub struct SubstrateClient<I, C: subxt::Config> {
    /// Ismp host implementation
    host: I,
    /// Subxt client for the substrate chain
    client: OnlineClient<C>,
    /// Consensus state Id
    consensus_state_id: ConsensusStateId,
    /// State machine Identifier for this client.
    state_machine: StateMachine,
    /// The hashing algorithm that substrate chain uses.
    hashing: HashAlgorithm,
    /// Private key of the signing account
    signer: sr25519::Pair,
    /// Latest state machine height.
    latest_state_machine_height: Arc<Mutex<u64>>,
}

impl<T, C> SubstrateClient<T, C>
where
    T: IsmpHost,
    C: subxt::Config,
{
    pub async fn new(host: T, config: SubstrateConfig) -> Result<Self, anyhow::Error> {
        let client = OnlineClient::<C>::from_url(&config.ws_url).await?;
        // If latest height of the state machine on the counterparty is not provided in config
        // Set it to the latest parachain height
        let latest_state_machine_height =
            if let Some(latest_state_machine_height) = config.latest_state_machine_height {
                latest_state_machine_height
            } else {
                client
                    .rpc()
                    .header(None)
                    .await?
                    .expect("block header should be available")
                    .number()
                    .into()
            };
        let bytes = from_hex(&config.signer)?;
        let signer = sr25519::Pair::from_seed_slice(&bytes)?;
        let mut consensus_state_id: ConsensusStateId = Default::default();
        consensus_state_id.copy_from_slice(config.consensus_state_id.as_bytes());

        Ok(Self {
            host,
            client,
            consensus_state_id,
            state_machine: config.state_machine,
            hashing: config.hashing,
            signer,
            latest_state_machine_height: Arc::new(Mutex::new(latest_state_machine_height)),
        })
    }
}
