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

//! Storage migrations for `pallet-intents-coprocessor`.

use crate::{Config, Pallet};
use core::marker::PhantomData;
use polkadot_sdk::*;

use frame_support::{
	migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade, weights::Weight,
};

mod v1 {
	use super::*;
	use frame_support::traits::Get;

	/// Clears the old `CurrentPhantomOrder` value. It used to hold one entry per token pair, and
	/// the leading length prefix means the old bytes can still decode under the new single-order
	/// type, just into a meaningless commitment. Clearing it lets the next generation write the
	/// only value that matters.
	pub struct ClearLegacyPhantomOrder<T>(PhantomData<T>);

	impl<T: Config> UncheckedOnRuntimeUpgrade for ClearLegacyPhantomOrder<T> {
		fn on_runtime_upgrade() -> Weight {
			crate::CurrentPhantomOrder::<T>::kill();

			log::info!(
				target: "runtime::intents-coprocessor",
				"ClearLegacyPhantomOrder: cleared the per-pair phantom order batch",
			);

			T::DbWeight::get().writes(1)
		}
	}
}

/// Migration that clears the old per-pair `CurrentPhantomOrder` batch (v0 → v1).
///
/// Every configured pair now rides in a single phantom order, so the storage item holds one
/// commitment instead of a list of them.
pub type ClearLegacyPhantomOrder<T> = VersionedMigration<
	0,
	1,
	v1::ClearLegacyPhantomOrder<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
