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

use ethers::prelude::{Provider, Ws};
use ismp::{consensus::ConsensusStateId, host::StateMachine};
pub use ismp_sync_committee::types::{BeaconClientUpdate, ConsensusState};
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use sync_committee_primitives::{
	constants::Config,
	types::VerifierState,
	util::{compute_epoch_at_slot, compute_sync_committee_period_at_slot},
};
use sync_committee_prover::SyncCommitteeProver;
pub use sync_committee_verifier::verify_sync_committee_attestation;
use tesseract_evm::{arbitrum::client::ArbHost, optimism::client::OpHost, EvmClient, EvmConfig};

mod byzantine;
mod host;
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
	pub async fn into_client<T: Config + Send + Sync + 'static>(
		self,
	) -> anyhow::Result<EvmClient<SyncCommitteeHost<T>>> {
		let host = SyncCommitteeHost::new(&self).await?;
		let client = EvmClient::new(host, self.evm_config).await?;

		Ok(client)
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

pub struct SyncCommitteeHost<C: Config> {
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
	pub prover: SyncCommitteeProver<C>,
	/// Http URl beacon chain, required for subscribing to events SSE
	pub beacon_node_rpc: String,
	/// Interval in seconds at which consensus updates should happen
	pub consensus_update_frequency: Duration,
	/// Config
	pub config: SyncCommitteeConfig,
	/// Eth L1 execution client
	pub el: Arc<Provider<Ws>>,
}

impl<C: Config> SyncCommitteeHost<C> {
	pub async fn new(config: &SyncCommitteeConfig) -> Result<Self, anyhow::Error> {
		let prover = SyncCommitteeProver::new(config.beacon_http_url.clone());
		let el =
			Provider::<Ws>::connect_with_reconnects(&config.evm_config.execution_ws, 1000).await?;
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
			el: Arc::new(el),
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

	pub fn set_l2_hosts(&mut self, hosts: Vec<L2Host>) {
		for host in hosts {
			match host {
				L2Host::Arb(host) => self.set_arb_host(host),
				L2Host::Op(host) => self.set_op_host(host),
				L2Host::Base(host) => self.set_base_host(host),
			}
		}
	}

	pub async fn get_consensus_state(
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
			latest_finalized_epoch: compute_epoch_at_slot::<C>(block_header.slot),
			current_sync_committee: state.current_sync_committee,
			next_sync_committee: state.next_sync_committee,
			state_period: compute_sync_committee_period_at_slot::<C>(block_header.slot),
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

/// Hosts for the various l2s
pub enum L2Host {
	Arb(ArbHost),
	Op(OpHost),
	Base(OpHost),
}

impl<C: Config> Clone for SyncCommitteeHost<C> {
	fn clone(&self) -> Self {
		Self {
			consensus_state_id: self.consensus_state_id,
			state_machine: self.state_machine,
			arbitrum_client: self.arbitrum_client.clone(),
			optimism_client: self.optimism_client.clone(),
			base_client: self.base_client.clone(),
			prover: self.prover.clone(),
			beacon_node_rpc: self.beacon_node_rpc.clone(),
			consensus_update_frequency: self.consensus_update_frequency,
			config: self.config.clone(),
			el: self.el.clone(),
		}
	}
}
