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

//! Storage migrations for `pallet-collator-manager`.

use crate::{Config, Pallet};
use core::marker::PhantomData;
use polkadot_sdk::*;

use frame_support::{
	migrations::VersionedMigration,
	pallet_prelude::*,
	storage_alias,
	traits::{LockIdentifier, LockableCurrency, ReservableCurrency, UncheckedOnRuntimeUpgrade},
};

/// Lock identifier the pallet used for collator bonds before they moved to collator-selection
/// reserves. Matches the runtime's old `CollatorBondLockId`.
pub const COLLATOR_BOND_LOCK_ID: LockIdentifier = *b"collbond";

/// The legacy bond ledger, kept here only so the migration can drain it.
#[storage_alias]
pub type Bonded<T: Config> = StorageMap<
	Pallet<T>,
	Blake2_128Concat,
	<T as frame_system::Config>::AccountId,
	<T as Config>::Balance,
	ValueQuery,
>;

mod version_unchecked {
	use super::*;

	pub struct MigrateBondsToReserves<T>(PhantomData<T>);

	impl<T: Config> UncheckedOnRuntimeUpgrade for MigrateBondsToReserves<T> {
		fn on_runtime_upgrade() -> Weight {
			let mut migrated = 0u64;
			// Draining the ledger both processes and clears it, so nothing is left behind.
			for (who, bond) in Bonded::<T>::drain() {
				T::NativeCurrency::remove_lock(COLLATOR_BOND_LOCK_ID, &who);
				if let Err(err) = T::NativeCurrency::reserve(&who, bond) {
					log::warn!(
						target: "pallet-collator-manager",
						"bond migration could not reserve {bond:?} for {who:?}: {err:?}",
					);
				}
				migrated = migrated.saturating_add(1);
			}

			<T as frame_system::Config>::DbWeight::get()
				.reads_writes(migrated, migrated.saturating_mul(3))
		}
	}
}

/// Moves every collator bond from the old `collbond` lock into a collator-selection reserve and
/// drains the legacy ledger. Wrapped in `VersionedMigration` so it runs once, on the v0 to v1
/// upgrade, rather than on every runtime upgrade.
pub type MigrateBondsToReserves<T> = VersionedMigration<
	0,
	1,
	version_unchecked::MigrateBondsToReserves<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
