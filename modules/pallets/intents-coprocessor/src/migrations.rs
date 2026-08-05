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

	/// Clears the phantom state the single-order model obsoleted. `CurrentPhantomOrder` used to
	/// hold one entry per token pair, and the leading length prefix means the old bytes can still
	/// decode under the new single-order type, just into a meaningless commitment.
	/// `PhantomOrderConfig` predates `PhantomTokenPair::standard_amount_b`, so the stored value no
	/// longer decodes and governance must re-set it; `LastPhantomGeneration` goes with it so the
	/// re-set config generates immediately.
	pub struct ClearLegacyPhantomState<T>(PhantomData<T>);

	impl<T: Config> UncheckedOnRuntimeUpgrade for ClearLegacyPhantomState<T> {
		fn on_runtime_upgrade() -> Weight {
			crate::CurrentPhantomOrder::<T>::kill();
			crate::PhantomOrderConfig::<T>::kill();
			crate::LastPhantomGeneration::<T>::kill();

			log::info!(
				target: "runtime::intents-coprocessor",
				"ClearLegacyPhantomState: cleared the per-pair phantom order batch and pre-reverse-leg config",
			);

			T::DbWeight::get().writes(3)
		}
	}
}

/// Migration that clears the pre-single-order phantom state (v0 → v1): the per-pair
/// `CurrentPhantomOrder` batch, plus the `PhantomOrderConfig` whose pair type gained
/// `standard_amount_b` and must be re-set by governance.
pub type ClearLegacyPhantomState<T> = VersionedMigration<
	0,
	1,
	v1::ClearLegacyPhantomState<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
