use anyhow::Result;
use codec::Encode;
use ismp::{
	consensus::{ConsensusStateId, StateCommitment},
	host::StateMachine,
	messaging::{CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_tendermint::{ConsensusState, TENDERMINT_CONSENSUS_CLIENT_ID};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tendermint_primitives::{self, Client, CodecTrustedState};
use tendermint_prover::CometBFTClient;
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod notification;

/// Host configuration for Tendermint relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
	/// Frequency (in seconds) to check for new updates
	pub consensus_update_frequency: Option<u64>,
	/// Tendermint RPC URL
	pub rpc_url: String,
	/// Chain ID
	pub chain_id: String,
}

/// Top-level config for Tendermint relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TendermintConfig {
	pub host: HostConfig,
}

impl TendermintConfig {
	pub async fn into_client(self) -> anyhow::Result<Arc<dyn IsmpHost>> {
		Ok(Arc::new(TendermintHost::new(&self.host).await?))
	}

	pub fn state_machine(&self) -> StateMachine {
		StateMachine::Tendermint(self.host.chain_id.as_bytes().try_into().unwrap())
	}
}

/// The relayer host for Tendermint
#[derive(Clone)]
pub struct TendermintHost {
	pub consensus_state_id: ConsensusStateId,
	pub state_machine: StateMachine,
	pub host: HostConfig,
	pub provider: Arc<dyn IsmpProvider>,
	pub prover: CometBFTClient,
}

impl TendermintHost {
	/// Create a new TendermintHost
	pub async fn new(host: &HostConfig) -> Result<Self, anyhow::Error> {
		let ismp_provider = ; // TODO: Get the ismp provider for the Tendermint chain
		Ok(Self {
			consensus_state_id: Default::default(),
			state_machine: StateMachine::Tendermint(host.chain_id),
			host: host.clone(),
			provider: Arc::new(ismp_provider),
			prover: CometBFTClient::new(&host.rpc_url).await?,
		})
	}

	/// Fetch the current consensus state (for initial state creation)
	pub async fn get_consensus_state(&self) -> Result<ConsensusState, anyhow::Error> {
		let latest_height = self.prover.latest_height().await?;

		let trusted_header = self.prover.signed_header(latest_height).await?;

		let trusted_validators =
			self.prover.validators(trusted_header.header.height.into()).await?;
		let trusted_next_validators =
			self.prover.next_validators(trusted_header.header.height.into()).await?;

		let trusted_state = tendermint_primitives::TrustedState::new(
			self.host.chain_id.clone(),
			trusted_header.header.height.into(),
			trusted_header.header.time.unix_timestamp() as u64,
			trusted_header.header.hash().as_bytes().try_into().unwrap(),
			trusted_validators,
			trusted_next_validators,
			trusted_header.header.next_validators_hash.as_bytes().try_into().unwrap(),
			82 * 3600, // TODO: This is derived from the checkpoint interval, which changes as per each tendermint chain
			tendermint_primitives::VerificationOptions::default(),
		);

		let codec_trusted_state = CodecTrustedState::from(&trusted_state);

		let consensus_state =
			ConsensusState { tendermint_state: codec_trusted_state, chain_id: self.host.chain_id.as_bytes().try_into().unwrap() };

		Ok(consensus_state)
	}

	/// Get the ISMP provider
	pub fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}

#[async_trait::async_trait]
impl IsmpHost for TendermintHost {
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		use crate::notification::consensus_notification;
		let interval = tokio::time::interval(Duration::from_secs(
			self.host.consensus_update_frequency.unwrap_or(300),
		));
		let client = self.clone();
		let counterparty_clone = counterparty.clone();
		let mut interval = Box::pin(interval);
		let provider = self.provider();
		loop {
			interval.as_mut().tick().await;
			match consensus_notification(&client, counterparty_clone.clone()).await {
				Ok(Some(update)) => {
					use ismp::messaging::ConsensusMessage;
					let consensus_message = ConsensusMessage {
						consensus_proof: update.encode(),
						consensus_state_id: client.consensus_state_id,
						signer: counterparty.address(),
					};
					log::info!(
						target: "tesseract",
						"🛰️ Transmitting consensus message from {} to {}",
						provider.name(), counterparty.name()
					);
					let res = counterparty
						.submit(
							vec![Message::Consensus(consensus_message)],
							counterparty.state_machine_id().state_id,
						)
						.await;
					if let Err(err) = res {
						log::error!(
							"Failed to submit transaction to {}: {err:?}",
							counterparty.name()
						)
					}
				},
				Ok(None) => {
					// No update to send, just continue
				},
				Err(e) => {
					log::error!(target: "tesseract","Consensus task {}->{} encountered an error: {e:?}", provider.name(), counterparty.name())
				},
			}
		}
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		let initial_consensus_state: ConsensusState =
			self.get_consensus_state().await.map_err(|e| {
				anyhow::anyhow!("TendermintHost: fetch initial consensus state failed: {e}")
			})?;

		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: TENDERMINT_CONSENSUS_CLIENT_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 82 * 3600, // TODO: This is derived from the checkpoint interval, which changes as per each tendermint chain
			challenge_periods: vec![(self.state_machine, 2 * 60)].into_iter().collect(),
			state_machine_commitments: vec![(
				ismp::consensus::StateMachineId {
					state_id: self.state_machine,
					consensus_state_id: self.consensus_state_id,
				},
				StateCommitmentHeight {
					commitment: StateCommitment {
						timestamp: initial_consensus_state.tendermint_state.timestamp,
						overlay_root: None,
						state_root: primitive_types::H256(
							initial_consensus_state.tendermint_state.finalized_header_hash,
						),
					},
					height: initial_consensus_state.tendermint_state.height,
				},
			)],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}
