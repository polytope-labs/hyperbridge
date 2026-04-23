// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

//! `tesseract-admin-relayer` — admin-driven BEEFY mandatory-consensus relayer
//! for EVM chains.
//!
//! ```text
//!    ┌───────────────┐   mandatory proofs   ┌────────────────────────────┐
//!    │ beefy-prover  │ ───────────────────▶ │ ProofBackend (Redis/etc.)  │
//!    │ (separate)    │                       └─────────────┬──────────────┘
//!    └───────────────┘                                     │  drain(EpochChanged)
//!                                                          ▼
//!                                       ┌──────────────────────────────────┐
//!                                       │  per-chain mandatory task        │
//!                                       │  (see `task::run_mandatory_task`)│
//!                                       └──────────────┬───────────────────┘
//!                                                      │  ERC-7821 batch
//!                                                      ▼
//!                                       unfreeze → handleConsensus → freeze
//!                                             submitted from the
//!                                             EIP-7702 delegated EOA
//! ```

use std::sync::Arc;

use anyhow::{anyhow, Context};
use clap::Parser;
use futures::FutureExt;
use ismp::host::StateMachine;
use polkadot_sdk::sc_service::TaskManager;
use tesseract_admin_relayer::{
	config::{AdminRelayerConfig, SubmissionMode},
	delegation::ensure_delegated,
	logging,
	task::run_mandatory_task,
};
use tesseract_beefy::backend::{ProofBackend, RedisProofBackend};
use tesseract_evm::EvmClient;

/// CLI for the admin-consensus relayer.
#[derive(Parser, Debug)]
#[command(version, about = "Tesseract admin-relayer (mandatory consensus, EVM only)")]
struct Cli {
	/// Path to the TOML config file.
	#[arg(short, long)]
	config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	rustls::crypto::ring::default_provider()
		.install_default()
		.map_err(|_| anyhow!("failed to install rustls crypto provider"))?;

	logging::setup()?;

	let cli = Cli::parse();
	log::info!("🧊 tesseract-admin-relayer starting (config={})", cli.config);

	let config = AdminRelayerConfig::load(&cli.config).await?;
	log::info!("loaded {} EVM chain(s) from config", config.chains.len());

	// Connect to the shared Redis proof-queue. Force `realtime = true` so
	// we receive keyspace notifications from the prover process.
	let mut redis_cfg = config.hyperbridge.redis.clone();
	redis_cfg.realtime = true;
	let consensus_state_id = config.hyperbridge.consensus_state_id;
	let backend: Arc<dyn ProofBackend> = Arc::new(
		RedisProofBackend::new(redis_cfg)
			.await
			.context("failed to connect to Redis proof backend")?,
	);
	log::info!("connected to Redis proof backend");

	// Build an EVM client per configured chain, tracking each chain's submission mode.
	let mut clients: Vec<(StateMachine, EvmClient, SubmissionMode)> =
		Vec::with_capacity(config.chains.len());
	for (label, chain_cfg) in &config.chains {
		let state_machine = chain_cfg.evm.state_machine;
		if !matches!(state_machine, StateMachine::Evm(_)) {
			return Err(anyhow!("[{label}] expected EVM state machine, got {state_machine}"));
		}
		let client = EvmClient::new(chain_cfg.evm.clone())
			.await
			.with_context(|| format!("[{label}] failed to initialise EVM client"))?;
		log::info!(
			"[{label}] initialised EVM client chain_id={} state_machine={} mode={:?}",
			client.chain_id,
			state_machine,
			chain_cfg.submission_mode,
		);
		clients.push((state_machine, client, chain_cfg.submission_mode));
	}

	// Register each chain's queue in the backend so the prover knows where to
	// write proofs for it.
	let state_machines: Vec<StateMachine> = clients.iter().map(|(s, ..)| *s).collect();
	backend
		.init_queues(&state_machines)
		.await
		.context("failed to initialise proof backend queues")?;
	log::info!("initialised queues for {} state machine(s)", state_machines.len());

	// One-shot: delegate the signer on every chain whose submission mode is Batched.
	// Sequential-mode chains use plain EOA txs and do not need EIP-7702 delegation.
	for (state_machine, client, mode) in &clients {
		match mode {
			SubmissionMode::Batched => ensure_delegated(client).await?,
			SubmissionMode::Sequential =>
				log::info!("[{state_machine}] sequential mode — skipping EIP-7702 delegation"),
		}
	}

	// Spawn one task per chain.
	let tokio_handle = tokio::runtime::Handle::current();
	let mut task_manager = TaskManager::new(tokio_handle, None)?;

	for (state_machine, client, mode) in clients {
		let task_name = Box::leak(Box::new(format!("admin-{state_machine}")));
		let backend = backend.clone();
		task_manager.spawn_essential_handle().spawn_blocking(
			task_name,
			"consensus",
			async move {
				let res = run_mandatory_task(backend, consensus_state_id, client, mode).await;
				log::error!(target: "tesseract-admin-relayer",
					"[{state_machine}] task terminated with {res:?}");
			}
			.boxed(),
		);
	}

	log::info!("all mandatory consensus tasks spawned; waiting…");
	task_manager.future().await?;
	log::info!("task manager exited; relayer stopping");
	Ok(())
}
