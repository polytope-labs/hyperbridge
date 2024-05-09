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
use ismp::host::StateMachine;
use serde::{Deserialize, Serialize};

pub use zk_beefy::Network;

// mod byzantine;
pub mod host;
pub mod prover;

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
