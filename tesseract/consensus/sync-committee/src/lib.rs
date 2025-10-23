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

use arb_host::{ArbConfig, ArbHost};
use ethers::{
	prelude::Provider,
	providers::{Http, Middleware},
};
use ismp::{consensus::ConsensusStateId, host::StateMachine};
pub use ismp_sync_committee::types::{BeaconClientUpdate, ConsensusState};
use ismp_sync_committee::{
	constants::{mainnet::Mainnet, sepolia::Sepolia},
	types::L2Consensus,
};
use op_host::{OpConfig, OpHost};
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use sync_committee_primitives::{
	constants::{gnosis, Config, ETH1_DATA_VOTES_BOUND_ETH, ETH1_DATA_VOTES_BOUND_GNO, PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM, PROPOSER_LOOK_AHEAD_LIMIT_GNO},
	types::VerifierState,
	util::{compute_epoch_at_slot, compute_sync_committee_period_at_slot},
};
use sync_committee_prover::SyncCommitteeProver;
pub use sync_committee_verifier::verify_sync_committee_attestation;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{IsmpHost, IsmpProvider};

mod host;
mod notification;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCommitteeConfig {
	/// Host config
	pub host: HostConfig,
	/// General ethereum config
	#[serde[flatten]]
	pub evm_config: EvmConfig,
	/// Supported L2s
	pub layer_twos: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
	/// Http url for a beacon client
	pub beacon_http_urls: Vec<String>,

	/// Interval in seconds at which consensus updates should happen
	pub consensus_update_frequency: u64,
}

impl SyncCommitteeConfig {
	/// Convert the config into a client.
	pub async fn into_sepolia(
		self,
		l2_config: BTreeMap<StateMachine, L2Config>,
	) -> anyhow::Result<Arc<dyn IsmpHost>> {
		let client = SyncCommitteeHost::<Sepolia, ETH1_DATA_VOTES_BOUND_ETH, PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM>::new(
			&self.host,
			&self.evm_config,
			l2_config,
		)
		.await?;

		Ok(Arc::new(client))
	}

	pub async fn into_mainnet(
		self,
		l2_config: BTreeMap<StateMachine, L2Config>,
	) -> anyhow::Result<Arc<dyn IsmpHost>> {
		let client = SyncCommitteeHost::<Mainnet, ETH1_DATA_VOTES_BOUND_ETH, PROPOSER_LOOK_AHEAD_LIMIT_ETHEREUM>::new(
			&self.host,
			&self.evm_config,
			l2_config,
		)
		.await?;

		Ok(Arc::new(client))
	}

	pub async fn into_chiado(self) -> anyhow::Result<Arc<dyn IsmpHost>> {
		let client = SyncCommitteeHost::<gnosis::Testnet, ETH1_DATA_VOTES_BOUND_GNO, PROPOSER_LOOK_AHEAD_LIMIT_GNO>::new(
			&self.host,
			&self.evm_config,
			Default::default(),
		)
		.await?;

		Ok(Arc::new(client))
	}

	pub async fn into_gnosis(self) -> anyhow::Result<Arc<dyn IsmpHost>> {
		let client = SyncCommitteeHost::<gnosis::Mainnet, ETH1_DATA_VOTES_BOUND_GNO, PROPOSER_LOOK_AHEAD_LIMIT_GNO>::new(
			&self.host,
			&self.evm_config,
			Default::default(),
		)
		.await?;

		Ok(Arc::new(client))
	}

	pub fn state_machine(&self) -> StateMachine {
		self.evm_config.state_machine
	}
}

pub struct SyncCommitteeHost<C: Config, const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize> {
	/// Consensus state id on counterparty chain
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this chain.
	pub state_machine: StateMachine,
	/// L2 consensus clients
	pub l2_clients: BTreeMap<StateMachine, L2Host>,
	/// Consensus prover
	pub prover: SyncCommitteeProver<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>,
	/// Interval in seconds at which consensus updates should happen
	pub consensus_update_frequency: Duration,

