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

use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
};

use anyhow::Context;
use clap::Parser;
use futures::FutureExt;
use ismp::host::StateMachine;
use polkadot_sdk::sc_service::TaskManager;
use tesseract_consensus::cli::create_client_map;
use tesseract_primitives::{IsmpProvider, TxReceipt};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use tokio::sync::mpsc::{self, Sender};
use transaction_fees::TransactionPayment;

use crate::{
	config::HyperbridgeConfig,
	outbound,
	provider::{ConsensusProofSource, OffchainProofSource},
};

#[derive(Parser, Debug)]
#[command(
	version,
	about = "Consolidated Hyperbridge relayer — inbound messaging, inbound consensus, and outbound fan-out in one process",
	long_about = "Reads one TOML config and spawns:\n\
	\n\
	  • Inbound messaging (chain → Hyperbridge) for every chain in [chains.*].\n\
	\n\
	  • Inbound consensus (chain → Hyperbridge) only for chains that declare a\n\
	    [chains.X.consensus] sub-table. Host essentials (rpc_urls, signer, ...) are\n\
	    inherited from the parent so they don't need to be re-specified.\n\
	\n\
	  • Outbound (Hyperbridge → chain). A single task subscribed to pallet\n\
	    `beefy-consensus-proofs::ProofAccepted` events on Hyperbridge; on each event\n\
	    it fetches the accepted proof from the HB node's offchain storage and fans\n\
	    out a batched (consensus + messages) submission to every chain whose\n\
	    per-chain `outbound` flag is true (the default). Authority-set rotations\n\
	    (mandatory proofs) always propagate; messaging-only proofs skip chains with\n\
	    no pending messages.\n\
	\n\
	The Hyperbridge node must expose `offchain_localStorageGet` for outbound to read proofs\n\
	(typically requires `--rpc-methods Unsafe`). See `docs/` or module-level docs in\n\
	`tesseract/relayer/src/{config,outbound,provider}.rs` for full details."
)]
pub struct Cli {
	/// Path to the relayer config file
	#[arg(short, long)]
	pub config: String,

	/// Path to the relayer database file (for fee tracking)
	#[arg(short, long)]
	pub db: String,
}

