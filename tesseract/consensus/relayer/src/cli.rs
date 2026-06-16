use anyhow::anyhow;
use clap::Parser;
use codec::Decode;
use ismp::host::StateMachine;
use std::str::FromStr;
use tesseract_beefy::backend::ProofBackend;
use tesseract_consensus_config::create_client_map;
use tesseract_primitives::IsmpHost;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};

use crate::{
	any::AnyHost, config::HyperbridgeConfig, logging, monitor::monitor_clients,
	subcommand::Subcommand,
};
use futures::FutureExt;
use polkadot_sdk::sc_service::TaskManager;

/// The tesseract multi-chain consensus relayer.
///
/// Tesseract consensus queries consensus proofs for multiple chains to be submitted to the
/// Hyperbridge blockchain.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,
	/// Path to the relayer config file
	#[arg(short, long)]
	pub config: String,

	/// Optional base state machine to use for consensus initialization
	#[arg(long)]
	base: Option<String>,
}

impl Cli {
	/// Start the consensus tasks
	pub async fn start_consensus(&self) -> Result<(), anyhow::Error> {
		logging::setup()?;
		log::info!(target: crate::LOG_TARGET, "🧊 Initializing tesseract consensus");

		let config = HyperbridgeConfig::parse_conf(&self.config).await?;
		let HyperbridgeConfig { hyperbridge, relayer, chains, .. } = config.clone();
		// This binary drives consensus + the BEEFY prover/host — it requires
		// the full `[hyperbridge]` section. The consolidated `tesseract-relayer`
		// doesn't, which is why this field is `Option`.
		let hyperbridge_config = hyperbridge.ok_or_else(|| {
			anyhow!("[hyperbridge] section required for the consensus/prover binary")
		})?;

		let tokio_handle = tokio::runtime::Handle::current();
		let mut task_manager = TaskManager::new(tokio_handle, None)?;

		let hyperbridge = hyperbridge_config
			.clone()
			.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
			.await?;

		// initialize the beefy proof queues for all evm state machines
		if let AnyHost::Beefy(ref beefy_host) = hyperbridge {
			let state_machines: Vec<_> =
				chains.keys().cloned().filter(|s| matches!(s, StateMachine::Evm(_))).collect();

			beefy_host.backend.init_queues(&state_machines).await?;
		}

		let clients = create_client_map(chains.clone()).await?;
		let relayer = relayer.unwrap_or_default();

		if let Some(ref state_machine_str) = self.base {
			let state_machine = StateMachine::from_str(state_machine_str.as_str())
				.map_err(|err| anyhow!("{err}"))?;
			log::info!(target: crate::LOG_TARGET, "Setting base consensus state from base state machine: {state_machine}");
			let client = clients.get(&state_machine).ok_or_else(|| anyhow!("Client not found"))?;
			let consensus_state_bytes =
				client.provider().query_consensus_state(None, *b"PARA").await?;
			let consensus_state =
				tesseract_beefy::ConsensusState::decode(&mut &consensus_state_bytes[..])?;

			if let AnyHost::Beefy(beefy) = hyperbridge {
				beefy.hydrate_initial_consensus_state(consensus_state).await?;
			}
		}

		for (_, client) in clients.clone() {
			let hyperbridge = hyperbridge_config
				.clone()
				.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
				.await?;
			let hyper_bridge_name = hyperbridge.provider().name();
			let name =
				format!("consensus-{}-{}", hyper_bridge_name.clone(), client.provider().name());

			if relayer.enable_hyperbridge_consensus {
				task_manager.spawn_essential_handle().spawn_blocking(
					Box::leak(Box::new(name.clone())),
					"consensus",
					{
						let client = client.clone();
						async move {
							let res = hyperbridge.start_consensus(client.provider()).await;
							log::error!(target: crate::LOG_TARGET, "{name} has terminated with result {res:?}")
						}
						.boxed()
					},
				);
			}

			let name = format!("consensus-{}-{}", client.provider().name(), hyper_bridge_name);
			task_manager.spawn_essential_handle().spawn_blocking(
				Box::leak(Box::new(name.clone())),
				"consensus",
				{
					let hyperbridge = hyperbridge_config
						.clone()
						.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
						.await?;
					async move {
						let res = client.start_consensus(hyperbridge.provider()).await;
						log::error!(target: crate::LOG_TARGET, "{name} has terminated with result {res:?}")
					}
				},
			);
		}

		// If there is a configuration for the maximum interval between consensus updates, then
		// spawn monitoring task
		if relayer.clone().maximum_update_intervals.is_some_and(|val| !val.is_empty()) {
			log::info!(target: crate::LOG_TARGET, "Initializing consensus update monitoring task");
			task_manager.spawn_essential_handle().spawn("monitoring", "consensus", {
				async move {
					let _res = monitor_clients(
						hyperbridge_config,
						clients,
						relayer.maximum_update_intervals.expect("Is Some"),
					)
					.await;
					log::error!(target: crate::LOG_TARGET, "monitoring task has terminated")
				}
				.boxed()
			});
		}

		log::info!(target: crate::LOG_TARGET, "Initialized consensus tasks");

		task_manager.future().await?;

		log::info!(target: crate::LOG_TARGET, "Consensus Tasks aborted, restart relayer");

		Ok(())
	}
}
