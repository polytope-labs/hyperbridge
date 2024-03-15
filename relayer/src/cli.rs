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

use crate::{config::HyperbridgeConfig, fees, fees::Subcommand, logging};
use anyhow::{anyhow, Context};
use clap::Parser;
use codec::Encode;
use ethers::prelude::H160;
use ismp::host::{Ethereum, StateMachine};
use primitives::IsmpProvider;
use rust_socketio::ClientBuilder;
use sp_core::{ecdsa, ByteArray, Pair};
use std::{collections::HashMap, sync::Arc};
use telemetry_server::{Message, SECRET_KEY};
use tesseract_client::AnyClient;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};
use tesseract_sync_committee::L2Host;
use transaction_fees::TransactionPayment;

/// CLI interface for tesseract relayer.
#[derive(Parser, Debug)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,
	/// Path to the relayer config file
	#[arg(short, long)]
	pub config: String,

	/// Path to the relayer database file
	/// e.g /home/root/dev.db
	#[arg(short, long)]
	pub db: String,

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
		logging::setup()?;
		log::info!("🧊 Initializing tesseract");

		let config = HyperbridgeConfig::parse_conf(&self.config).await?;

		let HyperbridgeConfig { hyperbridge: hyperbridge_config, relayer, .. } = config.clone();

		let _client_map = create_client_map(config.clone()).await?;
		let mut processes = vec![];

		#[cfg(feature = "consensus")]
		{
			let hyperbridge = hyperbridge_config
				.clone()
				.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
				.await?;
			// set up initial consensus states
			if self.setup_eth || self.setup_para {
				initialize_consensus_clients(
					&hyperbridge,
					&_client_map,
					&relayer,
					self.setup_eth,
					self.setup_para,
				)
				.await?;
				log::info!("Initialized consensus states");
			}

			if relayer.consensus.unwrap_or(false) {
				// consensus streams
				for (_, client) in _client_map.iter() {
					processes.push(tokio::spawn(consensus::relay(
						hyperbridge.clone(),
						client.clone(),
						relayer.clone(),
					)));
				}

				if let Some(ref host) = hyperbridge.host {
					let state_machine = host
						.host
						.base_state_machine
						.clone()
						.unwrap_or(StateMachine::Ethereum(Ethereum::ExecutionLayer));
					let ethereum = _client_map.get(&state_machine).ok_or_else(|| {
						anyhow!("Please provide a config option for {state_machine}")
					})?;
					host.spawn_prover(ethereum.clone()).await?;
				}

				log::info!("Initialized consensus tasks");
			}

			if relayer.fisherman.unwrap_or(false) {
				let clients = create_client_map(config.clone()).await?;
				for (_state_machine, client) in clients {
					let hyperbridge = hyperbridge_config
						.clone()
						.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
						.await?;
					processes.push(tokio::spawn(fisherman::fish(hyperbridge, client)));
				}
				log::info!("Initialized fishermen");
			}
		}

		let mut metadata = vec![];
		if relayer.messaging.unwrap_or(true) {
			if relayer.minimum_profit_percentage == 0 {
				log::warn!("Setting the minimum_profit_percentage=0 is not reccomended in live environments!");
			}
			let tx_payment = Arc::new(
				TransactionPayment::initialize(&self.db)
					.await
					.map_err(|err| anyhow!("Error initializing database: {err:?}"))?,
			);
			let clients = create_client_map(config.clone()).await?;
			if config.relayer.delivery_endpoints.is_empty() {
				log::warn!("Delivery endpoints not specified in relayer config, will deliver to all chains.");
			}
			// messaging streams
			for (state_machine, mut client) in clients.clone() {
				// If the delivery endpoint is not empty then we only spawn tasks for chains
				// explicitly mentioned in the config
				if !config.relayer.delivery_endpoints.is_empty() &&
					!config.relayer.delivery_endpoints.contains(&state_machine)
				{
					continue
				}

				let mut hyperbridge = hyperbridge_config
					.clone()
					.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
					.await?;
				hyperbridge.set_latest_finalized_height(&client).await?;
				client.set_latest_finalized_height(&hyperbridge).await?;
				let coprocessor = hyperbridge_config.substrate.chain;
				processes.push(tokio::spawn(messaging::relay(
					hyperbridge,
					client.clone(),
					relayer.clone(),
					coprocessor,
					tx_payment.clone(),
					clients.clone(),
				)));

				metadata
					.push((state_machine, H160::from_slice(&client.address().as_slice()[..20])));
			}

			let hyperbridge = hyperbridge_config
				.clone()
				.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
				.await?;
			tokio::spawn(fees::auto_withdraw(
				hyperbridge,
				clients.clone(),
				config.relayer.clone(),
				tx_payment,
			));

			log::info!("💬 Initialized messaging tasks");
		}

		let socket = tokio::task::spawn_blocking(|| {
			let pair = ecdsa::Pair::from_seed(&SECRET_KEY);
			let mut message = Message { signature: vec![], metadata };
			message.signature = pair.sign(message.metadata.encode().as_slice()).to_raw_vec();
			ClientBuilder::new("http://34.77.39.71:3000")
				.namespace("/")
				.auth(json::to_value(message.clone())?)
				.reconnect(true)
				.reconnect_on_disconnect(true)
				.max_reconnect_attempts(255)
				.on("error", |_err, _| {
					log::error!("Disconnected from telemetry with: {:#?}, reconnecting.", _err)
				})
				.connect()
		})
		.await?;

		let (_result, _index, tasks) = futures::future::select_all(processes).await;

		for task in tasks {
			task.abort();
		}

		if let Ok(socket) = socket {
			socket.disconnect()?;
		}

		Ok(())
	}
}

