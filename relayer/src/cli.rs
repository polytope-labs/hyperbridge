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
use futures::FutureExt;
use ismp::host::StateMachine;
use primitives::IsmpProvider;
use rust_socketio::asynchronous::ClientBuilder;
use sp_core::{ecdsa, ByteArray, Pair};
use std::{collections::HashMap, sync::Arc};
use telemetry_server::Message;
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
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
}

impl Cli {
	/// Run the relayer
	pub async fn run(self) -> Result<(), anyhow::Error> {
		logging::setup()?;
		log::info!("🧊 Initializing tesseract");
		let config = HyperbridgeConfig::parse_conf(&self.config).await?;
		let HyperbridgeConfig { hyperbridge: hyperbridge_config, relayer, .. } = config.clone();

		let mut processes = vec![];
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
			let mut clients = create_client_map(config.clone()).await?;
			// Add hyperbridge to the client map
			let hyperbridge =
				SubstrateClient::<KeccakSubstrateChain>::new(hyperbridge_config.clone()).await?;
			clients.insert(hyperbridge.state_machine_id().state_id, Arc::new(hyperbridge.clone()));

			if config.relayer.delivery_endpoints.is_empty() {
				log::warn!("Delivery endpoints not specified in relayer config, will deliver to all chains.");
			}

			// messaging tasks
			for (state_machine, mut client) in &mut clients {
				// If the delivery endpoint is not empty then we only spawn tasks for chains
				// explicitly mentioned in the config
				if !config.relayer.delivery_endpoints.is_empty() &&
					!config.relayer.delivery_endpoints.contains(&state_machine)
				{
					continue;
				}

				let Some(ref mut inner) = Arc::get_mut(&mut client) else {
					Err(anyhow!("Failed to get mutable reference for client {state_machine:?}"))?
				};
				inner.set_latest_finalized_height(Arc::new(hyperbridge.clone())).await?;
				metadata.push((
					state_machine.clone(),
					H160::from_slice(&client.address().as_slice()[..20]),
				));
			}

			for (state_machine, client) in &clients {
				// If the delivery endpoint is not empty then we only spawn tasks for chains
				// explicitly mentioned in the config
				if !config.relayer.delivery_endpoints.is_empty() &&
					!config.relayer.delivery_endpoints.contains(&state_machine)
				{
					continue;
				}

				let mut new_hyperbridge =
					SubstrateClient::<KeccakSubstrateChain>::new(hyperbridge_config.clone())
						.await?;
				new_hyperbridge.set_latest_finalized_height(client.clone()).await?;

				let coprocessor = hyperbridge_config.state_machine;
				processes.push(tokio::spawn(messaging::relay(
					new_hyperbridge,
					client.clone(),
					relayer.clone(),
					coprocessor,
					tx_payment.clone(),
					clients.clone(),
				)));

				metadata.push((
					state_machine.clone(),
					H160::from_slice(&client.address().as_slice()[..20]),
				));
			}

			tokio::spawn(fees::auto_withdraw(
				hyperbridge,
				clients.clone(),
				config.relayer.clone(),
				tx_payment,
			));

			log::info!("💬 Initialized messaging tasks");
		}

		let socket = {
			if let Some(key) = option_env!("TELEMETRY_SECRET_KEY") {
				let bytes = hex::decode(key)?;
				let pair = ecdsa::Pair::from_seed_slice(&bytes)
					.expect("TELEMETRY_SECRET_KEY must be 64 chars!");
				let mut message = Message { signature: vec![], metadata };
				message.signature = pair.sign(message.metadata.encode().as_slice()).to_raw_vec();
				// todo: use compile-time env for telemetry url
				let client = ClientBuilder::new("https://hyperbridge-telemetry.blockops.network/")
					.namespace("/")
					.auth(json::to_value(message.clone())?)
					.reconnect(true)
					.reconnect_on_disconnect(true)
					.max_reconnect_attempts(u8::MAX)
					.on("open", |_, _| async move { log::info!("Connected to telemetry") }.boxed())
					.on("error", |_err, _| {
						async move {
							log::error!(
								"Disconnected from telemetry with: {:#?}, reconnecting.",
								_err
							)
						}
						.boxed()
					})
					.connect()
					.await?;

				Some(client)
			} else {
				None
			}
		};

		let (_result, _index, tasks) = futures::future::select_all(processes).await;

		for task in tasks {
			task.abort();
		}

		if let Some(socket) = socket {
			socket.disconnect().await?;
		}

		Ok(())
	}
}

pub async fn create_client_map(
	config: HyperbridgeConfig,
) -> anyhow::Result<HashMap<StateMachine, Arc<dyn IsmpProvider>>> {
	let HyperbridgeConfig { chains, .. } = config.clone();
	let mut clients = HashMap::new();

	for (state_machine, config) in chains {
		let client = config
			.into_client()
			.await
			.context(format!("Failed to create client for {state_machine:?}"))?;
		clients.insert(state_machine, client);
	}

	Ok(clients)
}
