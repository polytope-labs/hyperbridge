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
use ethers::prelude::H160;
use futures::FutureExt;
use ismp::host::StateMachine;
use polkadot_sdk::sc_service::TaskManager;
use std::{collections::HashMap, sync::Arc};
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use transaction_fees::TransactionPayment;

/// CLI interface for tesseract relayer.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
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

		let mut metadata = vec![];
		let tokio_handle = tokio::runtime::Handle::current();
		let mut task_manager = TaskManager::new(tokio_handle, None)?;

		if relayer.minimum_profit_percentage == 0 {
			log::warn!(
				"Setting the minimum_profit_percentage=0 is not reccomended in live environments!"
			);
		}

		let tx_payment = Arc::new(
			TransactionPayment::initialize(&self.db)
				.await
				.map_err(|err| anyhow!("Error initializing database: {err:?}"))?,
		);
		// Add hyperbridge to the client map
		let hyperbridge =
			SubstrateClient::<KeccakSubstrateChain>::new(hyperbridge_config.clone()).await?;
		let mut clients = create_client_map(config.clone(), Arc::new(hyperbridge.clone())).await?;
		clients.insert(hyperbridge.state_machine_id().state_id, Arc::new(hyperbridge.clone()));

		if config.relayer.delivery_endpoints.is_empty() {
			log::warn!(
				"Delivery endpoints not specified in relayer config, will deliver to all chains."
			);
		}

		// messaging tasks
		for (state_machine, client) in &clients {
			if *state_machine == hyperbridge_config.state_machine {
				continue;
			}
			// If the delivery endpoint is not empty then we only spawn tasks for chains
			// explicitly mentioned in the config
			if !config.relayer.delivery_endpoints.is_empty() &&
				!config.relayer.delivery_endpoints.contains(&state_machine.to_string())
			{
				continue;
			}

			let mut new_hyperbridge =
				SubstrateClient::<KeccakSubstrateChain>::new(hyperbridge_config.clone()).await?;
			new_hyperbridge.set_latest_finalized_height(client.clone()).await?;

			let coprocessor = hyperbridge_config.state_machine;

			tesseract_messaging::relay(
				new_hyperbridge.clone(),
				client.clone(),
				relayer.clone(),
				coprocessor,
				tx_payment.clone(),
				clients.clone(),
				&task_manager,
			)
			.await?;

			if relayer.fisherman.unwrap_or_default() {
				tesseract_fisherman::fish(
					Arc::new(new_hyperbridge),
					client.clone(),
					&task_manager,
					coprocessor,
				)
				.await?
			}

			metadata.push((
				state_machine.clone(),
				H160::from_slice(&client.address().as_slice()[..20]),
			));
		}

		task_manager.spawn_essential_handle().spawn(
			"auto-withdraw",
			"fees",
			async move {
				let _ =
					fees::auto_withdraw(hyperbridge, clients, config.relayer.clone(), tx_payment)
						.await;
			}
			.boxed(),
		);

		log::info!("💬 Initialized messaging tasks");

		task_manager.future().await?;

		Ok(())
	}
}

pub async fn create_client_map(
	config: HyperbridgeConfig,
	hyperbridge: Arc<dyn IsmpProvider>,
) -> anyhow::Result<HashMap<StateMachine, Arc<dyn IsmpProvider>>> {
	let HyperbridgeConfig { chains, .. } = config.clone();
	let mut clients = HashMap::new();

	for (state_machine, config) in chains {
		let client = config
			.into_client(hyperbridge.clone())
			.await
			.context(format!("Failed to create client for {state_machine:?}"))?;
		clients.insert(state_machine, client);
	}

	Ok(clients)
}
