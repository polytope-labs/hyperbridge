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
use polkadot_sdk::*;

use frame_support::weights::{constants::RocksDbWeight, Weight};

/// No-op `WeightInfo` impl for tests and the no-op runtime configuration.
impl WeightInfo for () {
	fn add_parachain(_n: u32) -> Weight {
		Weight::zero()
	}
	fn remove_parachain(_n: u32) -> Weight {
		Weight::zero()
	}
	fn update_parachain_consensus() -> Weight {
		Weight::zero()
	}
}
/// The weight information provider trait for dispatchable extrinsics
pub trait WeightInfo {
	/// Weight for adding parachains, scaled by the number of machines
	/// * n: The number of parachains being added
	fn add_parachain(n: u32) -> Weight;
	/// Weight for removing parachains, scaled by the number of machines
	/// * n: The number of parachains being removed
	fn remove_parachain(n: u32) -> Weight;
	/// Weight for updating a parachain's consensus
	fn update_parachain_consensus() -> Weight;

	/// Worst-case weight of the steady-state `on_finalize` insert + bounded eviction.
	/// Conservative default; runtimes should override with benchmarked numbers.
	fn on_finalize_bound_relay_state_commitments() -> Weight {
		RocksDbWeight::get().reads_writes(68, 4)
	}

	/// Worst-case weight of one `SteppedMigration` step that drains a single
	/// `RelayChainStateCommitments` entry. Conservative default; runtimes should
	/// override with benchmarked numbers.
	fn migrate_relay_state_commitments_step() -> Weight {
		RocksDbWeight::get().reads_writes(2, 3)
	}
}