	pub evm: EvmConfig,
	/// Eth L1 execution client
	pub el: Arc<Provider<Http>>,

	/// Ismp Provider
	pub provider: Arc<dyn IsmpProvider>,

	// retry policy for consensus client requests
	pub retry: again::RetryPolicy,
}

impl<C: Config, const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize> SyncCommitteeHost<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT> {
	pub async fn new(
		host: &HostConfig,
		evm: &EvmConfig,
		l2_config: BTreeMap<StateMachine, L2Config>,
	) -> Result<Self, anyhow::Error> {
		let prover = SyncCommitteeProver::new(host.beacon_http_urls.clone());
		let el = Provider::new(Http::new_client_with_chain_middleware(
			evm.rpc_urls.iter().map(|url| url.parse()).collect::<Result<_, _>>()?,
			None,
		));

		let provider = Arc::new(EvmClient::new(evm.clone()).await?);

		// Create the hosts for the L2s if config is present
		let mut l2_clients = BTreeMap::new();

		for (state_machine, config) in l2_config {
			match config {
				L2Config::ArbitrumOrbit(arb_config) => {
					let host = ArbHost::new(&arb_config.host, &arb_config.evm_config).await?;
					l2_clients.insert(state_machine, L2Host::ArbitrumOrbit(host));
				},
				L2Config::OpStack(op_config) => {
					let host = OpHost::new(&op_config.host, &op_config.evm_config).await?;
					l2_clients.insert(state_machine, L2Host::OpStack(host));
				},
			}
		}

		Ok(Self {
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(evm.consensus_state_id.as_bytes());
				consensus_state_id
			},
			state_machine: evm.state_machine,
			l2_clients,
			prover,
			provider,
			evm: evm.clone(),
			consensus_update_frequency: Duration::from_secs(host.consensus_update_frequency),
			el: Arc::new(el),
			retry: again::RetryPolicy::fixed(Duration::from_millis(500)),
		})
	}

	pub async fn get_consensus_state(
		&self,
		params: GetConsensusStateParams,
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

		let mut l2_consensus = BTreeMap::new();
		for (state_machine, address) in params.l2_oracle_address {
			l2_consensus.insert(state_machine, L2Consensus::OpL2Oracle(address));
		}

		for (state_machine, (address, respected_game_types)) in params.dispute_factory_address {
			l2_consensus.insert(
				state_machine,
				L2Consensus::OpFaultProofGames((address, respected_game_types)),
			);
		}

		for (state_machine, address) in params.rollup_core_address {
			l2_consensus.insert(state_machine, L2Consensus::ArbitrumOrbit(address));
		}

		let chain_id = self.el.get_chainid().await?;
		let consensus_state = ConsensusState {
			frozen_height: None,
			light_client_state: client_state,
			chain_id: chain_id.low_u32(),
			l2_consensus,
		};

		Ok(consensus_state)
	}
}

#[derive(Clone)]
/// Hosts for the various l2s
pub enum L2Host {
	ArbitrumOrbit(ArbHost),
	OpStack(OpHost),
}

#[derive(Clone)]
/// Configuration for various L2 consensus types
pub enum L2Config {
	ArbitrumOrbit(ArbConfig),
	OpStack(OpConfig),
}

impl<C: Config, const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize> Clone
	for SyncCommitteeHost<C, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>
{
	fn clone(&self) -> Self {
		Self {
			consensus_state_id: self.consensus_state_id,
			state_machine: self.state_machine,
			l2_clients: self.l2_clients.clone(),
			prover: self.prover.clone(),
			consensus_update_frequency: self.consensus_update_frequency,
			evm: self.evm.clone(),
			el: self.el.clone(),
			retry: self.retry.clone(),
			provider: self.provider.clone(),
		}
	}
}

pub struct GetConsensusStateParams {
	pub l2_oracle_address: BTreeMap<StateMachine, H160>,
	pub rollup_core_address: BTreeMap<StateMachine, H160>,
	pub dispute_factory_address: BTreeMap<StateMachine, (H160, Vec<u32>)>,
}
