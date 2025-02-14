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

use frame_support::weights::Weight;
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
}
