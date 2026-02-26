//! Polyhedron: BEEFY Fiat-Shamir Proof Relayer for Tron
//!
//! This binary combines both the BEEFY prover and consensus relayer functionality,
//! specifically for relaying BEEFY Fiat-Shamir proofs to Tron only.
//!
//! Architecture:
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │          Polyhedron Main Loop               │
//! ├─────────────────────────────────────────────┤
//! │                                             │
//! │  ┌──────────────┐      ┌─────────────────┐  │
//! │  │ BeefyProver  │─────▶│ InMemoryBackend │  │
//! │  │              │      │  (queue/state)  │  │
//! │  └──────────────┘      └─────────────────┘  │
//! │         │                      │            │
//! │         │                      │            │
//! │         │              ┌───────▼─────────┐  │
//! │         │              │   BeefyHost     │  │
//! │         │              │(start_consensus)│  │
//! │         │              └─────────────────┘  │
//! │         │                      │            │
//! │         │                      ▼            │
//! │         └──────────────▶ ┌───────────────┐  │
//! │                          │ TronClient    │  │
//! │                          │ (IsmpProvider)│  │
//! │                          └───────────────┘  │
//! │                                 │           │
//! │                                 ▼           │
//! │                          Tron Network       │
//! └─────────────────────────────────────────────┘
//! ```

use anyhow::Context;
use clap::Parser;
use codec::Decode;
use futures::FutureExt;
use ismp::consensus::StateMachineId;
use polkadot_sdk::sc_service::TaskManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tesseract_beefy::{
	backend::{InMemoryProofBackend, ProofBackend},
	host::{BeefyHost, BeefyHostConfig},
	prover::{BeefyProver, BeefyProverConfig, Prover, ProverConfig, ProverConsensusState},
};
use tesseract_consensus::logging;
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tesseract_substrate::{
	config::{Blake2SubstrateChain, KeccakSubstrateChain},
	SubstrateClient, SubstrateConfig,
};
use tesseract_tron::TronConfig;

/// CLI arguments for Polyhedron
#[derive(Parser, Debug)]
#[command(author, version, about = "Polyhedron: BEEFY Fiat-Shamir Proof Relayer for Tron")]
pub struct Cli {
	/// Path to the polyhedron config file
	#[arg(short, long)]
	pub config: String,
}

/// Complete configuration for Polyhedron
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolyhedronConfig {
	/// Configuration for the BEEFY prover (relay/para chain connections)
	pub prover: ProverConfig,

	/// Configuration for the substrate client (Hyperbridge)
	pub substrate: SubstrateConfig,

	/// Configuration for the BEEFY prover behavior
	pub beefy: BeefyProverConfig,

	/// Configuration for the BEEFY host
	pub host: BeefyHostConfig,

	/// Configuration for the Tron client
	pub tron: TronConfig,
}

impl PolyhedronConfig {
	/// Load configuration from a TOML file
	pub async fn load(path: &str) -> anyhow::Result<Self> {
		let contents = tokio::fs::read_to_string(path)
			.await
			.context(format!("Failed to read config file: {}", path))?;

		let config: PolyhedronConfig =
			toml::from_str(&contents).context("Failed to parse TOML config")?;

		// Validate configuration
		config.validate()?;

		Ok(config)
	}

