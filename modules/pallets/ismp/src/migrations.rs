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

//! Multi-block migrations for pallet-ismp legacy storage cleanup.

use crate::{
	child_trie::{CHILD_TRIE_PREFIX, STATE_COMMITMENTS_KEY},
	Config, StateCommitments, StateMachineUpdateTime,
};
use frame_support::{
	migrations::{SteppedMigration, SteppedMigrationError},
	weights::{Weight, WeightMeter},
};
use polkadot_sdk::*;
use sp_core::storage::ChildInfo;

/// Number of entries to clear per step from each legacy map.
const CLEAR_BATCH_SIZE: u32 = 20_000;

/// Conservative weight per single storage removal (read + write + trie update).
const WEIGHT_PER_REMOVAL: Weight = Weight::from_parts(20_000, 0);

/// Drains legacy [`StateCommitments`], [`StateMachineUpdateTime`], and
/// child trie state commitment entries using bulk `clear()` operations.
pub struct DrainLegacyStateCommitments<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> SteppedMigration for DrainLegacyStateCommitments<T> {
	type Cursor = ();
	type Identifier = u8;

	fn id() -> Self::Identifier {
		2
	}

	fn max_steps() -> Option<u32> {
		None
	}

	fn step(
		_cursor: Option<Self::Cursor>,
		meter: &mut WeightMeter,
	) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
		let step_weight = WEIGHT_PER_REMOVAL.saturating_mul(CLEAR_BATCH_SIZE as u64 * 3);
		if meter.remaining().any_lt(step_weight) {
			return Err(SteppedMigrationError::InsufficientWeight { required: step_weight });
		}

		let mut did_work = false;

		// Bulk-clear StateCommitments
		let sc_result = StateCommitments::<T>::clear(CLEAR_BATCH_SIZE, None);
		meter.consume(WEIGHT_PER_REMOVAL.saturating_mul(sc_result.unique as u64));
		if sc_result.unique > 0 || sc_result.maybe_cursor.is_some() {
			did_work = true;
		}

		// Bulk-clear StateMachineUpdateTime
		let smu_result = StateMachineUpdateTime::<T>::clear(CLEAR_BATCH_SIZE, None);
		meter.consume(WEIGHT_PER_REMOVAL.saturating_mul(smu_result.unique as u64));
		if smu_result.unique > 0 || smu_result.maybe_cursor.is_some() {
			did_work = true;
		}

		// Drain child trie entries in batch
		let child_info = ChildInfo::new_default(CHILD_TRIE_PREFIX);
		let mut child_removed = 0u32;
		for _ in 0..CLEAR_BATCH_SIZE {
			if let Some(key) = sp_io::default_child_storage::next_key(
				child_info.storage_key(),
				STATE_COMMITMENTS_KEY,
			) {
				if key.starts_with(STATE_COMMITMENTS_KEY) {
					sp_io::default_child_storage::clear(child_info.storage_key(), &key);
					child_removed += 1;
					did_work = true;
				} else {
					break;
				}
			} else {
				break;
			}
		}
		meter.consume(WEIGHT_PER_REMOVAL.saturating_mul(child_removed as u64));

		if did_work {
			Ok(Some(()))
		} else {
			Ok(None)
		}
	}
}