impl Cli {
	pub async fn run(self) -> Result<(), anyhow::Error> {
		setup_logging()?;
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

		// Hyperbridge is a plain substrate client here — no BEEFY prover/host.
		// The prover lives in a separate binary; this relayer only consumes its
		// output (accepted consensus proofs in the pallet's offchain storage).
		let hyperbridge_substrate =
			SubstrateClient::<KeccakSubstrateChain>::new(config.hyperbridge.clone()).await?;
		let hyperbridge_provider: Arc<dyn IsmpProvider> = Arc::new(hyperbridge_substrate.clone());
		let coprocessor = hyperbridge_provider.state_machine_id().state_id;
		let hb_rpc_client = hyperbridge_substrate.rpc_client.clone();

		// Build the IsmpHost for every chain that opted into inbound consensus.
		// `create_client_map` reads the filtered consensus sub-config produced
		// by `HyperbridgeConfig::consensus_config`.
		let consensus_hosts = create_client_map(config.consensus_config()).await?;

		// One `Arc<dyn IsmpProvider>` per chain. Chains with consensus reuse the
		// consensus host's provider; chains without consensus build a dedicated
		// messaging-only provider from their per-chain messaging config.
		let mut providers: HashMap<StateMachine, Arc<dyn IsmpProvider>> = HashMap::new();
		for (sm, host) in &consensus_hosts {
			providers.insert(*sm, host.provider());
		}
		for (sm, pc) in &config.chains {
			if providers.contains_key(sm) {
				continue;
			}
			let provider = pc
				.messaging
				.clone()
				.into_client(hyperbridge_provider.clone())
				.await
				.with_context(|| format!("failed to build messaging client for {sm}"))?;
			providers.insert(*sm, provider);
		}

		let mut provider_clients = providers.clone();
		provider_clients.insert(coprocessor, hyperbridge_provider.clone());

		let messaging_config: tesseract_primitives::config::RelayerConfig = relayer.clone().into();
		let fees_disabled = messaging_config.disable_fee_accumulation.unwrap_or_default();

		// Inbound consensus — only for chains whose config includes `[<chain>.consensus]`.
		for (state_machine, host) in &consensus_hosts {
			let hb = hyperbridge_provider.clone();
			let name = format!("inbound-consensus-{}-{}", host.provider().name(), hb.name());
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
			log::debug!(target: "tesseract", "spawned inbound-consensus for {state_machine}");
		}

		// One fee-accumulation channel per chain. Both inbound messaging (chain→HB)
		// and outbound (HB→chain) submissions push `TxReceipt`s into the same
		// per-chain sender. A single fee-accumulation task per chain drains the
		// receiver and claims the accumulated fees on Hyperbridge.
		let mut outbound_fee_senders: HashMap<StateMachine, Sender<Vec<TxReceipt>>> =
			HashMap::new();

		// Inbound messaging — every chain in `[chains.*]` gets an inbound
		// messaging task. There's no opt-in gate: if you configured the chain,
		// you want its inbound messages relayed.
		for (state_machine, provider) in &providers {
			let mut hb_for_messaging =
				SubstrateClient::<KeccakSubstrateChain>::new(config.hyperbridge.clone()).await?;
			hb_for_messaging.set_latest_finalized_height(provider.clone()).await?;

			// Fee receipts channel for this destination (shared with outbound).
			let (fee_sender, fee_receiver) = mpsc::channel::<Vec<TxReceipt>>(64);
			if !fees_disabled {
				outbound_fee_senders.insert(*state_machine, fee_sender.clone());

				let name = format!("fee-acc-{}-{}", provider.name(), hyperbridge_provider.name());
				let hb_for_fees = hb_for_messaging.clone();
				let dest = provider.clone();
				let client_map = provider_clients.clone();
				let tx_payment = tx_payment.clone();
				task_manager.spawn_essential_handle().spawn_blocking(
					Box::leak(Box::new(name.clone())),
					"fees",
					async move {
						let res = tesseract_messaging::fee_accumulation(
							fee_receiver,
							dest,
							hb_for_fees,
							client_map,
							tx_payment,
						)
						.await;
						log::error!(target: "tesseract", "{name} terminated: {res:?}");
					}
					.boxed(),
				);
			}

			tesseract_messaging::relay(
				hb_for_messaging,
				provider.clone(),
				messaging_config.clone(),
				coprocessor,
				tx_payment.clone(),
				provider_clients.clone(),
				&task_manager,
				(!fees_disabled).then_some(fee_sender),
			)
			.await?;
		}

		// Outbound — one task, fans out over every chain that opted in via
		// per-chain `outbound = true` (the default).
		let destinations: BTreeMap<StateMachine, Arc<dyn IsmpProvider>> = config
			.chains
			.iter()
			.filter(|(_, pc)| pc.outbound)
			.filter_map(|(sm, _)| providers.get(sm).map(|p| (*sm, p.clone())))
			.collect();

		if destinations.is_empty() {
			log::info!(
				target: "tesseract",
				"no chains opted into outbound — skipping outbound task"
			);
		} else {
			let proof_source: Arc<dyn ConsensusProofSource> =
				Arc::new(OffchainProofSource::new(hb_rpc_client));
			let hb = hyperbridge_provider.clone();
			let name = format!("outbound-{}", hb.name());
			let outbound_relayer_cfg = messaging_config.clone();
			let outbound_client_map = provider_clients.clone();
			let outbound_fee_senders_snapshot = outbound_fee_senders.clone();

			task_manager.spawn_essential_handle().spawn_blocking(
				Box::leak(Box::new(name.clone())),
				"outbound",
				async move {
					let res = outbound::run(
						hb,
						destinations,
						proof_source,
						outbound_relayer_cfg,
						outbound_client_map,
						outbound_fee_senders_snapshot,
					)
					.await;
					log::error!(target: "tesseract", "{name} terminated: {res:?}");
				}
				.boxed(),
			);
		}

		// Fee withdrawal — one global task, periodic per `relayer.withdrawal_frequency`.
		// Queries each destination's unclaimed balance on HB, submits a withdrawal
		// request once the minimum threshold is crossed.
		if !fees_disabled {
			let hb_for_withdraw =
				SubstrateClient::<KeccakSubstrateChain>::new(config.hyperbridge.clone()).await?;
			let withdraw_clients = providers.clone();
			let withdraw_cfg = messaging_config.clone();
			let withdraw_db = tx_payment.clone();
			let name = format!("fee-withdraw-{}", hyperbridge_provider.name());
			task_manager.spawn_essential_handle().spawn_blocking(
				Box::leak(Box::new(name.clone())),
				"fees",
				async move {
					let res = tesseract_messaging::fees::auto_withdraw(
						hb_for_withdraw,
						withdraw_clients,
						withdraw_cfg,
						withdraw_db,
					)
					.await;
					log::error!(target: "tesseract", "{name} terminated: {res:?}");
				}
				.boxed(),
			);
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
