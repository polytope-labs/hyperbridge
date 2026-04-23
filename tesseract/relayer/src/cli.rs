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
use tracing::Instrument;
use transaction_fees::TransactionPayment;

use crate::{
	config::HyperbridgeConfig,
	fees::AccumulateFees,
	provider::{ConsensusProofSource, OffchainProofSource},
};
use messaging::outbound;

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
	    out a batched (consensus + messages) submission to every chain that has a\n\
	    non-empty `signer` configured (signer presence is the toggle).\n\
	    Authority-set rotations (mandatory proofs) always propagate; messaging-only\n\
	    proofs skip chains with no pending messages.\n\
	\n\
	The Hyperbridge node must expose `offchain_localStorageGet` for outbound to read proofs\n\
	(typically requires `--rpc-methods Unsafe`). See `docs/` or module-level docs in\n\
	`tesseract/relayer/src/{config,outbound,provider}.rs` for full details."
)]
pub struct Cli {
	/// Optional subcommand. When absent, runs the relayer in the usual
	/// long-running mode.
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,
	/// Path to the relayer config file
	#[arg(short, long)]
	pub config: String,
	/// Path to the relayer database file (for fee tracking)
	#[arg(short, long)]
	pub db: String,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommand {
	/// Fetch and print the initial ConsensusState for a given state machine,
	/// hex-encoded.
	LogConsensusState {
		/// State machine whose consensus state should be logged, e.g.
		/// `EVM-97`, `POLKADOT-1000`.
		state_machine: String,
	},
	/// Run a one-shot fee-withdrawal pass over every configured destination,
	/// then exit. Uses the same code path as the periodic `auto_withdraw`
	/// loop; `relayer.minimum_withdrawal_amount` still gates.
	Withdraw,
	/// Claim fees for every past delivery recorded in the local DB, in both
	/// directions, and (optionally) withdraw the resulting hyperbridge
	/// balance to each destination.
	AccumulateFees(AccumulateFees),
}

const BANNER: &str = r"
 ████████╗███████╗███████╗███████╗███████╗██████╗  █████╗  ██████╗████████╗
 ╚══██╔══╝██╔════╝██╔════╝██╔════╝██╔════╝██╔══██╗██╔══██╗██╔════╝╚══██╔══╝
    ██║   █████╗  ███████╗███████╗█████╗  ██████╔╝███████║██║        ██║
    ██║   ██╔══╝  ╚════██║╚════██║██╔══╝  ██╔══██╗██╔══██║██║        ██║
    ██║   ███████╗███████║███████║███████╗██║  ██║██║  ██║╚██████╗   ██║
    ╚═╝   ╚══════╝╚══════╝╚══════╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝   ╚═╝
";

impl Cli {
	pub async fn run(self) -> Result<(), anyhow::Error> {
		eprintln!("{BANNER}");
		setup_logging()?;

		tracing::info!(
			target: crate::LOG_TARGET, version = env!("CARGO_PKG_VERSION"),
			config = %self.config,
			"starting tesseract-relayer",
		);

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
		// Shared between outbound fan-out and periodic fee-withdraw — both
		// need to fetch accepted BEEFY proofs from HB's offchain storage.
		let proof_source: Arc<dyn ConsensusProofSource> =
			Arc::new(OffchainProofSource::new(hb_rpc_client));
		tracing::info!(target: crate::LOG_TARGET, hb = %hyperbridge_provider.name(), %coprocessor, "connected to Hyperbridge");

		// Build the IsmpHost for every chain that opted into inbound consensus.
		// `create_client_map` takes paired (consensus variant, host kind)
		// entries — we assemble those from each chain's `PerChainConfig`.
		let consensus_hosts = create_client_map(config.consensus_chains()).await?;
		// Redundant with the final "relayer tasks initialized" summary below.
		tracing::trace!(target: crate::LOG_TARGET, count = consensus_hosts.len(), "consensus hosts built");

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
		tracing::trace!(target: crate::LOG_TARGET, count = providers.len(), "chain providers built");

		let mut provider_clients = providers.clone();
		provider_clients.insert(coprocessor, hyperbridge_provider.clone());

		let messaging_config: tesseract_primitives::config::RelayerConfig = relayer.clone().into();
		let fees_disabled = messaging_config.disable_fee_accumulation.unwrap_or_default();

		// Inbound consensus — only for chains whose config includes `[<chain>.consensus]`.
		for (state_machine, host) in &consensus_hosts {
			let hb = hyperbridge_provider.clone();
			let name = format!("inbound-consensus-{}-{}", host.provider().name(), hb.name());
			let host = host.clone();
			let span = tracing::info_span!(
				"inbound_consensus",
				chain = %host.provider().name(),
				hb = %hb.name(),
			);

			task_manager.spawn_essential_handle().spawn_blocking(
				Box::leak(Box::new(name)),
				"consensus",
				async move {
					tracing::trace!(target: crate::LOG_TARGET, "task started");
					let res = host.start_consensus(hb).await;
					tracing::error!(target: crate::LOG_TARGET, ?res, "task terminated");
				}
				.instrument(span)
				.boxed(),
			);
			tracing::trace!(target: crate::LOG_TARGET, %state_machine, "spawned inbound-consensus");
		}

		// Inbound messaging — every chain in `[chains.*]` gets an inbound
		// messaging task. There's no opt-in gate: if you configured the chain,
		// you want its inbound messages relayed. Fee accumulation is NOT wired
		// here — it's an outbound-relayer concern and is spawned below only for
		// chains that opted into outbound.
		let fisherman_enabled = relayer.fisherman.unwrap_or(false);
		for (state_machine, provider) in &providers {
			let mut hb_for_messaging =
				SubstrateClient::<KeccakSubstrateChain>::new(config.hyperbridge.clone()).await?;
			hb_for_messaging.set_latest_finalized_height(provider.clone()).await?;

			messaging::inbound(
				hb_for_messaging,
				provider.clone(),
				messaging_config.clone(),
				coprocessor,
				tx_payment.clone(),
				provider_clients.clone(),
				&task_manager,
				None,
			)
			.await?;

			// Fisherman watches chain_b ↔ HB for byzantine state-machine updates
			// and dispatches veto extrinsics. Opt-in via `relayer.fisherman =
			// true`. Vetoes need to write to the chain (e.g. an EVM veto call
			// reads the relayer's `address` slot), so the fisherman task is
			// only spawned for chains with a signer configured.
			let chain_has_signer = config
				.chains
				.get(state_machine)
				.map(|pc| pc.outbound_enabled())
				.unwrap_or(false);
			if fisherman_enabled && chain_has_signer {
				let hb_for_fisherman: Arc<dyn IsmpProvider> = Arc::new(
					SubstrateClient::<KeccakSubstrateChain>::new(config.hyperbridge.clone())
						.await?,
				);
				tesseract_fisherman::fish(
					hb_for_fisherman,
					provider.clone(),
					&task_manager,
					coprocessor,
				)
				.await?;
			} else if fisherman_enabled {
				tracing::info!(
					target: crate::LOG_TARGET, %state_machine,
					"fisherman skipped: chain has no signer configured (vetoes require one)",
				);
			}
		}

		// Outbound: one task, fans out over every chain that has a non-empty
		// signer configured. The signer's presence is the toggle (a chain
		// without a signer cannot submit transactions, so it stays inbound
		// only). Fee accumulation is part of the outbound pipeline: each
		// outbound-enabled chain gets a dedicated fee-accumulation task that
		// drains the receipts the outbound fan-out produces after a
		// successful destination submit.
		let destinations: BTreeMap<StateMachine, Arc<dyn IsmpProvider>> = config
			.chains
			.iter()
			.filter(|(_, pc)| pc.outbound_enabled())
			.filter_map(|(sm, _)| providers.get(sm).map(|p| (*sm, p.clone())))
			.collect();

		if destinations.is_empty() {
			tracing::info!(target: crate::LOG_TARGET, "no chains have a signer configured, skipping outbound task");
		} else {
			// One fee-accumulation channel per outbound destination. Populated
			// only when fees aren't globally disabled.
			let mut outbound_fee_senders: HashMap<StateMachine, Sender<Vec<TxReceipt>>> =
				HashMap::new();
			if !fees_disabled {
				for (sm, provider) in &destinations {
					let (fee_sender, fee_receiver) = mpsc::channel::<Vec<TxReceipt>>(64);
					outbound_fee_senders.insert(*sm, fee_sender);

					let name =
						format!("fee-acc-{}-{}", provider.name(), hyperbridge_provider.name());
					let hb_for_fees =
						SubstrateClient::<KeccakSubstrateChain>::new(config.hyperbridge.clone())
							.await?;
					let dest = provider.clone();
					let client_map = provider_clients.clone();
					let tx_payment = tx_payment.clone();
					let span = tracing::info_span!(
						"fee_accumulation",
						chain = %provider.name(),
						hb = %hyperbridge_provider.name(),
					);
					task_manager.spawn_essential_handle().spawn_blocking(
						Box::leak(Box::new(name)),
						"fees",
						async move {
							tracing::trace!(target: crate::LOG_TARGET, "task started");
							let res = messaging::fee_accumulation(
								fee_receiver,
								dest,
								hb_for_fees,
								client_map,
								tx_payment,
							)
							.await;
							tracing::error!(target: crate::LOG_TARGET, ?res, "task terminated");
						}
						.instrument(span)
						.boxed(),
					);
				}
			}

			let hb = hyperbridge_provider.clone();
			let name = format!("outbound-{}", hb.name());
			let outbound_relayer_cfg = messaging_config.clone();
			let outbound_client_map = provider_clients.clone();
			let outbound_proof_source = proof_source.clone();

			task_manager.spawn_essential_handle().spawn_blocking(
				Box::leak(Box::new(name)),
				"outbound",
				async move {
					tracing::trace!(target: crate::LOG_TARGET, "task started");
					let res = outbound::run(
						hb,
						destinations,
						outbound_proof_source,
						outbound_relayer_cfg,
						outbound_client_map,
						outbound_fee_senders,
					)
					.await;
					tracing::error!(target: crate::LOG_TARGET, ?res, "task terminated");
				}
				.boxed(),
			);
		}

		// Fee withdrawal: one global task, periodic per
		// `relayer.withdrawal_frequency`. Queries each destination's unclaimed
		// balance on HB, submits a withdrawal request once the minimum
		// threshold is crossed. The withdrawal POST that comes back is
		// delivered by the relayer on the destination chain, so it requires
		// a signer there. Skip chains without one.
		if !fees_disabled {
			let hb_for_withdraw =
				SubstrateClient::<KeccakSubstrateChain>::new(config.hyperbridge.clone()).await?;
			let withdraw_clients: HashMap<StateMachine, Arc<dyn IsmpProvider>> = config
				.chains
				.iter()
				.filter(|(_, pc)| pc.outbound_enabled())
				.filter_map(|(sm, _)| providers.get(sm).map(|p| (*sm, p.clone())))
				.collect();
			let withdraw_cfg = messaging_config.clone();
			let withdraw_db = tx_payment.clone();
			let withdraw_proof_source = proof_source.clone();
			let name = format!("fee-withdraw-{}", hyperbridge_provider.name());
			let span = tracing::info_span!("fee_withdrawal", hb = %hyperbridge_provider.name());
			task_manager.spawn_essential_handle().spawn_blocking(
				Box::leak(Box::new(name)),
				"fees",
				async move {
					tracing::trace!(target: crate::LOG_TARGET, "task started");
					let res = messaging::fees::auto_withdraw(
						hb_for_withdraw,
						withdraw_clients,
						withdraw_cfg,
						withdraw_db,
						withdraw_proof_source,
					)
					.await;
					tracing::error!(target: crate::LOG_TARGET, ?res, "task terminated");
				}
				.instrument(span)
				.boxed(),
			);
		}

		tracing::info!(
			target: crate::LOG_TARGET, chains = config.chains.len(),
			consensus_enabled = consensus_hosts.len(),
			fees = !fees_disabled,
			"relayer tasks initialized",
		);
		task_manager.future().await?;
		Ok(())
	}

