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

use anyhow::anyhow;
use beefy_prover::{
	relay::{fetch_latest_beefy_justification, fetch_next_beefy_justification},
	runtime::{self},
};
use beefy_verifier_primitives::ConsensusState;
use codec::Decode;
use ismp::{consensus::ConsensusStateId, host::StateMachine, messaging::ConsensusMessage};
use prover::Prover;
use serde::{Deserialize, Serialize};
use sp_core::H160;
use sp_runtime::traits::Keccak256;
use std::{sync::Arc, time::Duration};
use subxt::{config::Header, ext::sp_runtime::traits::Zero};
use tesseract_primitives::IsmpProvider;
use tesseract_substrate::{SubstrateClient, SubstrateConfig};
use tokio::{sync::broadcast, time};
pub use zk_beefy::Network;

// mod byzantine;
// mod host;
mod prover;

const VALIDATOR_SET_ID_KEY: [u8; 32] =
	hex_literal::hex!("08c41974a97dbf15cfbec28365bea2da8f05bccc2f70ec66a32999c5761156be");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
	/// RPC ws url for a relay chain
	pub relay_rpc_ws: String,
	/// Interval in seconds at which consensus updates should happen
	pub consensus_update_frequency: u64,
	/// The intended network for zk beefy
	pub zk_beefy: Option<Network>,
}

pub(crate) fn extract_para_id(state_machine: StateMachine) -> Result<u32, anyhow::Error> {
	let para_id = match state_machine {
		StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id,
		_ => Err(anyhow!("Invalid state machine: {state_machine:?}"))?,
	};

	Ok(para_id)
}

async fn highest_consensus_state(
	clients: &[Arc<dyn IsmpProvider>],
) -> Result<ConsensusState, anyhow::Error> {
	let mut consensus_states = vec![];
	for client in clients {
		match client
			.query_consensus_state(None, client.state_machine_id().consensus_state_id.clone())
			.await
		{
			Ok(cs_state) => {
				let consensus_state = ConsensusState::decode(&mut &cs_state[..])
					.expect("Consensus state should always decode correctly");
				consensus_states.push(consensus_state);
			},

			Err(_) => {
				log::error!(
					"Failed to fetch consensus state for {:?} in beefy prover",
					client.state_machine_id().state_id
				)
			},
		}
	}

	let max = consensus_states
		.into_iter()
		.max_by(|a, b| a.latest_beefy_height.cmp(&b.latest_beefy_height))
		.ok_or_else(|| anyhow!("No consensus state found for all clients"))?;
	Ok(max)
}
