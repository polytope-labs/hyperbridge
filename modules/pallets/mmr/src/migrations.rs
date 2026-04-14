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

//! Storage migrations for `pallet-mmr-tree`.
//!
//! Follows the pattern documented at
//! <https://docs.polkadot.com/parachains/runtime-maintenance/storage-migrations/>:
//! the actual migration logic lives in a private [`UncheckedOnRuntimeUpgrade`]
//! implementation and is exposed wrapped in a [`VersionedMigration`], which
//! handles on-chain storage-version gating and bumping automatically.

use crate::{Config, HashOf, InitialHeight, Nodes, NumberOfLeaves, Pallet, RootHash};
use core::marker::PhantomData;
use polkadot_sdk::*;

use frame_support::{
	migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade, weights::Weight,
};
use sp_core::H256;

/// Private module holding the **version-unchecked** reset logic.
///
/// This is kept private and only reachable through the [`ResetMmrTree`] alias
/// wrapped in [`VersionedMigration`], so it cannot accidentally be added to a
/// runtime's `Migrations` tuple without storage-version gating.
mod version_unchecked {
	use super::*;
	use frame_support::traits::Get;

	/// Version-unchecked body of the `ResetMmrTree` migration (v0 → v1).
	///
	/// Clears the pallet's on-chain MMR state so that new leaves accumulate
	/// under the new [`Config::INDEXING_PREFIX`] from scratch:
	/// - Kills [`NumberOfLeaves`], [`RootHash`], [`InitialHeight`].
	/// - Clears every entry in [`Nodes`] (bounded above by `Nodes::count()`,
	///   since it is a `CountedStorageMap`).
	///
	/// [`crate::IntermediateLeaves`] is intentionally **not** touched: it is
	/// only ever populated transiently within a single block's execution, so it
	/// is already empty by the time any subsequent block's `on_runtime_upgrade`
	/// runs.
	///
	/// The legacy canonical entries previously written under the old offchain
	/// prefix are also not touched — they become orphaned keys in each node's
	/// local offchain DB that nothing reads (the pallet and the `mmr-gadget`
	/// both use the new prefix after this migration), and are harmless.
	pub struct InnerResetMmrTree<T, I = ()>(PhantomData<(T, I)>);

	impl<T, I> UncheckedOnRuntimeUpgrade for InnerResetMmrTree<T, I>
	where
		T: Config<I>,
		I: 'static,
		HashOf<T, I>: Into<H256>,
	{
		fn on_runtime_upgrade() -> Weight {
			// `Nodes` is a `CountedStorageMap`, so `count()` is the exact number
			// of entries to scan — no overshoot.
			let nodes_count = Nodes::<T, I>::count();

			NumberOfLeaves::<T, I>::kill();
			RootHash::<T, I>::kill();
			InitialHeight::<T, I>::kill();
			let _ = Nodes::<T, I>::clear(nodes_count, None);

			log::info!(
				target: "pallet-mmr",
				"ResetMmrTree migration: cleared {} on-chain MMR nodes; new leaves will accumulate under the new offchain prefix",
				nodes_count,
			);

			// 1 read for `Nodes::count()`, 3 kills + `nodes_count` removals.
			T::DbWeight::get().reads_writes(1, 3u64 + u64::from(nodes_count))
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<sp_std::prelude::Vec<u8>, sp_runtime::TryRuntimeError> {
			use codec::Encode;

			let nodes_count = Nodes::<T, I>::count();
			let leaves = NumberOfLeaves::<T, I>::get();
			log::info!(
				target: "pallet-mmr",
				"ResetMmrTree pre_upgrade: {} leaves, {} nodes on-chain",
				leaves,
				nodes_count,
			);
			Ok(nodes_count.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			_pre_state: sp_std::prelude::Vec<u8>,
		) -> Result<(), sp_runtime::TryRuntimeError> {
			use sp_runtime::TryRuntimeError;

			if NumberOfLeaves::<T, I>::get() != 0 {
				return Err(TryRuntimeError::Other("NumberOfLeaves not cleared"));
			}
			if RootHash::<T, I>::exists() {
				return Err(TryRuntimeError::Other("RootHash not cleared"));
			}
			if InitialHeight::<T, I>::exists() {
				return Err(TryRuntimeError::Other("InitialHeight not cleared"));
			}
			if Nodes::<T, I>::count() != 0 {
				return Err(TryRuntimeError::Other("Nodes not cleared"));
			}
			Ok(())
		}
	}
}

/// Migration that resets all on-chain MMR state so new leaves accumulate under
/// the new [`Config::INDEXING_PREFIX`] from scratch.
///
/// Wraps [`version_unchecked::InnerResetMmrTree`] in [`VersionedMigration`] so
/// that it runs **exactly once**: only when the pallet's on-chain storage
/// version equals `0`, after which the version is bumped to `1` and subsequent
/// runtime upgrades short-circuit it automatically.
pub type ResetMmrTree<T, I = ()> = VersionedMigration<
	0,
	1,
	version_unchecked::InnerResetMmrTree<T, I>,
	Pallet<T, I>,
	<T as frame_system::Config>::DbWeight,
>;
