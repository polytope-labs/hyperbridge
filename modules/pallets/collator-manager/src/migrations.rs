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
	pallet_prelude::*,
	storage_alias,
	traits::{LockIdentifier, LockableCurrency, OnRuntimeUpgrade, ReservableCurrency},
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

/// Moves every collator bond from the old pallet lock to a collator-selection reserve.
///
/// Collator-selection now holds the candidacy bond as a plain reserve, so for each bonded
/// account we drop the `collbond` lock and reserve the same amount, then clear the legacy
/// ledger. Draining the ledger makes it safe to run the migration more than once.
pub struct MigrateBondsToReserves<T>(PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for MigrateBondsToReserves<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut migrated = 0u64;
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
