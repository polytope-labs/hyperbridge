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


//! Weights for `pallet_beefy_consensus_proofs`.
//!
//! `submit_proof`, `set_proof_reward` and `set_sp1_vkey_hash` use numbers ported from
//! the `pallet_outbound_proofs` benchmark run on an AMD Ryzen Threadripper PRO
//! 5995WX (2026-04-18, wasm-execution=compiled, steps=50, repeat=20). The pallet was
//! subsequently renamed and its extrinsic surface redesigned (submit_proof is now
//! unsigned + SR25519-authed, `initialize_state` was added), so these numbers are a
//! close-but-not-exact starting point — regenerate once benchmarks are wired into CI.
//!
//! Original per-bench numbers:
//!   submit_proof         ~669ms   5r/2w   (dominated by SP1 verification)
//!   set_proof_reward     ~8.7µs   0r/1w
//!   set_sp1_vkey_hash    ~4.8µs   0r/1w

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use polkadot_sdk::*;
use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_beefy_consensus_proofs`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_beefy_consensus_proofs::WeightInfo for WeightInfo<T> {
	/// Storage: `Ismp::ConsensusStates` (r:1 w:1)
	/// Storage: `BeefyConsensusProofs::Sp1VkeyHash` (r:1 w:0)
	/// Storage: `BeefyConsensusProofs::LastProvenHeight` (r:1 w:1)
	/// Storage: `BeefyConsensusProofs::LastRewardedDispatchRoot` (r:1 w:1)
	/// Storage: `BeefyConsensusProofs::RecentProofs` (r:1 w:1)
	/// Storage: `BeefyConsensusProofs::ProofReward` (r:1 w:0)
	fn submit_proof() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `547`
		//  Estimated: `4012`
		// Minimum execution time: 669_751_633_000 picoseconds.
		Weight::from_parts(694_774_527_000, 0)
			.saturating_add(Weight::from_parts(0, 4012))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `BeefyConsensusProofs::ProofReward` (r:0 w:1)
	fn set_proof_reward() -> Weight {
		// Minimum execution time: 8_696_000 picoseconds.
		Weight::from_parts(9_027_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `BeefyConsensusProofs::Sp1VkeyHash` (r:0 w:1)
	fn set_sp1_vkey_hash() -> Weight {
		// Minimum execution time: 4_779_000 picoseconds.
		Weight::from_parts(5_490_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Ismp::ConsensusStateClient` (r:1 w:1)
	/// Storage: `Ismp::ConsensusStates` (r:0 w:1)
	/// Storage: `Ismp::UnbondingPeriod` (r:0 w:1)
	/// Storage: `Ismp::ConsensusClientUpdateTime` (r:0 w:1)
	/// Storage: `BeefyConsensusProofs::LastProvenHeight` (r:0 w:1)
	/// Storage: `BeefyConsensusProofs::LastRewardedDispatchRoot` (r:0 w:1)
	fn initialize_state() -> Weight {
		// Approximated: similar cost class to `set_sp1_vkey_hash`, plus several writes.
		Weight::from_parts(20_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(6))
	}
}
