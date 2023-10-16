// Copyright (C) 2023 Polytope Labs.
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

pub use consensus_client::types::{BeaconClientUpdate, ConsensusState};
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use primitive_types::H160;
use primitives::{
	types::VerifierState,
	util::{compute_epoch_at_slot, compute_sync_committee_period_at_slot},
};
use prover::SyncCommitteeProver;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::Duration};
use tesseract_evm::{arbitrum::client::ArbHost, optimism::client::OpHost, EvmClient, EvmConfig};
use tesseract_primitives::IsmpProvider;
pub use verifier::verify_sync_committee_attestation;

mod byzantine;
mod host;
#[cfg(any(test, feature = "testing"))]
pub mod mock;
mod notification;
#[cfg(test)]
mod test;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCommitteeConfig {
	/// Http url for a beacon client
	pub beacon_http_url: String,
	/// General ethereum config
	#[serde[flatten]]
	pub evm_config: EvmConfig,
	/// Interval in seconds at which consensus updates should happen
	pub consensus_update_frequency: u64,
}

impl SyncCommitteeConfig {
	/// Convert the config into a client.
	pub async fn into_client<C: IsmpProvider>(
		self,
		counterparty: &C,
	) -> anyhow::Result<EvmClient<SyncCommitteeHost>> {
		let host = SyncCommitteeHost::new(&self).await?;
		let client = EvmClient::new(host, self.evm_config, counterparty).await?;

		Ok(client)
	}
}

#[derive(Clone)]
pub struct SyncCommitteeHost {
	/// Consensus state id on counterparty chain
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this chain.
	pub state_machine: StateMachine,
	/// Arbitrum  client
	pub arbitrum_client: Option<ArbHost>,
	/// Optimism  client
	pub optimism_client: Option<OpHost>,
	/// Base  client
	pub base_client: Option<OpHost>,
	/// Consensus prover
	pub prover: SyncCommitteeProver,
	/// Http URl beacon chain, required for subscribing to events SSE
	pub beacon_node_rpc: String,
	/// Interval in seconds at which consensus updates should happen
	pub consensus_update_frequency: Duration,
	/// Config
	pub config: SyncCommitteeConfig,
}

impl SyncCommitteeHost {
	pub async fn new(config: &SyncCommitteeConfig) -> Result<Self, anyhow::Error> {
		let prover = SyncCommitteeProver::new(config.beacon_http_url.clone());
		Ok(Self {
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(config.evm_config.consensus_state_id.as_bytes());
				consensus_state_id
			},
			state_machine: config.evm_config.state_machine,
			arbitrum_client: None,
			optimism_client: None,
			base_client: None,
			prover,
			beacon_node_rpc: config.beacon_http_url.clone(),
			consensus_update_frequency: Duration::from_secs(config.consensus_update_frequency),
			config: config.clone(),
		})
	}

	pub fn set_arb_host(&mut self, host: ArbHost) {
		self.arbitrum_client = Some(host)
	}

	pub fn set_op_host(&mut self, host: OpHost) {
		self.optimism_client = Some(host)
	}

	pub fn set_base_host(&mut self, host: OpHost) {
		self.base_client = Some(host)
	}

	pub async fn get_initial_consensus_state(
		&self,
		ismp_contract_addresses: BTreeMap<StateMachine, H160>,
		l2_oracle: BTreeMap<StateMachine, H160>,
		rollup_core: H160,
		trusted_block_id: Option<&str>,
	) -> Result<ConsensusState, anyhow::Error> {
		let block_id = trusted_block_id.unwrap_or("finalized");
		let block_header = self.prover.fetch_header(&block_id).await?;
		let state = self.prover.fetch_beacon_state(&block_header.slot.to_string()).await?;

		let client_state = VerifierState {
			finalized_header: block_header.clone(),
			latest_finalized_epoch: compute_epoch_at_slot(block_header.slot),
			current_sync_committee: state.current_sync_committee,
			next_sync_committee: state.next_sync_committee,
			state_period: compute_sync_committee_period_at_slot(block_header.slot),
		};

		let consensus_state = ConsensusState {
			frozen_height: None,
			light_client_state: client_state,
			ismp_contract_addresses,
			l2_oracle_address: l2_oracle,
			rollup_core_address: rollup_core,
		};

		Ok(consensus_state)
	}
}
