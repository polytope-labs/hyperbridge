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
	ext_queue, SubstrateClient,
};
use tesseract_sync_committee::SyncCommitteeHost;

/// CLI interface for tesseract relayer.
#[derive(Parser, Debug)]
pub struct Cli {
	/// Path to the relayer config file
	#[arg(short, long)]
	config: String,

	/// Should we initialize the relevant consensus states on Eth chains?
	#[arg(short, long)]
	setup_eth: bool,

	/// Should we initialize the relevant Consensus state on hyperbridge?
	#[arg(short, long)]
	setup_para: bool,
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

		let mut hyperbridge = hyperbridge_config
			.clone()
			.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
			.await?;
		// extrinsic submission pipeline
		let queue = ext_queue::init_queue(hyperbridge.client.clone(), hyperbridge.signer.clone())?;
		hyperbridge.set_queue(queue.clone());

		let mut ethereum = eth_config.clone().into_client(&hyperbridge).await?;
		let mut arbitrum = arb_config.clone().into_client(&hyperbridge).await?;
		let mut optimism = op_config.clone().into_client(&hyperbridge).await?;
		let mut base = base_config.clone().into_client(&hyperbridge).await?;

		let eth_tx_queue = tesseract_evm::tx_queue::init_queue(
			ethereum.signer.clone(),
			eth_config.evm_config.handler,
			eth_config.evm_config.ismp_host,
		)?;
		let arb_tx_queue = tesseract_evm::tx_queue::init_queue(
			arbitrum.signer.clone(),
			arb_config.evm_config.handler,
			arb_config.evm_config.ismp_host,
		)?;
		let op_tx_queue = tesseract_evm::tx_queue::init_queue(
			optimism.signer.clone(),
			op_config.evm_config.handler,
			op_config.evm_config.ismp_host,
		)?;
		let base_tx_queue = tesseract_evm::tx_queue::init_queue(
			base.signer.clone(),
			base_config.evm_config.handler,
			base_config.evm_config.ismp_host,
		)?;
		ethereum.host.set_arb_host(arbitrum.host.clone());
		ethereum.host.set_op_host(optimism.host.clone());
		ethereum.host.set_base_host(base.host.clone());
		ethereum.set_queue(eth_tx_queue.clone());
		arbitrum.set_queue(arb_tx_queue.clone());
		optimism.set_queue(op_tx_queue.clone());
		base.set_queue(base_tx_queue.clone());

		// set up initial consensus states
		if self.setup_eth || self.setup_para {
			initialize_consensus_clients(
				&hyperbridge,
				&ethereum,
				&arbitrum,
				&optimism,
				&base,
				config,
				self.setup_eth,
				self.setup_para,
			)
			.await?;
			log::info!("Initialized consensus states");
		}

		let mut processes = vec![];
		if relayer.consensus {
			// consensus streams
			processes.push(tokio::spawn(consensus::relay(hyperbridge.clone(), ethereum)));
			processes.push(tokio::spawn(consensus::relay(hyperbridge.clone(), arbitrum)));
			processes.push(tokio::spawn(consensus::relay(hyperbridge.clone(), optimism)));
			processes.push(tokio::spawn(consensus::relay(hyperbridge.clone(), base)));
			log::info!("Initialized consensus streams");
		}

		if relayer.messaging {
			let mut ethereum = eth_config.into_client(&hyperbridge).await?;
			let mut arbitrum = arb_config.into_client(&hyperbridge).await?;
			let mut optimism = op_config.into_client(&hyperbridge).await?;
			let mut base = base_config.into_client(&hyperbridge).await?;
			ethereum.set_queue(eth_tx_queue);
			arbitrum.set_queue(arb_tx_queue);
			optimism.set_queue(op_tx_queue);
			base.set_queue(base_tx_queue);
			// messaging streams
			processes.push(tokio::spawn(messaging::relay(
				hyperbridge,
				ethereum,
				Some(relayer.clone()),
			)));
			let mut hyperbridge = hyperbridge_config
				.clone()
				.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
				.await?;
			hyperbridge.set_queue(queue.clone());

			processes.push(tokio::spawn(messaging::relay(
				hyperbridge,
				arbitrum,
				Some(relayer.clone()),
			)));
			let mut hyperbridge = hyperbridge_config
				.clone()
				.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
				.await?;
			hyperbridge.set_queue(queue.clone());

			processes.push(tokio::spawn(messaging::relay(
				hyperbridge,
				optimism,
				Some(relayer.clone()),
			)));

			let mut hyperbridge = hyperbridge_config
				.clone()
				.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
				.await?;
			hyperbridge.set_queue(queue.clone());

			processes.push(tokio::spawn(messaging::relay(
				hyperbridge,
				base,
				Some(relayer.clone()),
			)));
			log::info!("Initialized messaging streams");
		}

		let _ = futures::future::join_all(processes).await;

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
	setup_eth: bool,
	setup_para: bool,
) -> anyhow::Result<()> {
	let HyperbridgeConfig {
		ethereum: eth_config,
		arbitrum: arb_config,
		optimism: op_config,
		base: base_config,
		..
	} = config;

	if setup_eth {
		let initial_state: BeefyConsensusState =
			hyperbridge.host.prover.get_initial_consensus_state().await?.into();
		log::info!("setting consensus state on ethereum");
		ethereum.set_consensus_state(initial_state.clone().encode()).await?;
		log::info!("setting consensus state on abitrum");
		arbitrum.set_consensus_state(initial_state.clone().encode()).await?;
		log::info!("setting consensus state on optimism");
		optimism.set_consensus_state(initial_state.clone().encode()).await?;
		log::info!("setting consensus state on base");
		base.set_consensus_state(initial_state.clone().encode()).await?;
	}

	if setup_para {
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
			.get_initial_consensus_state(
				ismp_contract_addresses,
				l2_oracle,
				arb_config.rollup_core,
				None,
			)
			.await?;
		let consensus_state_id = {
			let mut tmp = [0u8; 4];
			tmp.copy_from_slice(eth_config.evm_config.consensus_state_id.as_bytes());
			tmp
		};

		log::info!("setting consensus state on hyperbridge");
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
	}
	Ok(())
}
