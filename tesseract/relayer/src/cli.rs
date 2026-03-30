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

use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use clap::Parser;
use futures::FutureExt;
use ismp::host::StateMachine;
use polkadot_sdk::sc_service::TaskManager;
use tesseract_consensus::cli::create_client_map;
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};
use transaction_fees::TransactionPayment;

use crate::{
	config::HyperbridgeConfig,
	outbound,
	provider::{ConsensusProofProvider, IndexerProofProvider},
};

#[derive(Parser, Debug)]
#[command(version, about = "Consolidated Hyperbridge relayer")]
pub struct Cli {
	/// Path to the relayer config file
	#[arg(short, long)]
	pub config: String,

	/// Path to the relayer database file (for fee tracking)
	#[arg(short, long)]
	pub db: String,

	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommand {
	/// Capture a BEEFY consensus proof from the relay chain and store it in the indexer DB
	CaptureProof {
		/// Relay chain websocket RPC URL
		#[arg(long)]
		relay_rpc: String,
		/// PostgreSQL connection URL for the indexer database
		#[arg(long)]
		db_url: String,
	},
}

impl Cli {
	pub async fn run(self) -> Result<(), anyhow::Error> {
		setup_logging()?;

		if let Some(subcommand) = self.subcommand {
			return match subcommand {
				Subcommand::CaptureProof { relay_rpc, db_url } =>
					crate::capture_proof::capture_and_store(&relay_rpc, &db_url).await,
			};
		}

		log::info!("Initializing tesseract relayer");

		let config = HyperbridgeConfig::parse_conf(&self.config).await?;
		let relayer = config.relayer.clone();

		let tokio_handle = tokio::runtime::Handle::current();
		let mut task_manager = TaskManager::new(tokio_handle, None)?;

		let tx_payment = Arc::new(
			TransactionPayment::initialize(&self.db)
				.await
				.context("Error initializing fee database")?,
		);

		// Hyperbridge as IsmpHost (for inbound consensus counterparty)
		let hyperbridge_host = config
			.hyperbridge
			.clone()
			.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
			.await?;
		let coprocessor = hyperbridge_host.provider().state_machine_id().state_id;
		let hyperbridge_provider = hyperbridge_host.provider();

		let host_clients = create_client_map(config.consensus_config()).await?;

		// IsmpProvider map derived from IsmpHost clients
		let mut provider_clients: HashMap<StateMachine, Arc<dyn IsmpProvider>> =
			host_clients.iter().map(|(sm, host)| (*sm, host.provider())).collect();
		provider_clients.insert(coprocessor, hyperbridge_provider.clone());

		let proof_provider: Option<Arc<dyn ConsensusProofProvider>> = match relayer.indexer_db_url {
			Some(ref db_url) => {
				let indexer = proof_indexer::ProofIndexer::initialize(db_url).await?;
				log::info!("Outbound proof provider connected to indexer DB");
				Some(Arc::new(IndexerProofProvider::new(indexer)))
			},
			None => None,
		};

		let messaging_config: tesseract_primitives::config::RelayerConfig = relayer.clone().into();

		for (state_machine, host) in &host_clients {
			if !relayer.delivery_endpoints.is_empty() &&
				!relayer.delivery_endpoints.contains(&state_machine.to_string())
			{
				continue;
			}

			let provider = host.provider();

			// -- Inbound consensus: EVM → Hyperbridge --
			{
				let hb = hyperbridge_provider.clone();
				let name = format!("inbound-consensus-{}-{}", provider.name(), hb.name());
				let host = host.clone();

				task_manager.spawn_essential_handle().spawn_blocking(
					Box::leak(Box::new(name.clone())),
					"consensus",
					async move {
						let res = host.start_consensus(hb).await;
						log::error!(target: "tesseract", "{name} terminated: {res:?}");
					}
					.boxed(),
				);
			}

			// -- Inbound messaging: EVM → Hyperbridge --
			{
				let mut hb_for_messaging = tesseract_substrate::SubstrateClient::<
					KeccakSubstrateChain,
				>::new(config.hyperbridge.substrate_config())
				.await?;
				hb_for_messaging.set_latest_finalized_height(provider.clone()).await?;

				tesseract_messaging::relay(
					hb_for_messaging,
					provider.clone(),
					messaging_config.clone(),
					coprocessor,
					tx_payment.clone(),
					provider_clients.clone(),
					&task_manager,
				)
				.await?;
			}

			// -- Outbound: Hyperbridge → EVM (merged consensus + messaging) --
			if let Some(ref proof_provider) = proof_provider {
				let hb = hyperbridge_provider.clone();
				let evm = provider.clone();
				let provider = proof_provider.clone();
				let coproc = coprocessor;
				let name = format!("outbound-{}-{}", hb.name(), evm.name());

				task_manager.spawn_essential_handle().spawn_blocking(
					Box::leak(Box::new(name.clone())),
					"outbound",
					async move {
						let res = outbound::run(hb, evm, provider, coproc).await;
						log::error!(target: "tesseract", "{name} terminated: {res:?}");
					}
					.boxed(),
				);
			}
		}

		log::info!("Initialized relayer tasks");
		task_manager.future().await?;

		Ok(())
	}
}

fn setup_logging() -> Result<(), anyhow::Error> {
	use tracing_subscriber::{fmt, prelude::*, EnvFilter};

	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
	tracing_subscriber::registry().with(fmt::layer()).with(filter).init();

	Ok(())
}
