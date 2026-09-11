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
	/// Minimum profit in basis points. e.g. 500 -> 5%, 1000 -> 10%
	pub minimum_profit_percentage: u32,
	/// How frequently to initiate withdrawals in seconds.
	pub withdrawal_frequency: Option<u64>,
	/// Minimum amount to withdraw when auto-withdrawing
	pub minimum_withdrawal_amount: Option<u64>,
	/// How frequently to retry parked deliveries, in seconds. Defaults to five
	/// minutes when unset; whether retries run at all is decided by `retry_modules`.
	pub retry_frequency: Option<u64>,
	/// Destination modules, as hex encoded module ids, whose requests are parked and retried
	/// when a delivery to an EVM chain is cancelled or never lands. Matched on the request's
	/// `to` field. A non empty list is what switches the retry task on; absent or empty
	/// parks nothing and spawns no retry loop.
	pub retry_modules: Option<Vec<String>>,
	/// Delivery endpoints: chains you intend to deliver messages to
	pub delivery_endpoints: Vec<String>,
	/// Flag to tell the messsaging process to deliver failed transactions
	pub deliver_failed: Option<bool>,
	/// Should the relayer run the fee accumulation task?
	pub disable_fee_accumulation: Option<bool>,
}

impl RelayerConfig {
	/// Whether undelivered messages are parked and a retry loop drains them, which
	/// is the case as soon as the operator lists a module in `retry_modules`.
	pub fn retries_enabled(&self) -> bool {
		self.retry_modules.as_ref().map_or(false, |modules| !modules.is_empty())
	}
}
