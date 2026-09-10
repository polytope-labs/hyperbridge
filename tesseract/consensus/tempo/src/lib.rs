//! Tempo consensus relayer for tesseract.
//!
//! Streams finalization certificates from a Tempo node's `consensus` RPC
//! namespace and submits them as ISMP consensus updates. Identity rotations
//! (full DKG ceremonies) are handled by submitting the epoch-boundary blocks
//! that commit the new group key before advancing past them.

/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "consensus-tempo";

use anyhow::{anyhow, Context};
use codec::Encode;
use ismp::{
	consensus::StateCommitment,
	host::StateMachine,
	messaging::{ConsensusMessage, CreateConsensusState, Message, StateCommitmentHeight},
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tempo_verifier::primitives::{
	compute_epoch, decode_dkg_outcome, is_epoch_boundary, ConsensusState, DkgOutcome, G2_LEN,
	TEMPO_MAINNET_CHAIN_ID, TEMPO_TESTNET_CHAIN_ID,
};
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod notification;

pub use tempo_prover::{FinalizationInfo, TempoProver};

/// Default epoch length (in blocks) for Tempo mainnet and testnet.
pub const DEFAULT_EPOCH_LENGTH: u64 = 21_600;

/// Host configuration for the Tempo relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempoHostConfig {
	/// Frequency (in seconds) to check for new finalizations
	pub consensus_update_frequency: Option<u64>,
	/// Tempo node JSON-RPC URL. Must serve both the `eth` and `consensus`
	/// namespaces.
	pub rpc_url: String,
	/// Number of blocks per epoch. Defaults to [`DEFAULT_EPOCH_LENGTH`].
	pub epoch_length: Option<u64>,
	/// Unbonding period in seconds for `CreateConsensusState`.
	pub unbonding_period_secs: Option<u64>,
	/// Challenge period in seconds for `CreateConsensusState`.
	pub challenge_period_secs: Option<u64>,
	/// Hex-encoded network identity (96-byte compressed G2 key) override for
	/// bootstrapping in the genesis epoch, before any boundary block exists.
	pub network_identity: Option<String>,
}

/// Top-level config for the Tempo relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempoConfig {
	#[serde(flatten)]
	pub host: TempoHostConfig,
}

impl TempoConfig {
	pub async fn into_client(self, evm_config: EvmConfig) -> anyhow::Result<Arc<dyn IsmpHost>> {
		Ok(Arc::new(TempoHost::new(&self.host, &evm_config).await?))
	}
}

/// The relayer host for Tempo
#[derive(Clone)]
pub struct TempoHost {
	pub consensus_state_id: ismp::consensus::ConsensusStateId,
	pub state_machine: StateMachine,
	pub host: TempoHostConfig,
	pub provider: Arc<dyn IsmpProvider>,
	pub prover: TempoProver,
}

impl TempoHost {
	pub async fn new(host: &TempoHostConfig, evm: &EvmConfig) -> anyhow::Result<Self> {
		let ismp_provider = EvmClient::new(evm.clone()).await?;
		Ok(Self {
			consensus_state_id: ismp_provider.consensus_state_id,
			state_machine: ismp_provider.state_machine,
			host: host.clone(),
			provider: Arc::new(ismp_provider),
			prover: TempoProver::new(&host.rpc_url),
		})
	}

	pub fn epoch_length(&self) -> u64 {
		self.host.epoch_length.unwrap_or(DEFAULT_EPOCH_LENGTH)
	}

