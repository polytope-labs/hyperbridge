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

//! Weight trait for `pallet-beefy-consensus-proofs`.
//!
//! Runtimes pick an implementation of [`WeightInfo`] (benchmarked or stub) and wire it
//! into [`Config::WeightInfo`](crate::pallet::Config::WeightInfo).

use polkadot_sdk::*;

use frame_support::weights::Weight;

/// Weight functions needed by `pallet-beefy-consensus-proofs`.
pub trait WeightInfo {
	/// Weight of `submit_proof`.
	fn submit_proof() -> Weight;
	/// Weight of `initialize_state`.
	fn initialize_state() -> Weight;
	/// Weight of `set_proof_reward`.
	fn set_proof_reward() -> Weight;
	/// Weight of `set_sp1_vkey_hash`.
	fn set_sp1_vkey_hash() -> Weight;
}

/// No-op [`WeightInfo`] for tests and genesis bootstrap.
impl WeightInfo for () {
	fn submit_proof() -> Weight {
		Weight::zero()
	}
	fn initialize_state() -> Weight {
		Weight::zero()
	}
	fn set_proof_reward() -> Weight {
		Weight::zero()
	}
	fn set_sp1_vkey_hash() -> Weight {
		Weight::zero()
	}
}
