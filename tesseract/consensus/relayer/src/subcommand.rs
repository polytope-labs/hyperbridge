use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use anyhow::anyhow;
use codec::Compact;
use subxt::tx::Payload;

use ismp::host::StateMachine;
use subxt_utils::values::{
	compact_u32_to_value, evm_hosts_btreemap_to_value, host_params_btreemap_to_value,
};
use tesseract_primitives::IsmpHost;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};

use crate::{cli::create_client_map, config::HyperbridgeConfig, logging};

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Output the serialized `ConsensusState` Message for a client
	LogConsensusState(LogMetatdata),
	/// Output the scale-encoded HostExecutive::update_evm_hosts extrinsic for an evm state machine
	LogHostParams(LogMetatdata),
}

#[derive(Debug, clap::Parser)]
#[command(
	propagate_version = true,
	args_conflicts_with_subcommands = true,
	subcommand_negates_reqs = true
)]
pub struct LogMetatdata {
	/// State Machine whose consensus state should be generated
	state_machine: String,
	/// Wrap the call in the sudo extrinsic
	sudo: Option<bool>,
}

impl LogMetatdata {
	pub async fn log_host_param(&self, config_path: String) -> Result<(), anyhow::Error> {
		logging::setup()?;

		let state_machine = StateMachine::from_str(&self.state_machine)
			.map_err(|_| anyhow!("Failed to deserialize state machine"))?;

		let mut config = HyperbridgeConfig::parse_conf(&config_path).await?;

		// remove all other chains
		config.chains.retain(|s, _| state_machine == *s);

		let hyperbridge = config
			.hyperbridge
			.clone()
			.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
			.await?;

		let clients = create_client_map(config.clone()).await?;
		let client = clients
			.get(&state_machine)
			.ok_or_else(|| anyhow!("Client for provided state machine was not found"))?;

		log::info!("Fetching host params for {state_machine}");
		let host_param = client.provider().query_host_params(state_machine).await?;
		let host_params: BTreeMap<_, _> = vec![(state_machine, host_param)].into_iter().collect();

		let host_address = config
			.chains
			.get(&state_machine)
			.ok_or_else(|| anyhow!("Config for {state_machine:?} not found"))?
			.host_address()
			.ok_or_else(|| anyhow!("Missing host address for {state_machine:?}"))?;
		let evm_hosts: BTreeMap<_, _> = vec![(state_machine, host_address)].into_iter().collect();

		// Call to set the HostParams
		let set_host_params = subxt::dynamic::tx(
			"HostExecutive",
			"set_host_params",
			vec![host_params_btreemap_to_value(&host_params)],
		);
		// Call to set the Host address
		let update_evm_hosts = subxt::dynamic::tx(
			"HostExecutive",
			"update_evm_hosts",
			vec![evm_hosts_btreemap_to_value(&evm_hosts)],
		);
		// batch them both
		let batch = subxt::dynamic::tx(
			"Utility",
			"batch_all",
			vec![
				compact_u32_to_value(Compact(2u32)),
				set_host_params.into_value(),
				update_evm_hosts.into_value(),
			],
		)
		.encode_call_data(&hyperbridge.client().client.metadata())?;

		let proposal = if self.sudo.unwrap_or_default() {
			subxt::dynamic::tx("Sudo", "sudo", batch)
				.encode_call_data(&hyperbridge.client().client.metadata())?
		} else {
			batch
		};

		log::info!("HostExecutive call for {state_machine:?}:\n0x{}", hex::encode(&proposal));

		Ok(())
	}

	pub async fn log_consensus_state(&self, config_path: String) -> Result<(), anyhow::Error> {
		logging::setup()?;

		let state_machine = StateMachine::from_str(&self.state_machine)
			.map_err(|_| anyhow!("Failed to deserialize state machine"))?;

		log::info!("🧊 Fetching consensus state for {state_machine}");
		let config = HyperbridgeConfig::parse_conf(&config_path).await?;

		let hyperbridge = config
			.hyperbridge
			.clone()
			.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
			.await?;

		let mut clients = create_client_map(config.clone()).await?;

		clients.insert(hyperbridge.provider().state_machine_id().state_id, Arc::new(hyperbridge));

		let client = clients
			.get(&state_machine)
			.ok_or_else(|| anyhow!("Client for provided state machine was not found"))?;

		let consensus_state = client
			.query_initial_consensus_state()
			.await?
			.ok_or_else(|| anyhow!("The state machine provided does not have a consensus state"))?;

		log::info!(
			"ConsensusState for {state_machine}:\n0x{}",
			hex::encode(&consensus_state.consensus_state)
		);

		Ok(())
	}
}