	/// Constructs the initial consensus state from the latest finalized block.
	///
	/// The active network identity is read from the previous epoch's boundary
	/// block; in the genesis epoch it must be supplied via
	/// [`TempoHostConfig::network_identity`].
	pub async fn get_consensus_state(&self) -> anyhow::Result<(ConsensusState, u64)> {
		let chain_id = match self.state_machine {
			StateMachine::Evm(chain_id) => chain_id,
			state_machine => Err(anyhow!("Unsupported state machine: {state_machine}"))?,
		};
		if chain_id != TEMPO_MAINNET_CHAIN_ID && chain_id != TEMPO_TESTNET_CHAIN_ID {
			log::warn!(
				target: LOG_TARGET,
				"Chain id {chain_id} is not a known Tempo network"
			);
		}

		let epoch_length = self.epoch_length();
		let latest = self.prover.finalization(None).await?;
		let epoch = compute_epoch(latest.height, epoch_length);

		// Which boundary block governs the state we are about to seed? If the
		// tip is itself a boundary, it commits the outcome for the *next* epoch
		// and the seeded height sits on it — so its own outcome is what the
		// following certificates are signed under. Reading the previous
		// boundary in that case would seed a key that is already retired, and
		// the client could never catch up (the boundary it needs is at or below
		// its finalized height, so it can never be submitted).
		let governing_boundary = if is_epoch_boundary(latest.height, epoch_length) {
			Some(latest.height)
		} else if epoch > 0 {
			Some(epoch * epoch_length - 1)
		} else {
			None
		};

		let (network_identity, from_epoch, full_dkg_pending) =
			match (&self.host.network_identity, governing_boundary) {
				// An explicit override still needs the boundary's announcement flag
				// when one is available, otherwise the first genuine rotation would
				// be rejected as unannounced.
				(Some(hex_identity), boundary) => {
					let bytes = hex::decode(hex_identity.trim_start_matches("0x"))
						.context("network_identity override is not valid hex")?;
					let identity: [u8; G2_LEN] = bytes
						.try_into()
						.map_err(|_| anyhow!("network_identity override must be {G2_LEN} bytes"))?;
					let (from_epoch, pending) = match boundary {
						Some(height) => {
							let outcome = self.boundary_outcome(height).await?;
							(outcome.epoch, outcome.is_next_full_dkg)
						},
						None => (epoch, false),
					};
					(identity, from_epoch, pending)
				},
				(None, Some(height)) => {
					let outcome = self.boundary_outcome(height).await?;
					(outcome.network_identity, outcome.epoch, outcome.is_next_full_dkg)
				},
				(None, None) => Err(anyhow!(
					"Tempo is in its genesis epoch; supply `network_identity` in the host config"
				))?,
			};

		let consensus_state = ConsensusState {
			network_identity,
			from_epoch,
			finalized_height: latest.height,
			finalized_hash: latest.digest,
			epoch_length,
			chain_id,
			full_dkg_pending,
		};

		Ok((consensus_state, latest.height))
	}

	/// Reads and decodes the DKG outcome committed by the boundary block at
	/// `height`.
	pub(crate) async fn boundary_outcome(&self, height: u64) -> anyhow::Result<DkgOutcome> {
		let extra_data = self.prover.extra_data(height).await?;
		decode_dkg_outcome(&extra_data)
			.map_err(|e| anyhow!("failed to decode DKG outcome at height {height}: {e}"))
	}

	pub fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}

#[async_trait::async_trait]
impl IsmpHost for TempoHost {
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let interval = tokio::time::interval(Duration::from_secs(
			self.host.consensus_update_frequency.unwrap_or(60),
		));
		let client = self.clone();
		let provider = self.provider();
		let mut interval = Box::pin(interval);
		loop {
			interval.as_mut().tick().await;
			match notification::consensus_updates(&client, counterparty.clone()).await {
				Ok(updates) => {
					for update in updates {
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
								target: "tesseract",
								"Failed to submit transaction to {}: {err:?}",
								counterparty.name()
							);
							// Stale updates would fail anyway; recompute from
							// the on-chain state on the next tick.
							break;
						}
					}
				},
				Err(e) => {
					log::error!(
						target: "tesseract",
						"Consensus task {}->{} encountered an error: {e:?}",
						provider.name(), counterparty.name()
					)
				},
			}
		}
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		let (consensus_state, height) = self.get_consensus_state().await?;
		let header = self.prover.header(height).await?;

		Ok(Some(CreateConsensusState {
			consensus_state: consensus_state.encode(),
			consensus_client_id: ismp_tempo::TEMPO_CONSENSUS_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: self.host.unbonding_period_secs.unwrap_or(60 * 60 * 24 * 14),
			challenge_periods: vec![(
				self.state_machine,
				self.host.challenge_period_secs.unwrap_or(5 * 60),
			)]
			.into_iter()
			.collect(),
			state_machine_commitments: vec![(
				ismp::consensus::StateMachineId {
					state_id: self.state_machine,
					consensus_state_id: self.consensus_state_id,
				},
				StateCommitmentHeight {
					commitment: StateCommitment {
						timestamp: header.inner.timestamp,
						overlay_root: None,
						state_root: header.inner.state_root,
					},
					height,
				},
			)],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}
