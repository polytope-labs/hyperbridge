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

//! One-shot runtime migrations for the gargantua runtime.

use crate::Runtime;
use cumulus_pallet_parachain_system::{LastProcessedDownwardMessage, LastProcessedHrmpMessage};
use polkadot_sdk::{
	frame_support::{traits::OnRuntimeUpgrade, weights::Weight},
	frame_system,
};

/// Resets the parachain-system message-processing cursors ahead of the Paseo relay-chain
/// migration.
///
/// The new relay chain restarts block numbering below values this parachain has already
/// observed. `LastProcessedDownwardMessage` / `LastProcessedHrmpMessage` record the relay
/// block number (`sent_at`) of the last message consumed on each queue. If left untouched,
/// messages from the new relay carry a *lower* `sent_at` than the stored cursor and are
/// silently dropped as "already processed" while building the `set_validation_data` inherent;
/// the resulting message-queue-chain mismatch then panics the inherent with a
/// "DMQ head mismatch" and halts the chain.
///
/// Killing both cursors makes the pallet fall back to the current relay-parent number
/// (`LastRelayChainBlockNumber`) when filtering, so messages from the new relay are processed
/// correctly.
///
/// This is a one-shot migration: remove it from the `Migrations` tuple once the upgrade has
/// been enacted on-chain, alongside reverting `CheckAssociatedRelayNumber` back to
/// `RelayNumberMonotonicallyIncreases`.
pub struct ResetDownwardMessageState;

impl OnRuntimeUpgrade for ResetDownwardMessageState {
	fn on_runtime_upgrade() -> Weight {
		LastProcessedDownwardMessage::<Runtime>::kill();
		LastProcessedHrmpMessage::<Runtime>::kill();

		// Two storage writes, no reads.
		<Runtime as frame_system::Config>::DbWeight::get().writes(2)
	}
}
