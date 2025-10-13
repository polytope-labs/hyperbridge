use anyhow::anyhow;
use clap::{arg, Parser};
use codec::Decode;
use ismp::host::StateMachine;
use std::{
	collections::{BTreeMap, HashMap},
	str::FromStr,
	sync::Arc,
};
use substrate_state_machine::HashAlgorithm;
use tesseract_primitives::IsmpHost;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};
use tesseract_sync_committee::L2Config;

use crate::{
	any::{AnyConfig, AnyHost},
	config::HyperbridgeConfig,
	logging,
	monitor::monitor_clients,
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
	/// Start the consensus and optionally fisherman tasks
	pub async fn start_consensus(&self) -> Result<(), anyhow::Error> {
		logging::setup()?;
		log::info!("🧊 Initializing tesseract consensus");

		let config = HyperbridgeConfig::parse_conf(&self.config).await?;
		let HyperbridgeConfig { hyperbridge: hyperbridge_config, relayer, chains, .. } =
			config.clone();

		let tokio_handle = tokio::runtime::Handle::current();
		let mut task_manager = TaskManager::new(tokio_handle, None)?;

		let hyperbridge = hyperbridge_config
			.clone()
			.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
			.await?;

		// initialize the beefy proof queues for all evm state machines
		if let AnyHost::Beefy(ref beefy_host) = hyperbridge {
			let state_machines =
				chains.keys().cloned().filter(|s| matches!(s, StateMachine::Evm(_))).collect();

			beefy_host.init_queues(state_machines).await?;
		}

		let clients = create_client_map(config.clone()).await?;
		let relayer = relayer.unwrap_or_default();

		if let Some(ref state_machine_str) = self.base {
			let state_machine = StateMachine::from_str(state_machine_str.as_str())
				.map_err(|err| anyhow!("{err}"))?;
			log::info!("Setting base consensus state from base state machine: {state_machine}");
			let client = clients.get(&state_machine).ok_or_else(|| anyhow!("Client not found"))?;
			let consensus_state_bytes =
				client.provider().query_consensus_state(None, *b"PARA").await?;
			let consensus_state =
				tesseract_beefy::ConsensusState::decode(&mut &consensus_state_bytes[..])?;

			if let AnyHost::Beefy(beefy) = hyperbridge {
				beefy.hydrate_initial_consensus_state(Some(consensus_state)).await?;
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
							log::error!(target: "tesseract", "{name} has terminated with result {res:?}")
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
						log::error!(target: "tesseract", "{name} has terminated with result {res:?}")
					}
				},
			);
		}

		// If there is a configuration for the maximum interval between consensus updates, then
		// spawn monitoring task
		if relayer.clone().maximum_update_intervals.is_some_and(|val| !val.is_empty()) {
			log::info!("Initializing consensus update monitoring task");
			task_manager.spawn_essential_handle().spawn("monitoring", "consensus", {
				async move {
					let _res = monitor_clients(
						hyperbridge_config,
						clients,
						relayer.maximum_update_intervals.expect("Is Some"),
					)
					.await;
					log::error!(target: "tesseract", "monitoring task has terminated")
				}
				.boxed()
			});
		}

		log::info!("Initialized consensus tasks");

		task_manager.future().await?;

		log::info!("Consensus Tasks aborted, restart relayer");

		Ok(())
	}
}

/// Extract all Eth L2 configs from the configurations provided
fn extract_l2_configs(
	supported_l2s: Vec<String>,
	config_map: HashMap<StateMachine, AnyConfig>,
) -> BTreeMap<StateMachine, L2Config> {
	let mut map = BTreeMap::new();
	for (state_machine, config) in config_map
		.into_iter()
		.filter(|(state_machine, ..)| supported_l2s.contains(&state_machine.to_string()))
	{
		match config {
			AnyConfig::ArbitrumOrbit(arb_orbit_config) => {
				map.insert(state_machine, L2Config::ArbitrumOrbit(arb_orbit_config));
			},
			AnyConfig::OpStack(op_config) => {
				map.insert(state_machine, L2Config::OpStack(op_config));
			},
			_ => {},
		}
	}

	map
}

/// Create a map of all clients supplied in config
pub async fn create_client_map(
	config: HyperbridgeConfig,
) -> anyhow::Result<HashMap<StateMachine, Arc<dyn IsmpHost>>> {
	let HyperbridgeConfig { chains, .. } = config.clone();
	let mut clients = HashMap::new();

	for (state_machine, config) in chains.clone() {
		let client = match config {
			AnyConfig::Sepolia(config) => {
				let l2_configs = extract_l2_configs(
					config.layer_twos.clone().unwrap_or_default(),
					chains.clone(),
				);
				let client = config.into_sepolia(l2_configs).await?;
				client
			},
			AnyConfig::Ethereum(config) => {
				let l2_configs = extract_l2_configs(
					config.layer_twos.clone().unwrap_or_default(),
					chains.clone(),
				);
				let client = config.into_mainnet(l2_configs).await?;
				client
			},
			AnyConfig::ArbitrumOrbit(config) => {
				let client = config.into_client().await?;
				client
			},
			AnyConfig::OpStack(config) => {
				let client = config.into_client().await?;
				client
			},
			AnyConfig::BscTestnet(config) => {
				let client = config.into_client::<tesseract_bsc::Testnet>().await?;
				client
			},
			AnyConfig::Bsc(config) => {
				let client = config.into_client::<tesseract_bsc::Mainnet>().await?;
				client
			},

			AnyConfig::Chiado(config) => {
				let client = config.into_chiado().await?;
				client
			},

			AnyConfig::Gnosis(config) => {
				let client = config.into_gnosis().await?;
				client
			},
			AnyConfig::Polygon(config) => {
				let client = config.into_client().await?;
				client
			},

			AnyConfig::Tendermint(config) => {
				let client = config.into_client().await?;
				client
			},

			AnyConfig::Grandpa(config) => match config.substrate.hashing {
				Some(HashAlgorithm::Keccak) => {
					let client =
						config.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>().await?;
					client
				},
				_ => {
					let client =
						config.into_client::<Blake2SubstrateChain, Blake2SubstrateChain>().await?;
					client
				},
			},
		};
		clients.insert(state_machine, client);
	}

	Ok(clients)
}
