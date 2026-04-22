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

//! Storage migrations for `pallet-ismp-optimism`.
//!
//! Follows the FRAME storage-migration pattern documented at
//! <https://docs.polkadot.com/parachains/runtime-maintenance/storage-migrations/>:
//! the actual migration logic lives in a private [`UncheckedOnRuntimeUpgrade`]
//! implementation and is exposed wrapped in a [`VersionedMigration`], which
//! handles on-chain storage-version gating and bumping automatically.

use crate::{
	pallet::{Config, StateMachinesDisputeGameFactoriesTypes},
	Pallet,
};
use alloc::{vec, vec::Vec};
use core::marker::PhantomData;
use hex_literal::hex;
use ismp::host::StateMachine;
use op_verifier::{DisputeGameImpl, GameTypeConfig};
use polkadot_sdk::*;
use primitive_types::H160;

use frame_support::{
	migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade, weights::Weight,
};

/// CANNON fault-proof implementation (`gameType = 0`) — shared across Optimism Superchain
/// deployments at the version pinned by the op-verifier.
const CANNON_IMPL: H160 = H160(hex!("6dDBa09bc4cCB0D6Ca9Fc5350580f74165707499"));
/// PERMISSIONED fault-proof implementation (`gameType = 1`).
const PERMISSIONED_IMPL: H160 = H160(hex!("58bf355C5d4EdFc723eF89d99582ECCfd143266A"));
/// Base multiproof AggregateVerifier (`gameType = 621`), Sepolia only at time of writing.
const AGGREGATE_VERIFIER_IMPL: H160 = H160(hex!("498313fB340CD5055c5568546364008299A47517"));

const CANNON_GAME_TYPE: u32 = 0;
const PERMISSIONED_GAME_TYPE: u32 = 1;
const AGGREGATE_VERIFIER_GAME_TYPE: u32 = 621;

// EVM chain IDs of the L2s supported by this migration.
const OPTIMISM_MAINNET: u32 = 10;
const OPTIMISM_SEPOLIA: u32 = 11155420;
const BASE_MAINNET: u32 = 8453;
const BASE_SEPOLIA: u32 = 84532;
const UNICHAIN_MAINNET: u32 = 130;
const SONEIUM_MAINNET: u32 = 1868;

/// Per-state-machine verification configuration that this migration installs. Returns `None`
/// when the state machine is not one of the L2s supported by the op-verifier, so the caller
/// knows to drop the entry rather than leave it with decoded-but-empty configs.
fn configs_for(state_machine: &StateMachine) -> Option<Vec<GameTypeConfig>> {
	let chain_id = match state_machine {
		StateMachine::Evm(id) => *id,
		_ => return None,
	};

	let mut configs = match chain_id {
		OPTIMISM_MAINNET | OPTIMISM_SEPOLIA | BASE_MAINNET | BASE_SEPOLIA | UNICHAIN_MAINNET |
		SONEIUM_MAINNET => vec![
			GameTypeConfig {
				game_type: CANNON_GAME_TYPE,
				expected_impl: CANNON_IMPL,
				kind: DisputeGameImpl::FaultDisputeGame,
			},
			GameTypeConfig {
				game_type: PERMISSIONED_GAME_TYPE,
				expected_impl: PERMISSIONED_IMPL,
				kind: DisputeGameImpl::FaultDisputeGame,
			},
		],
		_ => return None,
	};

	// AggregateVerifier is only deployed on Base Sepolia at the time of writing.
	if chain_id == BASE_SEPOLIA {
		configs.push(GameTypeConfig {
			game_type: AGGREGATE_VERIFIER_GAME_TYPE,
			expected_impl: AGGREGATE_VERIFIER_IMPL,
			kind: DisputeGameImpl::AggregateVerifier,
		});
	}

	Some(configs)
}

/// Private module holding the **version-unchecked** dispute-game-configs seeding migration.
///
/// Kept private so the unversioned migration cannot accidentally be added to a runtime's
/// `Migrations` tuple without storage-version gating.
mod version_unchecked {
	use super::*;
	use frame_support::traits::Get;

