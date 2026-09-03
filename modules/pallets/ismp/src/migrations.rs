// Copyright (c) 2025 Polytope Labs.
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

//! Storage migrations for `pallet-ismp`.

use core::marker::PhantomData;
use polkadot_sdk::{
	frame_support::{
		pallet_prelude::Weight,
		traits::{Get, OnRuntimeUpgrade},
	},
	*,
};
use ismp::{
	consensus::{ConsensusStateId, StateMachineId},
	host::StateMachine,
};

use crate::{Config, StateMachineCommitmentCap};

/// Retention window these caps are sized for, in seconds.
const RETENTION_WINDOW_SECS: u32 = 6 * 60 * 60;

/// Per-chain retention caps for chains whose block cadence makes the default
/// [`MAX_STATE_MACHINE_COMMITMENTS`](crate::pallet::MAX_STATE_MACHINE_COMMITMENTS)
/// too shallow. Each cap is `RETENTION_WINDOW_SECS / block time`, expressed in
/// milliseconds to keep sub-second block times exact.
const COMMITMENT_CAPS: [(StateMachine, ConsensusStateId, u32); 2] = [
	// BSC: 450ms blocks -> 48,000
	(StateMachine::Evm(56), *b"BSC0", RETENTION_WINDOW_SECS * 1_000 / 450),
	// Polygon: 2s blocks -> 10,800
	(StateMachine::Evm(137), *b"POLY", RETENTION_WINDOW_SECS * 1_000 / 2_000),
];

/// Seeds [`StateMachineCommitmentCap`] for fast-finality chains. Each cap is
/// only written if the chain has no cap configured yet, so re-running this
/// migration never clobbers a value set by governance in the meantime.
pub struct SeedCommitmentCaps<T>(PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for SeedCommitmentCaps<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut writes = 0u64;
		for (state_id, consensus_state_id, cap) in COMMITMENT_CAPS {
			let id = StateMachineId { state_id, consensus_state_id };
			if !StateMachineCommitmentCap::<T>::contains_key(id) {
				StateMachineCommitmentCap::<T>::insert(id, cap);
				log::info!(
					target: "ismp",
					"Seeded state commitment cap for {state_id:?}: {cap}",
				);
				writes += 1;
			}
		}

		<T as frame_system::Config>::DbWeight::get()
			.reads_writes(COMMITMENT_CAPS.len() as u64, writes)
	}
}
