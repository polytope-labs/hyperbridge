// Copyright (C) Polytope Labs Ltd.
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

//! Tesseract CLI utilities

use crate::{
    config::{AnyClient, HyperbridgeConfig},
    logging,
};
use clap::Parser;
use tokio::join;

/// Tesseract, the multi-chain ISMP relayer
#[derive(Parser, Debug)]
pub struct Cli {
    /// Path to the relayer config file
    #[arg(short, long)]
    config: String,
}

impl Cli {
    /// Run the relayer
    pub async fn run(self) -> Result<(), anyhow::Error> {
        logging::setup();

        let config = tokio::fs::read_to_string(&self.config).await?;

        let HyperbridgeConfig { hyperbridge, ethereum, arbitrum, optimism, relayer } =
            toml::from_str::<HyperbridgeConfig>(&config)?;

        let hyperbridge = hyperbridge.into_client().await?;
        let mut ethereum = ethereum.into_client().await?;
        let arbitrum = arbitrum.into_client().await?;
        let optimism = optimism.into_client().await?;
        // let base = base.into_client().await?;

        if let AnyClient::Ethereum(ref mut ethereum) = ethereum {
            if let AnyClient::Arbitrum(ref client) = arbitrum {
                ethereum.host.set_arb_host(client.host.clone());
            }
            if let AnyClient::Optimism(ref client) = optimism {
                ethereum.host.set_op_host(client.host.clone());
            }
        }

        let a = tokio::spawn(consensus::relay(hyperbridge.clone(), ethereum.clone()));
        let b = tokio::spawn(consensus::relay(hyperbridge.clone(), arbitrum.clone()));
        let c = tokio::spawn(consensus::relay(hyperbridge.clone(), optimism.clone()));
        // let d = tokio::spawn(consensus::relay(hyperbridge.clone(), base.clone()));
        let e =
            tokio::spawn(messaging::relay(hyperbridge.clone(), ethereum, Some(relayer.clone())));
        let f =
            tokio::spawn(messaging::relay(hyperbridge.clone(), arbitrum, Some(relayer.clone())));
        let g =
            tokio::spawn(messaging::relay(hyperbridge.clone(), optimism, Some(relayer.clone())));
        // let h = tokio::spawn(messaging::relay(hyperbridge.clone(), base, Some(relayer.clone())));

        let _ = join!(a, b, c, e, f, g);

        Ok(())
    }
}