#[cfg(feature = "consensus")]
/// Initializes the consensus state across all connected chains.
async fn initialize_consensus_clients(
	hyperbridge: &tesseract_substrate::SubstrateClient<
		tesseract_beefy::BeefyHost<Blake2SubstrateChain, KeccakSubstrateChain>,
		KeccakSubstrateChain,
	>,
	chains: &HashMap<StateMachine, AnyClient>,
	relayer: &primitives::config::RelayerConfig,
	setup_eth: bool,
	setup_para: bool,
) -> anyhow::Result<()> {
	use std::collections::BTreeMap;

	use primitives::IsmpHost;

	if setup_eth {
		let initial_state = hyperbridge
			.query_initial_consensus_state()
			.await?
			.ok_or_else(|| anyhow!("Failed to fetch beef consensus state"))?;
		for (state_machine, chain) in chains {
			log::info!("setting consensus state on {state_machine:?}");
			if let Err(err) = chain.set_initial_consensus_state(initial_state.clone()).await {
				log::error!("Failed to set initial consensus state on {state_machine:?}: {err:?}")
			}
		}
	}

	if setup_para {
		let mut addresses = BTreeMap::new();
		for (state_machine, client) in chains {
			log::info!("setting consensus state for {state_machine:?} on hyperbridge");
			let host_manager = client.query_host_manager_address().await?;
			addresses.insert(*state_machine, host_manager);
			if let Some(mut consensus_state) = client.query_initial_consensus_state().await? {
				consensus_state.challenge_period = relayer.challenge_period.unwrap_or_default();
				hyperbridge.set_initial_consensus_state(consensus_state).await?;
			}
		}
		log::info!("setting host manager addresses on on hyperbridge");
		hyperbridge.set_host_manager_addresses(addresses).await?;
	}

	Ok(())
}

pub async fn create_client_map(
	config: HyperbridgeConfig,
) -> anyhow::Result<HashMap<StateMachine, AnyClient>> {
	let HyperbridgeConfig { chains, .. } = config.clone();

	let mut clients = HashMap::new();
	let mut l2_hosts = vec![];

	for (state_machine, config) in chains {
		let client = config
			.into_client()
			.await
			.context(format!("Failed to create client for {state_machine:?}"))?;
		match &client {
			AnyClient::Arbitrum(client) =>
				if let Some(ref host) = client.host {
					l2_hosts.push(L2Host::Arb(host.clone()))
				},
			AnyClient::Base(client) =>
				if let Some(ref host) = client.host {
					l2_hosts.push(L2Host::Base(host.clone()))
				},
			AnyClient::Optimism(client) =>
				if let Some(ref host) = client.host {
					l2_hosts.push(L2Host::Op(host.clone()))
				},
			_ => {},
		}
		clients.insert(state_machine, client);
	}

	let execution_layer = clients.get_mut(&StateMachine::Ethereum(Ethereum::ExecutionLayer));
	if let Some(exec_layer) = execution_layer {
		match exec_layer {
			AnyClient::EthereumSepolia(client) =>
				if let Some(ref mut host) = client.host {
					host.set_l2_hosts(l2_hosts);
				},
			AnyClient::EthereumMainnet(client) =>
				if let Some(ref mut host) = client.host {
					host.set_l2_hosts(l2_hosts);
				},
			_ => unreachable!(),
		}
	}

	Ok(clients)
}
