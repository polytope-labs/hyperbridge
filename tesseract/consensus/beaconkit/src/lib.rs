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

//! BeaconKit consensus relayer for tesseract.
//!
//! This module provides a consensus relayer for BeaconKit chains (e.g., Berachain)
//! that combines Tendermint consensus with Ethereum execution layer via signed beacon blocks.

use anyhow::Result;
use codec::Encode;
use ismp::{
	consensus::{ConsensusStateId, StateCommitment},
	host::StateMachine,
	messaging::{CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_beacon_kit::{ConsensusState, BEACON_KIT_CONSENSUS_CLIENT_ID};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tendermint_primitives::{self, Client, CodecTrustedState};
use tendermint_prover::CometBFTClient;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod notification;

/// Host configuration for BeaconKit relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconKitHostConfig {
	/// Frequency (in seconds) to check for new updates
	pub consensus_update_frequency: Option<u64>,
	/// CometBFT Json RPC URL
	pub rpc_url: String,
	/// Trusting period in seconds for light client verification
	pub trusting_period_secs: Option<u64>,
	/// Unbonding period in seconds for CreateConsensusState
	pub unbonding_period_secs: Option<u64>,
}

/// Top-level config for BeaconKit relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconKitConfig {
	pub host: BeaconKitHostConfig,
	#[serde(flatten)]
	pub evm_config: EvmConfig,
}

impl BeaconKitConfig {
	pub async fn into_client(self) -> anyhow::Result<Arc<dyn IsmpHost>> {
		Ok(Arc::new(BeaconKitHost::new(&self.host, &self.evm_config).await?))
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

/// The relayer host for BeaconKit
#[derive(Clone)]
pub struct BeaconKitHost {
	pub consensus_state_id: ConsensusStateId,
	pub state_machine: StateMachine,
	pub host: BeaconKitHostConfig,
	pub provider: Arc<dyn IsmpProvider>,
	pub prover: Arc<CometBFTClient>,
}

impl BeaconKitHost {
	/// Create a new BeaconKitHost
	pub async fn new(host: &BeaconKitHostConfig, evm: &EvmConfig) -> Result<Self, anyhow::Error> {
		let ismp_provider = EvmClient::new(evm.clone()).await?;
		Ok(Self {
			consensus_state_id: Default::default(),
			state_machine: evm.state_machine,
			host: host.clone(),
			provider: Arc::new(ismp_provider),
			prover: Arc::new(CometBFTClient::new(&host.rpc_url).await?),
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
			trusted_header.header.chain_id.clone().into(),
			trusted_header.header.height.into(),
			trusted_header.header.time.unix_timestamp() as u64,
			trusted_header.header.hash().as_bytes().try_into().unwrap(),
			trusted_validators,
			trusted_next_validators,
			trusted_header.header.next_validators_hash.as_bytes().try_into().unwrap(),
			self.host.trusting_period_secs.unwrap_or(82 * 3600),
			tendermint_primitives::VerificationOptions::default(),
		);

		let codec_trusted_state = CodecTrustedState::from(&trusted_state);

		let chain_id = match self.state_machine {
			StateMachine::Evm(chain_id) => chain_id,
			_ => return Err(anyhow::anyhow!("Unsupported state machine")),
		};

		let consensus_state = ConsensusState { tendermint_state: codec_trusted_state, chain_id };

		Ok(consensus_state)
	}

	/// Get the ISMP provider
	pub fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}

#[async_trait::async_trait]
impl IsmpHost for BeaconKitHost {
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
						"🛰️ Transmitting BeaconKit consensus message from {} to {}",
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
					log::error!(target: "tesseract","BeaconKit consensus task {}->{} encountered an error: {e:?}", provider.name(), counterparty.name())
				},
			}
		}
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		let initial_consensus_state: ConsensusState =
			self.get_consensus_state().await.map_err(|e| {
				anyhow::anyhow!("BeaconKitHost: fetch initial consensus state failed: {e}")
			})?;

		let header = self
			.prover
			.signed_header(initial_consensus_state.tendermint_state.height.into())
			.await?;
		let app_hash: [u8; 32] = header.header.app_hash.as_bytes().try_into().unwrap();

		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: BEACON_KIT_CONSENSUS_CLIENT_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: self.host.unbonding_period_secs.unwrap_or(82 * 3600),
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
						state_root: primitive_types::H256(app_hash),
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
