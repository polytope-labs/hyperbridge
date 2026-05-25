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
pub use storage_v1::*;

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
				// Old V0 stored `()`; V1 stored `u64` slot duration. Default migrated
				// chains to 12s slot duration. Note: the value type is now `()` again
				// (see V2 migration), so this closure is only meaningful when running
				// V0 → V1 historically; the resulting `u64` will be dropped in V2.
				Parachains::<T>::translate(|_key: u32, _old_value: ()| Some(()));
				log::info!(target: "ismp_parachain", "Migrated Parachain storage on {} keys", storage_count);
				StorageVersion::new(1).put::<Pallet<T>>();
				Weight::from_all(storage_count)
			} else {
				Weight::zero()
			};
		}
	}
}

/// V1 to V2 migration: drop the per-parachain `slot_duration` value now that the
/// timestamp is read from the `ISMP_TIMESTAMP_ID` consensus digest.
pub mod storage_v1 {
	use super::*;
	use frame_support::{
		pallet_prelude::{GetStorageVersion, StorageVersion},
		weights::Weight,
	};

	/// Storage V1 migration helper.
	pub struct StorageV1 {}

	impl StorageV1 {
		/// Migrate storage from V1 to V2.
		pub fn migrate_to_v2<T: Config>() -> Weight {
			return if Pallet::<T>::on_chain_storage_version() == 1 {
				let storage_count = Parachains::<T>::iter_keys().count() as u64;
				Parachains::<T>::translate(|_key: u32, _old_value: u64| Some(()));
				log::info!(target: "ismp_parachain", "Migrated Parachain storage (drop slot_duration) on {} keys", storage_count);
				StorageVersion::new(2).put::<Pallet<T>>();
				Weight::from_all(storage_count)
			} else {
				Weight::zero()
			};
		}
	}
}
