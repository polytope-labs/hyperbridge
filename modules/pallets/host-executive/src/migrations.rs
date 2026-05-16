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

//! Storage migrations for `pallet-ismp-host-executive`.
//!
//! The slimming of `HostParam` (the `SubstrateHostParam` variant was dropped)
//! changed the SCALE encoding of the [`HostParams`] storage map's value type.
//! Old chain state can no longer be decoded, so this migration simply clears
//! every entry. Governance is expected to re-populate the map post-upgrade.

use core::marker::PhantomData;
use polkadot_sdk::{
	frame_support::{
		pallet_prelude::Weight,
		traits::{Get, OnRuntimeUpgrade, StorageVersion},
	},
	*,
};

use crate::{Config, HostParams, Pallet};

/// One-shot drain of the legacy [`HostParams`] storage. Reads every key and
/// removes the corresponding entry. Safe to run repeatedly — once the map is
/// empty subsequent invocations are cheap.
pub struct ClearLegacyHostParams<T>(PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for ClearLegacyHostParams<T> {
	fn on_runtime_upgrade() -> Weight {
		let current =
			<Pallet<T> as frame_support::traits::GetStorageVersion>::on_chain_storage_version();
		// Only run when migrating from v1 (or unset). After running, bump to v2.
		if current >= StorageVersion::new(2) {
			return <T as frame_system::Config>::DbWeight::get().reads(1);
		}

		// `clear` removes every entry under the [`HostParams`] prefix.
		// Passing `u32::MAX` plus a `None` cursor wipes the whole map in a
		// single call. The encoded values are unreadable, so we don't try
		// to migrate them — governance will repopulate the map afterwards.
		let result = HostParams::<T>::clear(u32::MAX, None);

		StorageVersion::new(2).put::<Pallet<T>>();

		<T as frame_system::Config>::DbWeight::get()
			.reads_writes(result.unique.into(), result.unique.saturating_add(1).into())
	}
}
