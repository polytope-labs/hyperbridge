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
//!
//! Three separate migrations that run sequentially via `pallet_migrations`:
//! 1. [`DrainLegacyStateCommitments`] — clears the legacy `StateCommitments` map
//! 2. [`DrainLegacyStateMachineUpdateTime`] — clears the legacy `StateMachineUpdateTime` map
//! 3. [`DrainLegacyChildTrieStateCommitments`] — clears state commitment entries from the child
//!    trie

use crate::{
	child_trie::{CHILD_TRIE_PREFIX, STATE_COMMITMENTS_KEY},
	weights::MigrationWeightInfo,
	Config, StateCommitments, StateMachineUpdateTime,
};
use frame_support::{
	migrations::{SteppedMigration, SteppedMigrationError},
	weights::WeightMeter,
};
use polkadot_sdk::*;
use sp_core::storage::ChildInfo;

/// Number of entries to clear per step for `StorageMap::clear()` operations.
/// Must fit within MbmServiceWeight (max_block / 2) when passed to
/// the benchmarked weight function.
const CLEAR_BATCH_SIZE: u32 = 1_000;

/// Number of child trie entries to drain per step.
/// Benchmarked at ~520ms per entry; 400 entries fits within MbmServiceWeight.
const CHILD_TRIE_BATCH_SIZE: u32 = 400;

/// Drains the legacy [`StateCommitments`] storage map.
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
		let required = T::MigrationWeightInfo::drain_state_commitments_step(CLEAR_BATCH_SIZE);
		if meter.remaining().any_lt(required) {
			return Err(SteppedMigrationError::InsufficientWeight { required });
		}

		let result = StateCommitments::<T>::clear(CLEAR_BATCH_SIZE, None);
		meter.consume(T::MigrationWeightInfo::drain_state_commitments_step(result.unique));

		if result.unique > 0 || result.maybe_cursor.is_some() {
			Ok(Some(()))
		} else {
			Ok(None)
		}
	}
}

/// Drains the legacy [`StateMachineUpdateTime`] storage map.
pub struct DrainLegacyStateMachineUpdateTime<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> SteppedMigration for DrainLegacyStateMachineUpdateTime<T> {
	type Cursor = ();
	type Identifier = u8;

	fn id() -> Self::Identifier {
		3
	}

	fn max_steps() -> Option<u32> {
		None
	}

	fn step(
		_cursor: Option<Self::Cursor>,
		meter: &mut WeightMeter,
	) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
		let required =
			T::MigrationWeightInfo::drain_state_machine_update_time_step(CLEAR_BATCH_SIZE);
		if meter.remaining().any_lt(required) {
			return Err(SteppedMigrationError::InsufficientWeight { required });
		}

		let result = StateMachineUpdateTime::<T>::clear(CLEAR_BATCH_SIZE, None);
		meter.consume(T::MigrationWeightInfo::drain_state_machine_update_time_step(result.unique));

		if result.unique > 0 || result.maybe_cursor.is_some() {
			Ok(Some(()))
		} else {
			Ok(None)
		}
	}
}

/// Drains state commitment entries from the ISMP child trie.
pub struct DrainLegacyChildTrieStateCommitments<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> SteppedMigration for DrainLegacyChildTrieStateCommitments<T> {
	type Cursor = ();
	type Identifier = u8;

	fn id() -> Self::Identifier {
		4
	}

	fn max_steps() -> Option<u32> {
		None
	}

	fn step(
		_cursor: Option<Self::Cursor>,
		meter: &mut WeightMeter,
	) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
		let required =
			T::MigrationWeightInfo::drain_child_trie_state_commitments_step(CHILD_TRIE_BATCH_SIZE);
		if meter.remaining().any_lt(required) {
			return Err(SteppedMigrationError::InsufficientWeight { required });
		}

		let child_info = ChildInfo::new_default(CHILD_TRIE_PREFIX);
		let mut removed = 0u32;

		for _ in 0..CHILD_TRIE_BATCH_SIZE {
			if let Some(key) = sp_io::default_child_storage::next_key(
				child_info.storage_key(),
				STATE_COMMITMENTS_KEY,
			) {
				if key.starts_with(STATE_COMMITMENTS_KEY) {
					sp_io::default_child_storage::clear(child_info.storage_key(), &key);
					removed += 1;
				} else {
					break;
				}
			} else {
				break;
			}
		}

		meter.consume(T::MigrationWeightInfo::drain_child_trie_state_commitments_step(removed));

		if removed > 0 {
			Ok(Some(()))
		} else {
			Ok(None)
		}
	}
}
