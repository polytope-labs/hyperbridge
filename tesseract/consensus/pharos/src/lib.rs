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

//! Tesseract consensus relayer for Pharos Network.

use anyhow::Result;
use codec::Encode;
use ismp::{
	consensus::{ConsensusStateId, StateCommitment},
	host::StateMachine,
	messaging::{ConsensusMessage, CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_pharos::{ConsensusState, PHAROS_CONSENSUS_CLIENT_ID};
use pharos_primitives::Config;
use pharos_prover::PharosProver;
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, sync::Arc, time::Duration};
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod notification;

pub use pharos_primitives::{Mainnet, Testnet};

/// Host configuration for Pharos relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PharosHostConfig {
	/// Frequency (in seconds) to check for new updates
	pub consensus_update_frequency: Option<u64>,
	/// Pharos JSON-RPC URL
	pub rpc_url: String,
}

/// Top-level config for Pharos relayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PharosConfig {
	/// Host configuration options
	pub host: PharosHostConfig,
	/// General EVM config
	#[serde(flatten)]
	pub evm_config: EvmConfig,
}

impl PharosConfig {
	/// Convert the config into a client.
	pub async fn into_client<C: Config + 'static>(self) -> anyhow::Result<Arc<dyn IsmpHost>> {
		Ok(Arc::new(PharosHost::<C>::new(&self.host, &self.evm_config).await?))
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

/// The relayer host for Pharos
#[derive(Clone)]
pub struct PharosHost<C: Config> {
	/// Consensus state id on counterparty chain
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this chain
	pub state_machine: StateMachine,
	/// Host config options
	pub host: PharosHostConfig,
	/// Ismp provider
	pub provider: Arc<dyn IsmpProvider>,
	/// Pharos prover for fetching proofs
	pub prover: PharosProver<C>,
	/// Phantom data for config
	_config: PhantomData<C>,
}

impl<C: Config> PharosHost<C> {
	/// Create a new PharosHost
	pub async fn new(host: &PharosHostConfig, evm: &EvmConfig) -> Result<Self, anyhow::Error> {
		let ismp_provider = EvmClient::new(evm.clone()).await?;
		let prover = PharosProver::new(&host.rpc_url);

		Ok(Self {
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(evm.consensus_state_id.as_bytes());
				consensus_state_id
			},
			state_machine: evm.state_machine,
			host: host.clone(),
			provider: Arc::new(ismp_provider),
			prover,
			_config: PhantomData,
		})
	}

	/// Fetch the current consensus state (for initial state creation)
	pub async fn get_consensus_state(&self) -> Result<ConsensusState, anyhow::Error> {
		let latest_block = self.prover.get_latest_block().await?;
		let update = self.prover.fetch_block_update(latest_block).await?;

		let header = &update.header;
		let header_hash = geth_primitives::Header::from(header).hash::<KeccakHasher>();

		let current_epoch = C::compute_epoch(latest_block);

		// try to get validator set for an epoch boundary
		// else, we query the previous epoch boundary
		let validator_set = if let Some(ref proof) = update.validator_set_proof {
			pharos_verifier::state_proof::verify_validator_set_proof::<KeccakHasher>(
				header.state_root,
				proof,
				current_epoch + 1,
			)?
		} else {
			// For initial state, fetch from an epoch boundary block
			let epoch_start = current_epoch * C::EPOCH_LENGTH_BLOCKS;
			let epoch_boundary = if epoch_start > 0 { epoch_start - 1 } else { 0 };
			let boundary_update = self.prover.fetch_block_update(epoch_boundary).await?;

			if let Some(ref proof) = boundary_update.validator_set_proof {
				pharos_verifier::state_proof::verify_validator_set_proof::<KeccakHasher>(
					boundary_update.header.state_root,
					proof,
					current_epoch,
				)?
			} else {
				return Err(anyhow::anyhow!("Cannot get initial validator set"));
			}
		};

		let chain_id = match self.state_machine {
			StateMachine::Evm(chain_id) => chain_id,
			_ => return Err(anyhow::anyhow!("Unsupported state machine")),
		};

		Ok(ConsensusState {
			current_validators: validator_set,
			finalized_height: latest_block,
			finalized_hash: header_hash,
			current_epoch,
			chain_id,
		})
	}
}

/// Keccak256 hasher implementation
pub struct KeccakHasher;

impl ismp::messaging::Keccak256 for KeccakHasher {
	fn keccak256(bytes: &[u8]) -> primitive_types::H256
	where
		Self: Sized,
	{
		sp_core::keccak_256(bytes).into()
	}
}

#[async_trait::async_trait]
impl<C: Config + 'static> IsmpHost for PharosHost<C> {
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
					let consensus_message = ConsensusMessage {
						consensus_proof: update.encode(),
						consensus_state_id: client.consensus_state_id,
						signer: counterparty.address(),
					};

					log::info!(
						target: "tesseract",
						"Transmitting consensus message from {} to {}",
						provider.name(),
						counterparty.name()
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
					log::error!(
						target: "tesseract",
						"Consensus task {}->{} encountered an error: {e:?}",
						provider.name(),
						counterparty.name()
					)
				},
			}
		}
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		let initial_consensus_state = self.get_consensus_state().await.map_err(|e| {
			anyhow::anyhow!("PharosHost: fetch initial consensus state failed: {e}")
		})?;

		let latest_block = self.prover.get_latest_block().await?;
		let update = self.prover.fetch_block_update(latest_block).await?;

		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: PHAROS_CONSENSUS_CLIENT_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 60 * 60 * 27,
			challenge_periods: vec![(self.state_machine, 5 * 60)].into_iter().collect(),
			state_machine_commitments: vec![(
				ismp::consensus::StateMachineId {
					state_id: self.state_machine,
					consensus_state_id: self.consensus_state_id,
				},
				StateCommitmentHeight {
					commitment: StateCommitment {
						timestamp: update.header.timestamp,
						overlay_root: None,
						state_root: update.header.state_root,
					},
					height: latest_block,
				},
			)],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}
