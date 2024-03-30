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

//! Relayer configuration options

use ismp::host::StateMachine;
use serde::{Deserialize, Serialize};

/// Configuration options for the relayer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelayerConfig {
	/// Modules we're interested in relaying
	pub module_filter: Option<Vec<Vec<u8>>>,
	/// Relay consensus messages
	pub consensus: Option<bool>,
	/// Consensus stream timeout
	pub consensus_stream_timeout: Option<u64>,
	/// Relay messages
	pub messaging: Option<bool>,
	/// Fisherman protocol
	pub fisherman: Option<bool>,
	/// Challenege period to be set on consensus states
	pub challenge_period: Option<u64>,
	/// Minimum profit percentage. e.g. 5 -> 5%, 10 -> 10%
	pub minimum_profit_percentage: u32,
	/// How frequently to initiate withdrawals in seconds.
	pub withdrawal_frequency: Option<u64>,
	/// Minimum amount to withdraw when auto-withdrawing
	pub minimum_withdrawal_amount: Option<u64>,
	/// How frequently to retry unprofitable messages in seconds.
	pub unprofitable_retry_frequency: Option<u64>,
	/// Delivery endpoints: chains you intend to deliver messages to
	pub delivery_endpoints: Vec<StateMachine>,
}

/// Hyperbridge's parachain runtimes
#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub enum Chain {
	/// Rococo parachain
	Gargantua,
	/// Kusama Parachain
	Messier,
	/// Local devnet
	Dev,
}

impl Chain {
	pub fn para_id(&self) -> u32 {
		match self {
			Chain::Gargantua => 4374,
			Chain::Messier => 3340,
			Chain::Dev => 2000,
		}
	}

	pub fn state_machine(&self) -> StateMachine {
		StateMachine::Kusama(self.para_id())
	}
}

impl Default for Chain {
	fn default() -> Self {
		Self::Dev
	}
}
