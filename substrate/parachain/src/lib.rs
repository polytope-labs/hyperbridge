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

//! Parachain client implementation for tesseract.

use ismp::host::StateMachine;
use serde::{Deserialize, Serialize};
use substrate_common::SubstrateConfig;
use subxt::{OnlineClient, PolkadotConfig};

mod byzantine;
mod host;
mod relay_chain;

pub use relay_chain::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParachainConfig {
    /// RPC url for the relay chain. Unneeded if the host is a parachain.
    pub relay_chain: String,

    /// substrate config options
    #[serde(flatten)]
    pub substrate: SubstrateConfig,
}

#[derive(Clone)]
pub struct ParachainHost<T: subxt::Config> {
    /// State machine Identifier for this client.
    pub state_machine: StateMachine,
    /// Subxt client for the relay chain. Unneeded if the host is a parachain.
    relay_chain: OnlineClient<PolkadotConfig>,
    /// Subxt client for the parachain.
    parachain: OnlineClient<T>,
}

impl<T> ParachainHost<T>
where
    T: subxt::Config + Send + Sync + Clone,
{
    pub async fn new(config: &ParachainConfig) -> Result<Self, anyhow::Error> {
        let relay_chain = OnlineClient::from_url(&config.relay_chain).await?;
        let parachain = OnlineClient::<T>::from_url(&config.substrate.ws_url).await?;

        Ok(ParachainHost { state_machine: config.substrate.state_machine, relay_chain, parachain })
    }
}