	/// Version-unchecked body of the [`super::SeedDisputeGameConfigs`] migration (v0 → v1).
	///
	/// Translates every entry of [`StateMachinesDisputeGameFactoriesTypes`] from the legacy
	/// `(H160, Vec<u32>)` value layout (factory address + bare game-type numbers) to the new
	/// `(H160, Vec<GameTypeConfig>)` layout that carries each game type's expected
	/// implementation address and storage-layout kind.
	///
	/// For the six L2s supported by the op-verifier — Optimism mainnet + Sepolia, Base
	/// mainnet + Sepolia, Unichain mainnet, and Soneium mainnet — the new configs are
	/// populated from the canonical Superchain deployment addresses pinned as module
	/// constants. Base Sepolia also receives the AggregateVerifier (game type 621) entry.
	///
	/// Any other entries are dropped: they cannot be translated in a meaningful way without
	/// known implementation addresses, and preserving them with empty configs would make
	/// verification silently fail. The factory address already in storage is preserved.
	pub struct InnerSeedDisputeGameConfigs<T>(PhantomData<T>);

	impl<T: Config> UncheckedOnRuntimeUpgrade for InnerSeedDisputeGameConfigs<T> {
		fn on_runtime_upgrade() -> Weight {
			let mut translated: u64 = 0;
			let mut dropped: u64 = 0;

			StateMachinesDisputeGameFactoriesTypes::<T>::translate::<(H160, Vec<u32>), _>(
				|state_machine_id, (factory, _old_game_types)| {
					match configs_for(&state_machine_id.state_id) {
						Some(configs) => {
							translated = translated.saturating_add(1);
							Some((factory, configs))
						},
						None => {
							dropped = dropped.saturating_add(1);
							None
						},
					}
				},
			);

			log::info!(
				target: "pallet-ismp-optimism",
				"SeedDisputeGameConfigs migration: translated {} entries, dropped {} unsupported entries",
				translated,
				dropped,
			);

			let touched = translated.saturating_add(dropped);
			T::DbWeight::get().reads_writes(touched, touched)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<sp_std::prelude::Vec<u8>, sp_runtime::TryRuntimeError> {
			use codec::Encode;

			// `iter_keys` walks the storage without decoding values, which is safe across the
			// value-type change this migration performs.
			let total = StateMachinesDisputeGameFactoriesTypes::<T>::iter_keys().count() as u64;
			log::info!(
				target: "pallet-ismp-optimism",
				"SeedDisputeGameConfigs pre_upgrade: entries={}",
				total,
			);
			Ok(total.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			_pre_state: sp_std::prelude::Vec<u8>,
		) -> Result<(), sp_runtime::TryRuntimeError> {
			use sp_runtime::TryRuntimeError;

			// Every remaining entry must decode under the new layout, and its configs must
			// match what `configs_for` would produce for that state machine.
			for (state_machine_id, (_factory, configs)) in
				StateMachinesDisputeGameFactoriesTypes::<T>::iter()
			{
				let expected = configs_for(&state_machine_id.state_id).ok_or_else(|| {
					TryRuntimeError::Other(
						"Entry survived migration for an unsupported state machine",
					)
				})?;
				if configs != expected {
					return Err(TryRuntimeError::Other(
						"Post-upgrade configs do not match expected seed values",
					));
				}
			}

			Ok(())
		}
	}
}

/// Seeds per-game-type verification configs in `StateMachinesDisputeGameFactoriesTypes`.
///
/// Wraps [`version_unchecked::InnerSeedDisputeGameConfigs`] in [`VersionedMigration`] so that
/// it runs **exactly once**: only when the pallet's on-chain storage version equals `0`, after
/// which the version is bumped to `1` and subsequent runtime upgrades short-circuit it
/// automatically.
pub type SeedDisputeGameConfigs<T> = VersionedMigration<
	0,
	1,
	version_unchecked::InnerSeedDisputeGameConfigs<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
