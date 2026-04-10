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

//! Storage migrations for `ismp-parachain`.

use super::*;
use log;
use polkadot_sdk::*;

pub use storage_v0::*;
pub use storage_v1::Migration as MigrationV2;

/// v0 → v1 migration of the `Parachains` storage map.
pub mod storage_v0 {
	use super::*;
	use frame_support::{
		pallet_prelude::{GetStorageVersion, StorageVersion},
		weights::Weight,
	};

	/// v0 → v1 migration runner.
	pub struct StorageV0 {}

	impl StorageV0 {
		/// Translates the legacy unit-valued `Parachains` map to the new `u64`
		/// slot-duration value and bumps the on-chain storage version.
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

/// v1 → v2 multi-block drain of `RelayChainStateCommitments`.
pub mod storage_v1 {
	use super::*;
	use alloc::vec;
	use core::marker::PhantomData;
	use frame_support::{
		migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
		pallet_prelude::StorageVersion,
		traits::ConstU32,
		weights::WeightMeter,
		BoundedVec,
	};
	use pallet_migrations;

	/// Maximum length of the in-band `SteppedMigration::Cursor`. The migration uses
	/// the cursor only as a sentinel "more work to do" flag (1 byte), so this can be
	/// tiny, it must be strictly smaller than
	/// `pallet_migrations::Config::CursorMaxLen` so the `MaxEncodedLen` integrity
	/// check at runtime upgrade time passes.
	pub type CursorBound = ConstU32<8>;

	#[cfg(feature = "try-runtime")]
	use alloc::vec::Vec;
	#[cfg(feature = "try-runtime")]
	use frame_support::pallet_prelude::GetStorageVersion;

	const PALLET_MIGRATIONS_ID: &[u8; 14] = b"IsmpParachainV";

	/// v1 → v2: drains the historical `RelayChainStateCommitments` backlog one entry at
	/// a time using the standard `pallet_migrations::SteppedMigration` interface, so the
	/// (potentially millions of) historical relay-chain state roots are removed across
	/// many blocks rather than charged to a single runtime upgrade.
	///
	/// After the drain completes, `on_finalize`'s steady-state eviction takes over and
	/// keeps the cache bounded at `MAX_RELAY_STATE_COMMITMENTS`.
	pub struct Migration<T: Config + pallet_migrations::Config>(PhantomData<T>);

	impl<T: Config + pallet_migrations::Config> SteppedMigration for Migration<T> {
		type Cursor = BoundedVec<u8, CursorBound>;
		type Identifier = MigrationId<14>;

		fn id() -> Self::Identifier {
			MigrationId { pallet_id: *PALLET_MIGRATIONS_ID, version_from: 1, version_to: 2 }
		}

		fn step(
			_cursor: Option<Self::Cursor>,
			meter: &mut WeightMeter,
		) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
			let weight_per_step =
				<T as crate::pallet::Config>::WeightInfo::migrate_relay_state_commitments_step();
			if meter.remaining().any_lt(weight_per_step) {
				log::info!(
					target: "ismp_parachain",
					"v1 → v2 step: insufficient weight (need {:?}, have {:?})",
					weight_per_step, meter.remaining(),
				);
				return Err(SteppedMigrationError::InsufficientWeight {
					required: weight_per_step,
				});
			}

			// Always remove the first live key. Removal naturally advances the
			// iterator on the next call, so the in-band `cursor` is unused, we
			// just return a sentinel `Some(_)` to signal "more work to do" and
			// `None` to signal completion. Doing the deletion this way avoids
			// relying on `clear(1, _)` honouring the limit (which it doesn't in
			// `TestExternalities` — see frame_support `unhashed::clear_prefix`).
			let next_key = crate::pallet::RelayChainStateCommitments::<T>::iter_keys().next();
			meter.consume(weight_per_step);

			match next_key {
				Some(key) => {
					log::info!(
						target: "ismp_parachain",
						"v1 → v2 step: removing key {key} (count before: {})",
						crate::pallet::RelayChainStateCommitments::<T>::count(),
					);
					crate::pallet::RelayChainStateCommitments::<T>::remove(key);
					// Sentinel non-empty cursor, value doesn't matter, only `Some` vs
					// `None` does. The bound trivially fits.
					let sentinel: Self::Cursor = BoundedVec::try_from(vec![0u8])
						.expect("1-byte vec fits CursorMaxLen >= 1");
					Ok(Some(sentinel))
				},
				None => {
					log::info!(
						target: "ismp_parachain",
						"v1 → v2 step: iter_keys returned None — drain complete"
					);
					// Drain finished. Reset the eviction pointer; `on_finalize` will
					// reinitialize it lazily on the next insert.
					crate::pallet::OldestRetainedRelayBlock::<T>::kill();
					StorageVersion::new(2).put::<crate::pallet::Pallet<T>>();
					Ok(None)
				},
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, frame_support::sp_runtime::TryRuntimeError> {
			log::info!(target: "ismp_parachain", "ismp-parachain v1 → v2 pre_upgrade check");
			assert_eq!(
				StorageVersion::get::<crate::pallet::Pallet<T>>(),
				1,
				"Expected on-chain storage version 1"
			);
			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			_state: Vec<u8>,
		) -> Result<(), frame_support::sp_runtime::TryRuntimeError> {
			log::info!(target: "ismp_parachain", "ismp-parachain v1 → v2 post_upgrade check");
			assert_eq!(
				StorageVersion::get::<crate::pallet::Pallet<T>>(),
				2,
				"Expected on-chain storage version 2"
			);
			Ok(())
		}
	}
}
