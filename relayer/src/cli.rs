use anyhow::anyhow;
use clap::{arg, Parser};
use codec::Decode;
use ismp::host::StateMachine;
use std::{
	collections::{BTreeMap, HashMap},
	str::FromStr,
	sync::Arc,
};
use tesseract_beefy::host::BeefyHost;
use tesseract_primitives::IsmpHost;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};
use tesseract_sync_committee::L2Config;

use crate::{
	any::AnyConfig,
	config::{HyperbridgeConfig, RelayerConfig},
	logging,
	subcommand::Subcommand,
};

/// CLI interface for tesseract relayer.
#[derive(Parser, Debug)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,
	/// Path to the relayer config file
	#[arg(short, long)]
	pub config: String,

	/// Should we initialize the relevant consensus states on Eth chains?
	#[arg(long)]
	setup_eth: bool,

	/// Should we initialize the relevant Consensus state on hyperbridge?
	#[arg(long)]
	setup_para: bool,

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
		let HyperbridgeConfig { hyperbridge: hyperbridge_config, relayer, .. } = config.clone();

		let clients = create_client_map(config.clone()).await?;
		let mut processes = vec![];
		let hyperbridge = hyperbridge_config
			.clone()
			.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
			.await?;
		let relayer = relayer.ok_or_else(|| anyhow!("Relayer config was not supplied"))?;
		// set up initial consensus states
		if self.setup_eth || self.setup_para {
			initialize_consensus_clients(
				&hyperbridge,
				&clients,
				&relayer,
				self.setup_eth,
				self.setup_para,
			)
			.await?;
			log::info!("Initialized consensus states");
		}

		if let Some(ref state_machine_str) = self.base {
			let state_machine = StateMachine::from_str(state_machine_str.as_str())
				.map_err(|err| anyhow!("{err}"))?;
			log::info!("Setting base consensus state from base state machine: {state_machine:?}");
			let client = clients.get(&state_machine).ok_or_else(|| anyhow!("Client not found"))?;
			let consensus_state_bytes =
				client.provider().query_consensus_state(None, *b"PARA").await?;
			let consensus_state =
				tesseract_beefy::ConsensusState::decode(&mut &consensus_state_bytes[..])?;
			hyperbridge.hydrate_initial_consensus_state(Some(consensus_state)).await?;
		}

		for (_, client) in clients {
			let hyperbridge = hyperbridge_config
				.clone()
				.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
				.await?;
			processes.push(tokio::spawn({
				let client = client.clone();
				async move { hyperbridge.start_consensus(client.provider()).await }
			}));
			processes.push(tokio::spawn({
				let hyperbridge = hyperbridge_config
					.clone()
					.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
					.await?;
				async move { client.start_consensus(hyperbridge.provider()).await }
			}));
		}

		log::info!("Initialized consensus tasks");

		let (_result, _index, tasks) = futures::future::select_all(processes).await;

		log::info!("Task {_index} aborted with result {_result:#?}");

		for task in tasks {
			task.abort();
		}

		Ok(())
	}

	/// Start the fisherman tasks
	pub async fn start_fisherman(&self) -> Result<(), anyhow::Error> {
		logging::setup()?;
		log::info!("🧊 Initializing tesseract fisherman");

		let config = HyperbridgeConfig::parse_conf(&self.config).await?;

		let HyperbridgeConfig { hyperbridge: hyperbridge_config, .. } = config.clone();

		let _client_map = create_client_map(config.clone()).await?;
		let mut processes = vec![];

		for (_, client) in _client_map.iter() {
			let hyperbridge = hyperbridge_config
				.clone()
				.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
				.await?;
			processes.push(tokio::spawn(tesseract_fisherman::fish(
				Arc::new(hyperbridge),
				client.clone(),
			)));
		}

		log::info!("Initialized fisherman tasks");

		let (_result, _index, tasks) = futures::future::select_all(processes).await;

		for task in tasks {
			task.abort();
		}

		Ok(())
	}
}

/// Initializes the consensus state across all connected chains.
async fn initialize_consensus_clients(
	hyperbridge: &BeefyHost<Blake2SubstrateChain, KeccakSubstrateChain>,
	chains: &HashMap<StateMachine, Arc<dyn IsmpHost>>,
	relayer: &RelayerConfig,
	setup_eth: bool,
	setup_para: bool,
) -> anyhow::Result<()> {
	if setup_eth {
		let initial_state = hyperbridge.hydrate_initial_consensus_state(None).await?;

		// write this consensus state to redis
		for (state_machine, chain) in chains {
			let provider = chain.provider();
			log::info!("setting consensus state on {state_machine:?}");
			if let Err(err) = provider.set_initial_consensus_state(initial_state.clone()).await {
				log::error!("Failed to set initial consensus state on {state_machine:?}: {err:?}")
			}
		}
	}

	if setup_para {
		let mut params = BTreeMap::new();
		for (state_machine, client) in chains {
			let provider = client.provider();
			log::info!("setting consensus state for {state_machine:?} on hyperbridge");
			let host_param = provider.query_host_params(*state_machine).await?;
			params.insert(*state_machine, host_param);
			if let Some(mut consensus_state) = client.query_initial_consensus_state().await? {
				consensus_state.challenge_period = relayer.challenge_period.unwrap_or_default();
				hyperbridge.client().create_consensus_state(consensus_state).await?;
			}
		}

		log::info!("setting host params on on hyperbridge");
		hyperbridge.client().set_host_params(params).await?;
	}

	Ok(())
}

/// Extract all Eth L2 configs from the configurations provided
fn extract_l2_configs(
	config_map: HashMap<StateMachine, AnyConfig>,
) -> BTreeMap<StateMachine, L2Config> {
	let mut map = BTreeMap::new();
	for (state_machine, config) in config_map {
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
				let l2_configs = extract_l2_configs(chains.clone());
				let client = config.into_sepolia(l2_configs).await?;
				client
			},
			AnyConfig::Ethereum(config) => {
				let l2_configs = extract_l2_configs(chains.clone());
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
			AnyConfig::Bsc(config) => {
				let client = config.into_client().await?;
				client
			},
		};
		clients.insert(state_machine, client);
	}

	Ok(clients)
}
