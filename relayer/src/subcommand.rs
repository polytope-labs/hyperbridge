use std::str::FromStr;

use crate::{cli::create_client_map, config::HyperbridgeConfig, logging};
use anyhow::anyhow;
use ismp::host::StateMachine;
use tesseract_substrate::config::{Blake2SubstrateChain, KeccakSubstrateChain};

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Set consensus state for a client on Hyperbridge
	SetConsensusState(SetConsensusState),
	/// Output the scale encoded `CreateConsensusState` Message
	LogConsensusState(SetConsensusState),
}

#[derive(Debug, clap::Parser)]
#[command(
	propagate_version = true,
	args_conflicts_with_subcommands = true,
	subcommand_negates_reqs = true
)]
pub struct SetConsensusState {
	state_machine: String,
}

impl SetConsensusState {
	pub async fn set_consensus_state(&self, config_path: String) -> Result<(), anyhow::Error> {
		logging::setup()?;
		let config = HyperbridgeConfig::parse_conf(&config_path).await?;
		let HyperbridgeConfig { hyperbridge: hyperbridge_config, .. } = config.clone();

		let hyperbridge = hyperbridge_config
			.clone()
			.into_client::<Blake2SubstrateChain, KeccakSubstrateChain>()
			.await?;

		let clients = create_client_map(config).await?;

		let state_machine = StateMachine::from_str(&self.state_machine)
			.map_err(|_| anyhow!("Failed to deserialize state machine"))?;

		let client = clients
			.get(&state_machine)
			.ok_or_else(|| anyhow!("Client for provided state machine was not found"))?;

		let consensus_state = client
			.query_initial_consensus_state()
			.await?
			.ok_or_else(|| anyhow!("The state machine provided does not have a consensus state"))?;

		hyperbridge.client().create_consensus_state(consensus_state).await?;

		Ok(())
	}

	pub async fn log_consensus_state(&self, config_path: String) -> Result<(), anyhow::Error> {
		logging::setup()?;
		let config = HyperbridgeConfig::parse_conf(&config_path).await?;

		let clients = create_client_map(config).await?;

		let state_machine = StateMachine::from_str(&self.state_machine)
			.map_err(|_| anyhow!("Failed to deserialize state machine"))?;

		let client = clients
			.get(&state_machine)
			.ok_or_else(|| anyhow!("Client for provided state machine was not found"))?;

		let _consensus_state = client
			.query_initial_consensus_state()
			.await?
			.ok_or_else(|| anyhow!("The state machine provided does not have a consensus state"))?;

		// todo: Make CreateConsensus serializable
		// let json_string = json::to_string(&consensus_state)?;

		// let mut stdout = tokio::io::stdout();
		// stdout.write_all(json_string).await?;

		Ok(())
	}
}
