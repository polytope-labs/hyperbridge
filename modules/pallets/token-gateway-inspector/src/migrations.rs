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

//! Storage migrations for `pallet-token-gateway-inspector`.
//!
//! Follows the FRAME storage-migration pattern documented at
//! <https://docs.polkadot.com/parachains/runtime-maintenance/storage-migrations/>:
//! the actual migration logic lives in a private [`UncheckedOnRuntimeUpgrade`]
//! implementation and is exposed wrapped in a [`VersionedMigration`], which
//! handles on-chain storage-version gating and bumping automatically.

use crate::{Config, InflowBalances, Pallet};
use core::marker::PhantomData;
use polkadot_sdk::*;

use frame_support::{
	migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade, weights::Weight,
};

/// Private module holding the **version-unchecked** TokenGateway-inspector
/// state reset.
///
/// Kept private so the unversioned migration cannot accidentally be added to a
/// runtime's `Migrations` tuple without storage-version gating.
mod version_unchecked {
	use super::*;
	use frame_support::traits::Get;

	/// Version-unchecked body of the
	/// [`super::ResetTokenGatewayInspectorState`] migration (v0 → v1).
	///
	/// Wipes [`InflowBalances`], the only TokenGateway-related configuration
	/// tracked by this pallet — the per-(chain, asset) net inflow ledger that
	/// `inspect_request` and `handle_timeout` mutate as TokenGateway requests
	/// flow through the runtime.
	pub struct InnerResetTokenGatewayInspectorState<T>(PhantomData<T>);

	impl<T: Config> UncheckedOnRuntimeUpgrade for InnerResetTokenGatewayInspectorState<T> {
		fn on_runtime_upgrade() -> Weight {
			let result = InflowBalances::<T>::clear(u32::MAX, None);
			let cleared = result.unique as u64;

			log::info!(
				target: "pallet-token-gateway-inspector",
				"ResetTokenGatewayInspectorState migration: cleared {} InflowBalances entries",
				cleared,
			);

			T::DbWeight::get().reads_writes(cleared, cleared)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<sp_std::prelude::Vec<u8>, sp_runtime::TryRuntimeError> {
			use codec::Encode;

			let count = InflowBalances::<T>::iter().count() as u64;
			log::info!(
				target: "pallet-token-gateway-inspector",
				"ResetTokenGatewayInspectorState pre_upgrade: InflowBalances={}",
				count,
			);
			Ok(count.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			_pre_state: sp_std::prelude::Vec<u8>,
		) -> Result<(), sp_runtime::TryRuntimeError> {
			use sp_runtime::TryRuntimeError;

			if InflowBalances::<T>::iter().next().is_some() {
				return Err(TryRuntimeError::Other("InflowBalances not cleared"));
			}
			Ok(())
		}
	}
}

/// Migration that wipes every TokenGateway-related configuration in
/// `pallet-token-gateway-inspector`.
///
/// Wraps [`version_unchecked::InnerResetTokenGatewayInspectorState`] in
/// [`VersionedMigration`] so that it runs **exactly once**: only when the
/// pallet's on-chain storage version equals `0`, after which the version is
/// bumped to `1` and subsequent runtime upgrades short-circuit it
/// automatically.
pub type ResetTokenGatewayInspectorState<T> = VersionedMigration<
	0,
	1,
	version_unchecked::InnerResetTokenGatewayInspectorState<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
