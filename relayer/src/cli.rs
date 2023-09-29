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

use crate::{config::HyperbridgeConfig, logging};
use clap::Parser;
use codec::Encode;
use ethers::abi::AbiEncode;
use ismp::{
    host::{Ethereum, StateMachine},
    messaging::CreateConsensusState,
};
use std::collections::BTreeMap;
use tesseract_beefy::BeefyHost;
use tesseract_evm::{
    abi::BeefyConsensusState, arbitrum::client::ArbHost, optimism::client::OpHost, EvmClient,
};
use tesseract_substrate::{
    config::{Blake2SubstrateChain, KeccakSubstrateChain},
    SubstrateClient,
};
use tesseract_sync_committee::SyncCommitteeHost;
use tokio::join;

/// CLI interface for tesseract relayer.
#[derive(Parser, Debug)]
pub struct Cli {
    /// Path to the relayer config file
    #[arg(short, long)]
    config: String,

    /// Should we initialize the relevant consensus states?
    #[arg(short, long)]
    setup: bool,
}

impl Cli {
    /// Run the relayer
    pub async fn run(self) -> Result<(), anyhow::Error> {
        logging::setup();
        log::info!("Initializing tesseract");

        let config = {
            let toml = tokio::fs::read_to_string(&self.config).await?;
            toml::from_str::<HyperbridgeConfig>(&toml)?
        };

        let HyperbridgeConfig {
            hyperbridge: hyperbridge_config,
            ethereum: eth_config,
            arbitrum: arb_config,
            optimism: op_config,
            base: base_config,
            relayer,
        } = config.clone();

        let hyperbridge = hyperbridge_config
            .clone()
            .into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
            .await?;
        log::info!("Conected to the hyperbridge");
        let mut ethereum = eth_config.clone().into_client().await?;
        log::info!("Conected to ethereum");
        let arbitrum = arb_config.clone().into_client().await?;
        log::info!("Conected to arbitrum");
        let optimism = op_config.clone().into_client().await?;
        log::info!("Conected to optimism");
        let base = base_config.clone().into_client().await?;
        log::info!("Conected to base");

        ethereum.host.set_arb_host(arbitrum.host.clone());
        ethereum.host.set_op_host(optimism.host.clone());
        ethereum.host.set_base_host(base.host.clone());

        // set up initial consensus states
        if self.setup {
            log::info!("Initializing consensus states");
            initialize_consensus_clients(
                &hyperbridge,
                &ethereum,
                &arbitrum,
                &optimism,
                &base,
                config,
            )
            .await?;
        }

        // consensus streams
        let a = tokio::spawn(consensus::relay(hyperbridge.clone(), ethereum.clone()));
        let b = tokio::spawn(consensus::relay(hyperbridge.clone(), arbitrum.clone()));
        let c = tokio::spawn(consensus::relay(hyperbridge.clone(), optimism.clone()));
        let d = tokio::spawn(consensus::relay(hyperbridge.clone(), base.clone()));

        // messaging streams
        let e =
            tokio::spawn(messaging::relay(hyperbridge.clone(), ethereum, Some(relayer.clone())));
        let f =
            tokio::spawn(messaging::relay(hyperbridge.clone(), arbitrum, Some(relayer.clone())));
        let g =
            tokio::spawn(messaging::relay(hyperbridge.clone(), optimism, Some(relayer.clone())));
        let h = tokio::spawn(messaging::relay(hyperbridge.clone(), base, Some(relayer.clone())));

        let _ = join!(a, b, c, d, e, f, g, h);

        Ok(())
    }
}

type HyperbridgeChain =
    SubstrateClient<BeefyHost<Blake2SubstrateChain, KeccakSubstrateChain>, KeccakSubstrateChain>;

/// Initializes the consensus state across all connected chains.
async fn initialize_consensus_clients(
    hyperbridge: &HyperbridgeChain,
    ethereum: &EvmClient<SyncCommitteeHost>,
    arbitrum: &EvmClient<ArbHost>,
    optimism: &EvmClient<OpHost>,
    base: &EvmClient<OpHost>,
    config: HyperbridgeConfig,
) -> anyhow::Result<()> {
    let HyperbridgeConfig {
        ethereum: eth_config,
        arbitrum: arb_config,
        optimism: op_config,
        base: base_config,
        ..
    } = config;

    let initial_state: BeefyConsensusState =
        hyperbridge.host.prover.get_initial_consensus_state().await?.into();
    ethereum.set_consensus_state(initial_state.clone().encode()).await?;
    arbitrum.set_consensus_state(initial_state.clone().encode()).await?;
    optimism.set_consensus_state(initial_state.clone().encode()).await?;
    base.set_consensus_state(initial_state.clone().encode()).await?;

    let ismp_contract_addresses = BTreeMap::from([
        (StateMachine::Ethereum(Ethereum::ExecutionLayer), eth_config.evm_config.ismp_host),
        (StateMachine::Ethereum(Ethereum::Arbitrum), arb_config.evm_config.ismp_host),
        (StateMachine::Ethereum(Ethereum::Optimism), op_config.evm_config.ismp_host),
        (StateMachine::Ethereum(Ethereum::Base), base_config.evm_config.ismp_host),
    ]);

    let l2_oracle = BTreeMap::from([
        (StateMachine::Ethereum(Ethereum::Optimism), op_config.l2_oracle),
        (StateMachine::Ethereum(Ethereum::Base), base_config.l2_oracle),
    ]);

    let beacon_consensus_state = ethereum
        .host
        .get_initial_consensus_state(ismp_contract_addresses, l2_oracle, arb_config.rollup_core)
        .await?;
    let consensus_state_id = {
        let mut tmp = [0u8; 4];
        tmp.copy_from_slice(eth_config.evm_config.consensus_state_id.as_bytes());
        tmp
    };

    hyperbridge
        .create_consensus_state(CreateConsensusState {
            consensus_state: beacon_consensus_state.encode(),
            consensus_client_id: ismp_sync_committee::BEACON_CONSENSUS_ID,
            consensus_state_id,
            unbonding_period: 60 * 60 * 60 * 27,
            challenge_period: 0,
            state_machine_commitments: vec![],
        })
        .await?;

    Ok(())
}
