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

//! Storage migrations for `pallet-beefy-consensus-proofs`.

use crate::{Config, Pallet};
use core::marker::PhantomData;
use polkadot_sdk::*;

use frame_support::{
	migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade, weights::Weight,
};

mod version_unchecked {
	use super::*;
	use frame_support::traits::Get;

	/// Clears the old `Sp1VkeyHash` storage which was previously stored as `Vec<u8>`
	/// (ASCII hex). After this migration, the vkey must be re-set via `set_sp1_vkey_hash`
	/// using the new `H256` type.
	pub struct ClearSp1VkeyHash<T>(PhantomData<T>);

	impl<T: Config> UncheckedOnRuntimeUpgrade for ClearSp1VkeyHash<T> {
		fn on_runtime_upgrade() -> Weight {
			crate::Sp1VkeyHash::<T>::kill();

			log::info!(
				target: "pallet-beefy-consensus-proofs",
				"ClearSp1VkeyHash: cleared old Vec<u8> vkey storage; re-set via set_sp1_vkey_hash",
			);

			T::DbWeight::get().writes(1)
		}
	}
}

/// Migration that clears the old `Sp1VkeyHash` storage (v0 → v1).
///
/// The storage type changed from `Vec<u8>` (ASCII hex string) to `H256`.
/// Rather than transforming the value, we clear it so it must be re-set
/// via `set_sp1_vkey_hash` with the new type.
pub type ClearSp1VkeyHash<T> = VersionedMigration<
	0,
	1,
	version_unchecked::ClearSp1VkeyHash<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
