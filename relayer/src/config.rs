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

//! Tesseract config utilities

use ismp_parachain::consensus::HashAlgorithm;
use parachain::{Blake2Parachain, KeccakParachain, ParachainClient, ParachainConfig};
use serde::{Deserialize, Serialize};

crate::chain! {
    KeccakParachain(ParachainConfig, ParachainClient<KeccakParachain>),
    Parachain(ParachainConfig, ParachainClient<Blake2Parachain>),
}

/// Defines the format of the tesseract config.toml file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Configuration options for chain A.
    pub chain_a: AnyConfig,
    /// Configuration options for chain B.
    pub chain_b: AnyConfig,
}

impl AnyConfig {
    /// Convert the [`Config`] into an implementation of an [`IsmpHost`]
    pub async fn into_client(self) -> Result<AnyClient, anyhow::Error> {
        let client = match self {
            AnyConfig::KeccakParachain(config) | AnyConfig::Parachain(config) => {
                match config.hashing {
                    HashAlgorithm::Keccak => AnyClient::KeccakParachain(
                        ParachainClient::<KeccakParachain>::new(config).await?,
                    ),
                    HashAlgorithm::Blake2 => {
                        AnyClient::Parachain(ParachainClient::<Blake2Parachain>::new(config).await?)
                    }
                }
            }
        };

        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::ParachainConfig;
    use crate::config::AnyConfig;
    use ismp::host::StateMachine;
    use ismp_parachain::consensus::HashAlgorithm;

    #[test]
    fn serialize() {
        let config_a = ParachainConfig {
            hashing: HashAlgorithm::Keccak,
            state_machine: StateMachine::Kusama(2000),
            relay_chain: "ws://localhost:9944".to_string(),
            parachain: "ws://localhost:9988".to_string(),
            signer: "0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a"
                .to_string(),
            latest_state_machine_height: None,
        };

        let value = toml::to_string(&AnyConfig::Parachain(config_a)).unwrap();

        println!("{value}");
    }
}
