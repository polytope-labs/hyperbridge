//! Kaia consensus relayer for tesseract.
//!
//! Polls a Kaia node for finalized headers and submits them as ISMP consensus
//! updates. Kaia's Istanbul BFT finalizes every block instantly, so any header
//! can advance the client — but headers carrying validator votes or epoch
//! governance data must never be skipped, so the relayer scans the intervening
//! range and folds those headers into each update.

/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "consensus-kaia";

use anyhow::anyhow;
use codec::Encode;
use ismp::{
	consensus::StateCommitment,
	host::StateMachine,
	messaging::{ConsensusMessage, CreateConsensusState, Message, StateCommitmentHeight},
};
use kaia_prover::{KaiaProver, ProverOptions};
use kaia_verifier::primitives::{KAIA_MAINNET_CHAIN_ID, KAIA_TESTNET_CHAIN_ID};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod notification;

/// Host configuration for the Kaia relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaiaHostConfig {
	/// Frequency (in seconds) to check for new finalized headers
	pub consensus_update_frequency: Option<u64>,
	/// Kaia node JSON-RPC URLs. Must serve the `kaia` namespace (the standard
	/// `eth` namespace omits Kaia's governance header fields). Defaults to the
	/// EVM config's rpc urls.
	pub rpc_urls: Option<Vec<String>>,
	/// Maximum number of blocks to scan (for validator votes and epoch
	/// governance) in a single update while catching up. Each scanned block
	/// costs one RPC round trip, so this bounds both the work per update and
	/// how much progress is lost when a scan fails partway.
	pub max_scan_range: Option<u64>,
	/// Concurrent header requests issued while scanning. Lower this for rate
	/// limited endpoints.
	pub scan_concurrency: Option<usize>,
	/// Unbonding period in seconds for `CreateConsensusState`. Defaults to
	/// Kaia's 7-day stake withdrawal delay.
	pub unbonding_period_secs: Option<u64>,
	/// Challenge period in seconds for `CreateConsensusState`.
	pub challenge_period_secs: Option<u64>,
}

/// Top-level config for the Kaia relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaiaConfig {
	#[serde(flatten)]
	pub host: KaiaHostConfig,
}

impl KaiaConfig {
	pub async fn into_client(self, evm_config: EvmConfig) -> anyhow::Result<Arc<dyn IsmpHost>> {
		Ok(Arc::new(KaiaHost::new(&self.host, &evm_config).await?))
	}
}

/// The relayer host for Kaia
#[derive(Clone)]
pub struct KaiaHost {
	pub consensus_state_id: ismp::consensus::ConsensusStateId,
	pub state_machine: StateMachine,
	pub host: KaiaHostConfig,
	pub provider: Arc<dyn IsmpProvider>,
	pub prover: KaiaProver,
}

impl KaiaHost {
	pub async fn new(host: &KaiaHostConfig, evm: &EvmConfig) -> anyhow::Result<Self> {
		let urls = host.rpc_urls.clone().unwrap_or_else(|| evm.rpc_urls.clone());
		let mut options = ProverOptions::default();
		if let Some(concurrency) = host.scan_concurrency {
			options.scan_concurrency = concurrency;
		}
		let prover = KaiaProver::with_options(urls, options)?;
		let ismp_provider = EvmClient::new(evm.clone()).await?;
		Ok(Self {
			consensus_state_id: ismp_provider.consensus_state_id,
			state_machine: ismp_provider.state_machine,
			host: host.clone(),
			provider: Arc::new(ismp_provider),
			prover,
		})
	}

	/// Blocks scanned per update. Kept modest so that a failed scan costs one
	/// short retry rather than a full re-walk: Kaia produces 86,400 blocks a
	/// day, so catching up runs many updates regardless, and each one that
	/// lands advances the on-chain client permanently.
	pub fn max_scan_range(&self) -> u64 {
		self.host.max_scan_range.unwrap_or(5_000)
	}

	pub fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}

#[async_trait::async_trait]
impl IsmpHost for KaiaHost {
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
			match notification::consensus_update(&client, counterparty.clone()).await {
				Ok(Some(update)) => {
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
					}
				},
				Ok(None) => {},
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
		let chain_id = match self.state_machine {
			StateMachine::Evm(chain_id) => chain_id,
			state_machine => Err(anyhow!("Unsupported state machine: {state_machine}"))?,
		};
		if chain_id != KAIA_MAINNET_CHAIN_ID && chain_id != KAIA_TESTNET_CHAIN_ID {
			log::warn!(target: LOG_TARGET, "Chain id {chain_id} is not a known Kaia network");
		}

		let latest = self.prover.latest_block_number().await?;
		let (consensus_state, header) = self.prover.fetch_consensus_state(latest).await?;

		Ok(Some(CreateConsensusState {
			consensus_state: consensus_state.encode(),
			consensus_client_id: ismp_kaia::KAIA_CONSENSUS_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: self.host.unbonding_period_secs.unwrap_or(60 * 60 * 24 * 7),
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
						timestamp: header.timestamp,
						overlay_root: None,
						state_root: header.state_root,
					},
					height: latest,
				},
			)],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}