	/// `log-consensus-state <STATE_MACHINE>` — one-shot: fetch and print the
	/// initial ConsensusState for the given chain hex-encoded.
	pub async fn log_consensus_state(
		&self,
		state_machine_str: String,
	) -> Result<(), anyhow::Error> {
		use std::str::FromStr;
		let state_machine = StateMachine::from_str(&state_machine_str)
			.map_err(|err| anyhow::anyhow!("invalid state machine '{state_machine_str}': {err}"))?;

		tracing::info!(target: crate::LOG_TARGET, %state_machine, "fetching consensus state");
		let config = HyperbridgeConfig::parse_conf(&self.config).await?;
		let consensus_hosts = create_client_map(config.consensus_chains()).await?;
		let host = consensus_hosts.get(&state_machine).ok_or_else(|| {
			anyhow::anyhow!(
				"no consensus host for {state_machine} — did you forget `[chains.{state_machine}.consensus]`?"
			)
		})?;

		let consensus_state = host.query_initial_consensus_state().await?.ok_or_else(|| {
			anyhow::anyhow!("{state_machine} has no queryable initial consensus state")
		})?;
		tracing::info!(
			target: crate::LOG_TARGET, %state_machine,
			"ConsensusState:\n0x{}",
			hex::encode(&consensus_state.consensus_state)
		);
		Ok(())
	}

