use anyhow::{anyhow, Result};
use codec::Encode;
use ismp::{
	consensus::{ConsensusStateId, StateCommitment},
	host::StateMachine,
	messaging::{CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_polygon::{ConsensusState, PolygonConsensusUpdate, POLYGON_CONSENSUS_CLIENT_ID};
use serde::{Deserialize, Serialize};
use std::{future::pending, sync::Arc, time::Duration};
use tendermint_primitives::{self, Client, CodecTrustedState};
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod client;
mod notification;

pub use client::HeimdallClient;

/// Host configuration for Polygon POS relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
	/// Frequency (in seconds) to check for new updates
	pub consensus_update_frequency: Option<u64>,
	/// Heimdall RPC URL
	pub heimdall_rpc_url: String,
	/// Disable consensus task
	pub disable: Option<bool>,
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
		let execution_rpc_url =
			evm.rpc_urls.get(0).ok_or_else(|| anyhow!("No rpc urls privided"))?;
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
			prover: HeimdallClient::new(&host.heimdall_rpc_url, &execution_rpc_url)?,
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

		let chain_id = match self.state_machine {
			StateMachine::Evm(chain_id) => chain_id,
			_ => return Err(anyhow::anyhow!("Unsupported state machine")),
		};

		let trusted_state = tendermint_primitives::TrustedState::new(
			format!("heimdallv2-{}", chain_id),
			trusted_header.header.height.into(),
			trusted_header.header.time.unix_timestamp() as u64,
			trusted_header.header.hash().as_bytes().try_into().unwrap(),
			trusted_validators,
			trusted_next_validators,
			trusted_header.header.next_validators_hash.as_bytes().try_into().unwrap(),
			82 * 3600,
			tendermint_primitives::VerificationOptions::default(),
		);

		let codec_trusted_state = CodecTrustedState::from(&trusted_state);
		let (_, milestone) = self
			.prover
			.get_latest_milestone_at_height(latest_height)
			.await?
			.ok_or_else(|| anyhow!("Failed to fetch latest milestone"))?;

		let consensus_state = ConsensusState {
			tendermint_state: codec_trusted_state,
			last_finalized_block: milestone.end_block,
			last_finalized_hash: milestone.hash,
			chain_id,
		};

		Ok(consensus_state)
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
		let disable = self.host.disable.unwrap_or_default();

		if disable {
			let _ = pending::<()>().await;
			Ok(())
		} else {
			use crate::notification::consensus_notification;
			let interval = tokio::time::interval(Duration::from_secs(
				self.host.consensus_update_frequency.unwrap_or(30),
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
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		let initial_consensus_state = self.get_consensus_state().await.map_err(|e| {
			anyhow::anyhow!("PolygonPosHost: fetch initial consensus state failed: {e}")
		})?;

		let evm_header = self
			.prover
			.fetch_header(initial_consensus_state.last_finalized_block)
			.await?
			.ok_or_else(|| anyhow::anyhow!("EVM header not found for milestone end block"))?;

		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: POLYGON_CONSENSUS_CLIENT_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 82 * 3600, // 82 checkpoints x 3600 seconds per checkpoint
			challenge_periods: vec![(self.state_machine, 2 * 60)].into_iter().collect(),
			state_machine_commitments: vec![(
				ismp::consensus::StateMachineId {
					state_id: self.state_machine,
					consensus_state_id: self.consensus_state_id,
				},
				StateCommitmentHeight {
					commitment: StateCommitment {
						timestamp: evm_header.timestamp,
						overlay_root: None,
						state_root: evm_header.state_root,
					},
					height: evm_header.number.low_u64(),
				},
			)],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}
