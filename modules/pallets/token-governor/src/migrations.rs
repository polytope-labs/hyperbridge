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

//! Storage migrations for `pallet-token-governor`.
//!
//! Follows the FRAME storage-migration pattern documented at
//! <https://docs.polkadot.com/parachains/runtime-maintenance/storage-migrations/>:
//! the actual migration logic lives in a private [`UncheckedOnRuntimeUpgrade`]
//! implementation and is exposed wrapped in a [`VersionedMigration`], which
//! handles on-chain storage-version gating and bumping automatically.

use crate::{Config, Pallet, StandaloneChainAssets, SupportedChains, TokenGatewayParams};
use core::marker::PhantomData;
use polkadot_sdk::*;

use frame_support::{
	migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade, weights::Weight,
};

/// Private module holding the **version-unchecked** TokenGateway-state reset.
///
/// Kept private so the unversioned migration cannot accidentally be added to a
/// runtime's `Migrations` tuple without storage-version gating.
mod version_unchecked {
	use super::*;
	use frame_support::traits::Get;

	/// Version-unchecked body of the [`super::ResetTokenGatewayState`] migration
	/// (v0 → v1).
	///
	/// Wipes every TokenGateway-related configuration tracked by this pallet:
	/// - [`TokenGatewayParams`] — per-chain TokenGateway protocol parameters.
	/// - [`SupportedChains`] — the asset → chain deployment matrix that the
	///   token-governor uses to route TokenGateway operations.
	/// - [`StandaloneChainAssets`] — the standalone-chain native-asset registry
	///   consulted by `pallet-token-gateway-inspector` when validating
	///   TokenGateway requests.
	///
	/// All other token-governor storage items (`PendingAsset`, `AssetMetadatas`,
	/// `AssetOwners`, `ProtocolParams`, `TokenRegistrarParams`,
	/// `IntentGatewayParams`) are intentionally **not** touched: they are not
	/// TokenGateway-specific configuration.
	pub struct InnerResetTokenGatewayState<T>(PhantomData<T>);

	impl<T: Config> UncheckedOnRuntimeUpgrade for InnerResetTokenGatewayState<T> {
		fn on_runtime_upgrade() -> Weight {
			let token_gateway_params = TokenGatewayParams::<T>::clear(u32::MAX, None);
			let supported_chains = SupportedChains::<T>::clear(u32::MAX, None);
			let standalone_chain_assets = StandaloneChainAssets::<T>::clear(u32::MAX, None);

			let cleared = token_gateway_params.unique as u64
				+ supported_chains.unique as u64
				+ standalone_chain_assets.unique as u64;

			log::info!(
				target: "pallet-token-governor",
				"ResetTokenGatewayState migration: cleared {} TokenGateway-related entries (TokenGatewayParams + SupportedChains + StandaloneChainAssets)",
				cleared,
			);

			// Each `clear` call performs `cleared`-many removals (1 read + 1 write each).
			T::DbWeight::get().reads_writes(cleared, cleared)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<sp_std::prelude::Vec<u8>, sp_runtime::TryRuntimeError> {
			use codec::Encode;

			let token_gateway_params = TokenGatewayParams::<T>::iter().count() as u64;
			let supported_chains = SupportedChains::<T>::iter().count() as u64;
			let standalone_chain_assets = StandaloneChainAssets::<T>::iter().count() as u64;
			log::info!(
				target: "pallet-token-governor",
				"ResetTokenGatewayState pre_upgrade: TokenGatewayParams={}, SupportedChains={}, StandaloneChainAssets={}",
				token_gateway_params,
				supported_chains,
				standalone_chain_assets,
			);
			Ok((token_gateway_params, supported_chains, standalone_chain_assets).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			_pre_state: sp_std::prelude::Vec<u8>,
		) -> Result<(), sp_runtime::TryRuntimeError> {
			use sp_runtime::TryRuntimeError;

			if TokenGatewayParams::<T>::iter().next().is_some() {
				return Err(TryRuntimeError::Other("TokenGatewayParams not cleared"));
			}
			if SupportedChains::<T>::iter().next().is_some() {
				return Err(TryRuntimeError::Other("SupportedChains not cleared"));
			}
			if StandaloneChainAssets::<T>::iter().next().is_some() {
				return Err(TryRuntimeError::Other("StandaloneChainAssets not cleared"));
			}
			Ok(())
		}
	}
}

/// Migration that wipes every TokenGateway-related configuration in
/// `pallet-token-governor`.
///
/// Wraps [`version_unchecked::InnerResetTokenGatewayState`] in
/// [`VersionedMigration`] so that it runs **exactly once**: only when the
/// pallet's on-chain storage version equals `0`, after which the version is
/// bumped to `1` and subsequent runtime upgrades short-circuit it
/// automatically.
pub type ResetTokenGatewayState<T> = VersionedMigration<
	0,
	1,
	version_unchecked::InnerResetTokenGatewayState<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