	/// `withdraw` — one-shot: run a single pass of fee withdrawal across every
	/// configured destination, then exit. Uses the same logic as the periodic
	/// `auto_withdraw` loop (threshold gating, DB persistence, etc.).
	pub async fn withdraw_once(&self) -> Result<(), anyhow::Error> {
		tracing::info!(target: crate::LOG_TARGET, "one-shot withdrawal starting");
		let config = HyperbridgeConfig::parse_conf(&self.config).await?;
		let messaging_config: tesseract_primitives::config::RelayerConfig =
			config.relayer.clone().into();

		let tx_payment = Arc::new(
			TransactionPayment::initialize(&self.db)
				.await
				.context("Error initializing fee database")?,
		);
		let hyperbridge_substrate =
			SubstrateClient::<KeccakSubstrateChain>::new(config.hyperbridge.clone()).await?;
		let hyperbridge_provider: Arc<dyn IsmpProvider> = Arc::new(hyperbridge_substrate.clone());
		let proof_source: Arc<dyn ConsensusProofSource> =
			Arc::new(OffchainProofSource::new(hyperbridge_substrate.rpc_client.clone()));

		// Build one messaging client per configured chain so `withdraw_once`
		// can deliver withdrawal receipts back to each destination.
		let mut providers: HashMap<StateMachine, Arc<dyn IsmpProvider>> = HashMap::new();
		for (sm, pc) in &config.chains {
			let provider = pc
				.messaging
				.clone()
				.into_client(hyperbridge_provider.clone())
				.await
				.with_context(|| format!("failed to build messaging client for {sm}"))?;
			providers.insert(*sm, provider);
		}

		messaging::fees::withdraw_once(
			&hyperbridge_substrate,
			&providers,
			&messaging_config,
			&tx_payment,
			&proof_source,
		)
		.await;
		tracing::info!(target: crate::LOG_TARGET, "one-shot withdrawal complete");
		Ok(())
	}
}

fn setup_logging() -> Result<(), anyhow::Error> {
	use tracing_subscriber::{fmt, prelude::*, EnvFilter};

	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
	tracing_subscriber::registry().with(fmt::layer()).with(filter).init();

	Ok(())
}
