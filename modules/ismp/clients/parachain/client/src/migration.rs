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

//! Storage migrations for ismp-parachain.

use super::*;
use log;
use polkadot_sdk::*;

pub use storage_v0::*;

/// V0 to V1 migration: populate `Parachains` slot durations.
pub mod storage_v0 {
	use super::*;
	use frame_support::{
		pallet_prelude::{GetStorageVersion, StorageVersion},
		weights::Weight,
	};

	/// Storage V0 migration helper.
	pub struct StorageV0 {}

	impl StorageV0 {
		/// Migrate storage from V0 to V1.
		pub fn migrate_to_v1<T: Config>() -> Weight {
			return if Pallet::<T>::on_chain_storage_version() == 0 {
				// track reads and write to be made
				let storage_count = Parachains::<T>::iter_keys().count() as u64;
				Parachains::<T>::translate(|_key: u32, _old_value: ()| Some(12_000));
				log::info!(target: "ismp_parachain", "Migrated Parachain storage on {} keys", storage_count);
				StorageVersion::new(1).put::<Pallet<T>>();
				Weight::from_all(storage_count)
			} else {
				Weight::zero()
			};
		}
	}
}

pub use storage_v1::Migration as MigrationV2;

/// Multi-block migration (V2): drain legacy `RelayChainStateCommitments`.
pub mod storage_v1 {
	use super::*;
	use frame_support::{
		migrations::{SteppedMigration, SteppedMigrationError},
		weights::{Weight, WeightMeter},
	};

	/// Number of entries to clear per step.
	const CLEAR_BATCH_SIZE: u32 = 50_000;

	/// Conservative weight per single storage removal.
	const WEIGHT_PER_REMOVAL: Weight = Weight::from_parts(20_000, 0);

	/// Drains the legacy [`RelayChainStateCommitments`] map using bulk `clear()`.
	pub struct Migration<T: Config>(core::marker::PhantomData<T>);

	impl<T: Config> SteppedMigration for Migration<T> {
		/// Sentinel cursor.
		type Cursor = ();
		type Identifier = u8;

		fn id() -> Self::Identifier {
			1
		}

		fn max_steps() -> Option<u32> {
			None
		}

		fn step(
			_cursor: Option<Self::Cursor>,
			meter: &mut WeightMeter,
		) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
			let step_weight = WEIGHT_PER_REMOVAL.saturating_mul(CLEAR_BATCH_SIZE as u64);
			if meter.remaining().any_lt(step_weight) {
				return Err(SteppedMigrationError::InsufficientWeight { required: step_weight });
			}

			let result = RelayChainStateCommitments::<T>::clear(CLEAR_BATCH_SIZE, None);
			meter.consume(WEIGHT_PER_REMOVAL.saturating_mul(result.unique as u64));

			if result.unique > 0 || result.maybe_cursor.is_some() {
				Ok(Some(()))
			} else {
				Ok(None)
			}
		}
	}
}