	/// Validate the configuration
	fn validate(&self) -> anyhow::Result<()> {
		// Ensure proof variant is Fiat-Shamir for Tron
		if !matches!(self.prover.proof_variant, tesseract_beefy::prover::ProofVariant::FiatShamir) {
			anyhow::bail!(
				"Polyhedron requires proof_variant = \"fiat_shamir\" for Tron compatibility"
			);
		}

		// Ensure redis is not configured (we use in-memory backend)
		if self.beefy.redis.is_some() {
			log::warn!("Redis configuration detected in beefy config but will be ignored - Polyhedron uses in-memory backend");
		}

		if self.host.redis.is_some() {
			log::warn!("Redis configuration detected in host config but will be ignored - Polyhedron uses in-memory backend");
		}

		// Ensure state_machines contains exactly the Tron state machine
		let tron_state_machine = self.tron.state_machine();
		if !self.beefy.state_machines.contains(&tron_state_machine) {
			anyhow::bail!(
				"beefy.state_machines must contain the Tron state machine: {:?}",
				tron_state_machine
			);
		}

		if self.beefy.state_machines.len() > 1 {
			log::warn!(
				"Multiple state machines configured, but Polyhedron will only relay to Tron: {:?}",
				tron_state_machine
			);
		}

		Ok(())
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	// Setup logging
	logging::setup()?;

	// Install rustls crypto provider
	// rustls::crypto::ring::default_provider()
	// 	.install_default()
	// 	.expect("Failed to install rustls crypto provider");

	log::info!("Initializing Polyhedron: BEEFY Fiat-Shamir Proof Relayer for Tron");

	// Parse CLI and load config
	let cli = Cli::parse();
	let config = PolyhedronConfig::load(&cli.config)
		.await
		.context("Failed to load configuration")?;

	log::info!("Configuration loaded and validated from: {}", cli.config);

	// Initialize TaskManager for lifecycle management
	let tokio_handle = tokio::runtime::Handle::current();
	let mut task_manager = TaskManager::new(tokio_handle, None)?;

	log::info!("Initializing Tron client...");

	// Initialize Tron client first (needed to query initial consensus state)
	let tron_client: Arc<dyn IsmpProvider> = Arc::new(
		config
			.tron
			.clone()
			.into_client()
			.await
			.context("Failed to create Tron client")?,
	);
	let tron_name = tron_client.name();
	let tron_state_machine = config.tron.state_machine();

	log::info!("Connected to Tron: {}", tron_name);

	log::info!("Initializing BEEFY prover components...");

	// Create prover instance
	let prover_instance = Prover::new(config.prover.clone())
		.await
		.context("Failed to create prover instance")?;

	// Query initial consensus state from Tron client
	log::info!("Querying initial consensus state from Tron...");
	let consensus_state_bytes = tron_client
		.query_consensus_state(None, config.beefy.consensus_state_id)
		.await
		.context("Failed to query consensus state from Tron")?;

	let consensus_state = tesseract_beefy::ConsensusState::decode(&mut &consensus_state_bytes[..])
		.context("Failed to decode consensus state")?;

	let finalized_parachain_height: u64 = tron_client
		.query_latest_height(StateMachineId {
			state_id: config.substrate.state_machine.clone(),
			consensus_state_id: Default::default(),
		})
		.await
		.context("Failed to query latest parachain height from Substrate")?
		.into();

	let prover_consensus_state =
		ProverConsensusState { inner: consensus_state.clone(), finalized_parachain_height };

	log::info!(
		"Initial consensus state from Tron: authority_set_id={}, latest_beefy_height={}",
		consensus_state.current_authorities.id,
		consensus_state.latest_beefy_height
	);

	// Create shared in-memory backend with initial state
	let backend = Arc::new(InMemoryProofBackend::new(prover_consensus_state));

	// Initialize Tron state machine for backend queues
	backend
		.init_queues(&[tron_state_machine])
		.await
		.context("Failed to initialize queues")?;

	log::info!("Initialized in-memory backend for state machine: {:?}", tron_state_machine);

	// Initialize substrate client (Hyperbridge)
	let substrate_client = SubstrateClient::new(config.substrate.clone())
		.await
		.context("Failed to create substrate client")?;

	log::info!("Connected to Hyperbridge: {}", substrate_client.name());

	// Initialize BeefyProver with in-memory backend
	let prover = Prover::new(config.prover.clone()).await.context("Failed to create prover")?;

	let mut beefy_prover =
		BeefyProver::<
			Blake2SubstrateChain,
			KeccakSubstrateChain,
			zk_beefy::LocalProver,
			InMemoryProofBackend,
		>::new(config.beefy.clone(), substrate_client.clone(), prover, backend.clone())
		.await
		.context("Failed to create BEEFY prover")?;

	log::info!("BEEFY prover initialized");

	// Initialize BeefyHost with same backend
	let beefy_host =
		BeefyHost::<
			Blake2SubstrateChain,
			KeccakSubstrateChain,
			zk_beefy::LocalProver,
			InMemoryProofBackend,
		>::new(config.host.clone(), prover_instance, substrate_client.clone(), backend)
		.await
		.context("Failed to create BEEFY host")?;

	log::info!("BEEFY host initialized");

	// Spawn prover task (generates proofs continuously)
	log::info!("Spawning BEEFY prover task");
	task_manager.spawn_essential_handle().spawn_blocking(
		"polyhedron-prover",
		"consensus",
		async move {
			beefy_prover.run().await;
			log::error!(target: "polyhedron", "Prover task has terminated unexpectedly");
		}
		.boxed(),
	);

	// Spawn host task (consumes and submits proofs to Tron)
	log::info!("Spawning BEEFY host task for Tron submission");
	task_manager.spawn_essential_handle().spawn_blocking(
		Box::leak(Box::new(format!("polyhedron-host-{}", tron_name))),
		"consensus",
		async move {
			let res = beefy_host.start_consensus(tron_client).await;
			log::error!(target: "polyhedron", "Host task has terminated with result: {res:?}");
		}
		.boxed(),
	);

	log::info!("Polyhedron initialized - relaying BEEFY proofs to Tron");
	log::info!("Monitoring tasks (press Ctrl+C to stop)...");

	// Wait for all tasks (will exit if any essential task fails)
	task_manager.future().await?;

	log::info!("Polyhedron tasks terminated - restart required");

	Ok(())
}
