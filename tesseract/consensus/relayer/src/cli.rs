use anyhow::anyhow;
use clap::Parser;
use codec::Decode;
use ismp::host::StateMachine;
use std::{
	collections::{BTreeMap, HashMap},
	str::FromStr,
	sync::Arc,
};
use substrate_state_machine::HashAlgorithm;
use tesseract_beefy::backend::ProofBackend;
use tesseract_primitives::IsmpHost;
use tesseract_substrate::{
	config::{Blake2SubstrateChain, KeccakSubstrateChain},
	SubstrateConfig,
};
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
		log::info!(target: "consensus-relayer", "🧊 Initializing tesseract consensus");

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
			log::info!(target: "consensus-relayer", "Setting base consensus state from base state machine: {state_machine}");
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
							log::error!(target: "consensus-relayer", "{name} has terminated with result {res:?}")
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
						log::error!(target: "consensus-relayer", "{name} has terminated with result {res:?}")
					}
				},
			);
		}

		// If there is a configuration for the maximum interval between consensus updates, then
		// spawn monitoring task
		if relayer.clone().maximum_update_intervals.is_some_and(|val| !val.is_empty()) {
			log::info!(target: "consensus-relayer", "Initializing consensus update monitoring task");
			task_manager.spawn_essential_handle().spawn("monitoring", "consensus", {
				async move {
					let _res = monitor_clients(
						hyperbridge_config,
						clients,
						relayer.maximum_update_intervals.expect("Is Some"),
					)
					.await;
					log::error!(target: "consensus-relayer", "monitoring task has terminated")
				}
				.boxed()
			});
		}

		log::info!(target: "consensus-relayer", "Initialized consensus tasks");

		task_manager.future().await?;

		log::info!(target: "consensus-relayer", "Consensus Tasks aborted, restart relayer");

		Ok(())
	}
}

/// Host-side config paired with a consensus variant. EVM-family consensus
/// clients need an [`EvmConfig`]; grandpa needs a [`SubstrateConfig`].
#[derive(Debug, Clone)]
pub enum HostKind {
	Evm(tesseract_evm::EvmConfig),
	Substrate(SubstrateConfig),
}

impl HostKind {
	pub fn as_evm(&self) -> Option<&tesseract_evm::EvmConfig> {
		match self {
			HostKind::Evm(e) => Some(e),
			_ => None,
		}
	}
	pub fn as_substrate(&self) -> Option<&SubstrateConfig> {
		match self {
			HostKind::Substrate(s) => Some(s),
			_ => None,
		}
	}
}

/// Extract all Eth L2 configs from the consensus/host pairings provided.
/// Keeps the paired `EvmConfig` alongside each variant because the inner L2
/// host constructors need it.
fn extract_l2_configs(
	supported_l2s: Vec<String>,
	config_map: HashMap<StateMachine, (AnyConfig, HostKind)>,
) -> BTreeMap<StateMachine, L2Config> {
	let mut map = BTreeMap::new();
	for (state_machine, (config, host)) in config_map
		.into_iter()
		.filter(|(state_machine, ..)| supported_l2s.contains(&state_machine.to_string()))
	{
		let HostKind::Evm(evm) = host else { continue };
		match config {
			AnyConfig::ArbitrumOrbit(arb) => {
				map.insert(state_machine, L2Config::ArbitrumOrbit(arb, evm));
			},
			AnyConfig::OpStack(op) => {
				map.insert(state_machine, L2Config::OpStack(op, evm));
			},
			_ => {},
		}
	}

	map
}

/// Create a map of all clients supplied in config.
///
/// Each chain entry now comes as a `(AnyConfig, HostKind)` pair: the
/// consensus variant and the host-side config (EVM or Substrate) the
/// consensus struct no longer embeds itself.
pub async fn create_client_map(
	chains: HashMap<StateMachine, (AnyConfig, HostKind)>,
) -> anyhow::Result<HashMap<StateMachine, Arc<dyn IsmpHost>>> {
	let mut clients = HashMap::new();

	// Snapshot for l2 resolution (each call into_* consumes its entry).
	let l2_source = chains.clone();

	for (state_machine, (config, host)) in chains {
		let client = match (config, host) {
			(AnyConfig::Sepolia(config), HostKind::Evm(evm)) => {
				let l2_configs = extract_l2_configs(
					config.layer_twos.clone().unwrap_or_default(),
					l2_source.clone(),
				);
				config.into_sepolia(evm, l2_configs).await?
			},
			(AnyConfig::Ethereum(config), HostKind::Evm(evm)) => {
				let l2_configs = extract_l2_configs(
					config.layer_twos.clone().unwrap_or_default(),
					l2_source.clone(),
				);
				config.into_mainnet(evm, l2_configs).await?
			},
			(AnyConfig::ArbitrumOrbit(config), HostKind::Evm(evm)) =>
				config.into_client(evm).await?,
			(AnyConfig::OpStack(config), HostKind::Evm(evm)) => config.into_client(evm).await?,
			(AnyConfig::BscTestnet(config), HostKind::Evm(evm)) =>
				config.into_client::<tesseract_bsc::Testnet>(evm).await?,
			(AnyConfig::Bsc(config), HostKind::Evm(evm)) =>
				config.into_client::<tesseract_bsc::Mainnet>(evm).await?,
			(AnyConfig::Chiado(config), HostKind::Evm(evm)) => config.into_chiado(evm).await?,
			(AnyConfig::Gnosis(config), HostKind::Evm(evm)) => config.into_gnosis(evm).await?,
			(AnyConfig::Polygon(config), HostKind::Evm(evm)) => config.into_client(evm).await?,
			(AnyConfig::Tendermint(config), HostKind::Evm(evm)) => config.into_client(evm).await?,
			(AnyConfig::EvmHost(config), HostKind::Evm(evm)) => config.into_client(evm).await?,
			(AnyConfig::Pharos(config), HostKind::Evm(evm)) => match evm.state_machine {
				StateMachine::Evm(688689) =>
					config.into_client::<pharos_primitives::Testnet>(evm).await?,
				_ => config.into_client::<pharos_primitives::Mainnet>(evm).await?,
			},
			(AnyConfig::Grandpa(config), HostKind::Substrate(substrate)) => {
				match substrate.hashing {
					Some(HashAlgorithm::Keccak) =>
						config
							.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>(substrate)
							.await?,
					_ =>
						config
							.into_client::<Blake2SubstrateChain, Blake2SubstrateChain>(substrate)
							.await?,
				}
			},
			(variant, host) => {
				return Err(anyhow!(
					"incompatible (consensus, host) pairing for {state_machine}: {variant:?} with \
					 {host:?}"
				));
			},
		};
		clients.insert(state_machine, client);
	}

	Ok(clients)
}
