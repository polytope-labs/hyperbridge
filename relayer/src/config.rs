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

// use grandpa::{GrandpaConfig, GrandpaHost};
use ismp_primitives::HashAlgorithm;
use parachain::ParachainHost;
use primitives::config::RelayerConfig;
use serde::{Deserialize, Serialize};
use tesseract_substrate::{
    config::{Blake2SubstrateChain, KeccakSubstrateChain},
    SubstrateClient, SubstrateConfig,
};

type Parachain<T> = SubstrateClient<ParachainHost, T>;
// type Grandpa<T> = SubstrateClient<GrandpaHost<T>, T>;

crate::chain! {
    KeccakParachain(SubstrateConfig, Parachain<KeccakSubstrateChain>),
    Parachain(SubstrateConfig, Parachain<Blake2SubstrateChain>),
    // Grandpa(GrandpaConfig, Grandpa<Blake2SubstrateChain>),
}

/// Defines the format of the tesseract config.toml file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Configuration options for chain A.
    pub chain_a: AnyConfig,
    /// Configuration options for chain B.
    pub chain_b: AnyConfig,
    /// Configuration options for the relayer.
    pub relayer: RelayerConfig,
}

impl AnyConfig {
    /// Convert the [`Config`] into an implementation of an [`IsmpHost`]
    pub async fn into_client(self) -> Result<AnyClient, anyhow::Error> {
        let client = match self {
            AnyConfig::KeccakParachain(config) | AnyConfig::Parachain(config) => {
                match config.hashing {
                    HashAlgorithm::Keccak => {
                        let host = ParachainHost::default();
                        AnyClient::KeccakParachain(Parachain::new(host, config).await?)
                    }
                    HashAlgorithm::Blake2 => {
                        let host = ParachainHost::default();
                        AnyClient::Parachain(Parachain::new(host, config).await?)
                    }
                }
            } /* AnyConfig::Grandpa(config) => {
               *     let host = GrandpaHost::new(&config).await?;
               *     AnyClient::Grandpa(Grandpa::new(host, config.substrate).await?)
               * } */
        };

        Ok(client)
    }
}
