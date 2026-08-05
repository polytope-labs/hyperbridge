// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Arc consensus relayer for tesseract.

/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "consensus-arc";

use anyhow::Result;
use arc_prover::ArcProver;
use codec::Encode;
use ismp::{
	consensus::ConsensusStateId,
	host::StateMachine,
	messaging::{CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_arc::ConsensusState;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod notification;

/// Host configuration for the Arc relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcHostConfig {
	/// Frequency (in seconds) to check for new finalized certificates
	pub consensus_update_frequency: Option<u64>,
	/// Arc execution JSON-RPC URL
	pub rpc_url: String,
	/// JSON-RPC URL serving `arc_getCertificate`, when `rpc_url` (e.g. a
	/// third-party provider) doesn't proxy Arc's custom namespace
	pub certificate_rpc_url: Option<String>,
	/// Unbonding period in seconds for CreateConsensusState
	pub unbonding_period_secs: Option<u64>,
}

/// Top-level config for the Arc relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcConfig {
	#[serde(flatten)]
	pub host: ArcHostConfig,
}

impl ArcConfig {
	pub async fn into_client(self, evm_config: EvmConfig) -> anyhow::Result<Arc<dyn IsmpHost>> {
		Ok(Arc::new(ArcHost::new(&self.host, &evm_config).await?))
	}
}

/// The relayer host for Arc
#[derive(Clone)]
pub struct ArcHost {
	pub consensus_state_id: ConsensusStateId,
	pub state_machine: StateMachine,
	pub host: ArcHostConfig,
	pub provider: Arc<dyn IsmpProvider>,
	pub prover: Arc<ArcProver>,
}

impl ArcHost {
	/// Create a new ArcHost
	pub async fn new(host: &ArcHostConfig, evm: &EvmConfig) -> Result<Self, anyhow::Error> {
		let ismp_provider = EvmClient::new(evm.clone()).await?;
		let certificate_rpc_url =
			host.certificate_rpc_url.clone().unwrap_or_else(|| host.rpc_url.clone());
		Ok(Self {
			consensus_state_id: ismp_provider.consensus_state_id,
			state_machine: ismp_provider.state_machine,
			host: host.clone(),
			provider: Arc::new(ismp_provider),
			prover: Arc::new(ArcProver::with_certificate_endpoint(
				host.rpc_url.clone(),
				certificate_rpc_url,
			)?),
		})
	}

	/// Fetch the current consensus state (for initial state creation)
	pub async fn get_consensus_state(&self) -> Result<ConsensusState, anyhow::Error> {
		let verifier_state = self.prover.fetch_latest_verifier_state().await?;

		let chain_id = match self.state_machine {
			StateMachine::Evm(chain_id) => chain_id,
			_ => return Err(anyhow::anyhow!("Unsupported state machine")),
		};

		Ok(ConsensusState {
			current_validators: verifier_state.current_validators,
			finalized_height: verifier_state.finalized_height,
			finalized_hash: verifier_state.finalized_hash,
			chain_id,
		})
	}
}

#[async_trait::async_trait]
impl IsmpHost for ArcHost {
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		use crate::notification::consensus_notification;
		let mut interval = Box::pin(tokio::time::interval(Duration::from_secs(
			self.host.consensus_update_frequency.unwrap_or(300),
		)));
		let client = self.clone();
		let provider = self.provider();
		loop {
			interval.as_mut().tick().await;
			match consensus_notification(&client, counterparty.clone()).await {
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
							target: "tesseract", "Failed to submit transaction to {}: {err:?}",
							counterparty.name()
						)
					}
				},
				Ok(None) => {
					// No update to send, just continue
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
		let initial_consensus_state: ConsensusState = self
			.get_consensus_state()
			.await
			.map_err(|e| anyhow::anyhow!("ArcHost: fetch initial consensus state failed: {e}"))?;

		let header = self
			.prover
			.rpc
			.get_block_by_number(initial_consensus_state.finalized_height)
			.await?;

		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: ismp_arc::ARC_CONSENSUS_CLIENT_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: self.host.unbonding_period_secs.unwrap_or(14 * 24 * 3600),
			challenge_periods: vec![(self.state_machine, 2 * 60)].into_iter().collect(),
			state_machine_commitments: vec![(
				ismp::consensus::StateMachineId {
					state_id: self.state_machine,
					consensus_state_id: self.consensus_state_id,
				},
				StateCommitmentHeight {
					commitment: ismp::consensus::StateCommitment {
						timestamp: header.timestamp,
						overlay_root: None,
						state_root: header.state_root,
					},
					height: initial_consensus_state.finalized_height,
				},
			)],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}
