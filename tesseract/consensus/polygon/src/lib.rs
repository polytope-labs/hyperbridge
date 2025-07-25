use anyhow::Result;
use codec::{Decode, Encode};
use ismp::{
	consensus::ConsensusStateId,
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use ismp_polygon::{ConsensusState, PolygonConsensusUpdate, POLYGON_CONSENSUS_CLIENT_ID};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tendermint_prover::HeimdallClient;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod notification;

/// Host configuration for Polygon POS relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
	/// Frequency (in seconds) to check for new updates
	pub consensus_update_frequency: Option<u64>,
	/// Heimdall RPC URL
	pub heimdall_rpc_url: String,
	/// Heimdall REST URL
	pub heimdall_rest_url: String,
	/// Execution RPC URL
	pub execution_rpc_url: String,
}

/// Top-level config for Polygon POS relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonPosConfig {
	pub host: HostConfig,
	#[serde(flatten)]
	pub evm_config: EvmConfig,
}

impl PolygonPosConfig {
	pub async fn into_client(self) -> anyhow::Result<Arc<dyn IsmpHost>> {
		Ok(Arc::new(PolygonPosHost::new(&self.host, &self.evm_config).await?))
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

/// The relayer host for Polygon POS
#[derive(Clone)]
pub struct PolygonPosHost {
	pub consensus_state_id: ConsensusStateId,
	pub state_machine: StateMachine,
	pub host: HostConfig,
	pub evm: EvmConfig,
	pub provider: Arc<dyn IsmpProvider>,
	pub prover: HeimdallClient,
}

impl PolygonPosHost {
	/// Create a new PolygonPosHost
	pub async fn new(host: &HostConfig, evm: &EvmConfig) -> Result<Self, anyhow::Error> {
		let ismp_provider = EvmClient::new(evm.clone()).await?;
		Ok(Self {
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(evm.consensus_state_id.as_bytes());
				consensus_state_id
			},
			state_machine: evm.state_machine,
			host: host.clone(),
			evm: evm.clone(),
			provider: Arc::new(ismp_provider),
			prover: HeimdallClient::new(
				&host.heimdall_rpc_url,
				&host.heimdall_rest_url,
				&host.execution_rpc_url,
			)?,
		})
	}

	/// Fetch the current consensus state (for initial state creation)
	pub async fn get_consensus_state(&self) -> Result<ConsensusState, anyhow::Error> {
		let consensus_state_serialized: Vec<u8> =
			self.provider.query_consensus_state(None, self.consensus_state_id).await?;
		Ok(ConsensusState::decode(&mut &consensus_state_serialized[..])?)
	}

	/// Get the ISMP provider
	pub fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}

#[async_trait::async_trait]
impl IsmpHost for PolygonPosHost {
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
						signer: vec![], // Q for reviewer: Who is the signer?
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
		let initial_consensus_state = self.get_consensus_state().await.map_err(|e| {
			anyhow::anyhow!("PolygonPosHost: fetch initial consensus state failed: {e}")
		})?;
		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: POLYGON_CONSENSUS_CLIENT_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_periods: vec![(self.state_machine, 5 * 60)].into_iter().collect(),
			state_machine_commitments: vec![],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}
