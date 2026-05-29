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

mod version_unchecked_v2 {
	use super::*;
	use pallet_collator_selection::Config as CollatorSelectionConfig;

	pub struct ReserveUnreservedBonds<T>(PhantomData<T>);

	impl<T: Config> UncheckedOnRuntimeUpgrade for ReserveUnreservedBonds<T> {
		fn on_runtime_upgrade() -> Weight {
			let mut fixed = 0u64;

			pallet_collator_selection::CandidateList::<T>::mutate(|list| {
				list.retain(|candidate| {
					let stash = &candidate.who;
					let expected = candidate.deposit;
					let reserved =
						<T as CollatorSelectionConfig>::Currency::reserved_balance(stash);

					if reserved >= expected {
						return true;
					}

					let shortfall = expected - reserved;
					match <T as CollatorSelectionConfig>::Currency::reserve(stash, shortfall) {
						Ok(_) => {
							fixed = fixed.saturating_add(1);
							true
						},
						Err(err) => {
							// Not enough free balance to cover the bond — remove from the candidate
							// set and clear every entry this pallet keyed on the stash or its
							// controller. Leaving an `Unbonding` row behind would wedge the
							// (re-)`unbond` flow if the operator ever rejoins (the `AlreadyUnbonding`
							// guard would fire), and a stale `RemovedValidators` entry would silently
							// keep them off the next selection round even after governance
							// reinstates them.
							log::warn!(
								target: "pallet-collator-manager",
								"removing candidate {stash:?}: reserve of {shortfall:?} failed: {err:?}",
							);
							crate::Unbonding::<T>::remove(stash);
							if let Some(controller) = crate::Controller::<T>::take(stash) {
								crate::Stash::<T>::remove(&controller);
								crate::RemovedValidators::<T>::remove(&controller);
							}
							false
						},
					}
				});
			});

			<T as frame_system::Config>::DbWeight::get().reads_writes(fixed + 1, fixed + 1)
		}
	}
}

/// Reserves the candidacy bond for any collator whose bond was not reserved by
/// `MigrateBondsToReserves` — typically because the account had no free balance above the
/// existential deposit at migration time. Runs once on the v1 to v2 upgrade.
pub type ReserveUnreservedBonds<T> = VersionedMigration<
	1,
	2,
	version_unchecked_v2::ReserveUnreservedBonds<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
