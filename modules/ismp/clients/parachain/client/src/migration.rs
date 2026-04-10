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

use super::*;
use log;
use polkadot_sdk::*;

pub use storage_v0::*;
pub mod storage_v0 {
	use super::*;
	use frame_support::{
		pallet_prelude::{GetStorageVersion, StorageVersion},
		weights::Weight,
	};

	pub struct StorageV0 {}

	impl StorageV0 {
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
