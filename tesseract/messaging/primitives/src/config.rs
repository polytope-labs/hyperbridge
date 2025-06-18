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

use serde::{Deserialize, Serialize};

/// Configuration options for the relayer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelayerConfig {
	/// Modules we're interested in relaying
	pub module_filter: Option<Vec<String>>,
	/// Minimum profit percentage. e.g. 5 -> 5%, 10 -> 10%
	pub minimum_profit_percentage: u32,
	/// How frequently to initiate withdrawals in seconds.
	pub withdrawal_frequency: Option<u64>,
	/// Minimum amount to withdraw when auto-withdrawing
	pub minimum_withdrawal_amount: Option<u64>,
	/// How frequently to retry unprofitable or failed messages in seconds.
	/// If this is value not supplied retries will not be enabled
	pub unprofitable_retry_frequency: Option<u64>,
	/// Delivery endpoints: chains you intend to deliver messages to
	pub delivery_endpoints: Vec<String>,
	/// Flag to tell the messsaging process to deliver failed transactions
	pub deliver_failed: Option<bool>,
	/// Start fisherman task?
	pub fisherman: Option<bool>,
	/// Should the relayer run the fee accumulation task?
	pub disable_fee_accumulation: Option<bool>,
}
