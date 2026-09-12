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

/// The phantom configuration as it stood before v2, when every chain lived in one value, which
/// `MigrateConfigToStorageMap` reads and clears. The map hashes its keys onto the same prefix, so
/// this value sits at a key no map entry ever occupies — reading or clearing it cannot disturb
/// migrated state.
mod legacy {
	use super::*;
	use crate::types::{PhantomTokenPair, MAX_PHANTOM_CHAINS, MAX_PHANTOM_TOKEN_PAIRS};
	use codec::{Decode, Encode};
	use frame_support::{pallet_prelude::OptionQuery, storage_alias, traits::ConstU32, BoundedVec};
	use ismp::consensus::StateMachineId;
	use scale_info::TypeInfo;

	/// The pre-v2 per-chain entry: the chain rode inside the value instead of keying it.
	#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq, Eq)]
	pub struct PhantomChainConfiguration {
		pub chain: StateMachineId,
		pub token_pairs: BoundedVec<PhantomTokenPair, ConstU32<MAX_PHANTOM_TOKEN_PAIRS>>,
	}

	/// The pre-v2 configuration: every chain in one value, sharing a single interval.
	#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq, Eq)]
	pub struct PhantomOrderConfiguration {
		pub chains: BoundedVec<PhantomChainConfiguration, ConstU32<MAX_PHANTOM_CHAINS>>,
		pub interval_blocks: u32,
	}

	#[storage_alias]
	pub type PhantomOrderConfig<T: Config> =
		StorageValue<Pallet<T>, PhantomOrderConfiguration, OptionQuery>;
}

mod v2 {
	use super::*;
	use crate::types::{PhantomTokenPair, MAX_PHANTOM_CHAINS};
	use frame_support::traits::Get;
	use hex_literal::hex;
	use ismp::{consensus::StateMachineId, host::StateMachine};
	use primitive_types::H160;

	/// BNB Chain, whose cNGN markets this migration seeds.
	pub(super) const BSC: StateMachineId =
		StateMachineId { state_id: StateMachine::Evm(56), consensus_state_id: *b"BSC0" };
	const CNGN_BSC: H160 = H160(hex!("a8AEA66B361a8d53e8865c62D142167Af28Af058"));
	const USDC_BSC: H160 = H160(hex!("8ac76a51cc950d9822d68b83fe1ad97b32cd580d"));
	const USDT_BSC: H160 = H160(hex!("55d398326f99059ff775485246999027b3197955"));
	/// One whole cNGN: it keeps its 6 decimals on BNB Chain...
	const CNGN_UNIT: u128 = 1_000_000;
	/// ...while the Binance-pegged stables beside it carry 18, so their unit is not the 1e6 a
	/// 6-decimal USDC would use on other chains.
	const BSC_STABLE_UNIT: u128 = 1_000_000_000_000_000_000;

	/// cNGN/USDC and cNGN/USDT on BNB Chain. Each is probed in both directions, so the two
	/// entries cover all four legs.
	pub(super) fn bsc_cngn_pairs() -> [PhantomTokenPair; 2] {
		[USDC_BSC, USDT_BSC].map(|stable| PhantomTokenPair {
			token_a: CNGN_BSC,
			token_b: stable,
			standard_amount: CNGN_UNIT,
			standard_amount_b: BSC_STABLE_UNIT,
		})
	}

	/// A pair's identity for duplicate detection: both directions ride in the same order, so
	/// the token set is unordered, exactly as `set_phantom_order_config` checks it.
	fn unordered(pair: &PhantomTokenPair) -> (H160, H160) {
		if pair.token_a <= pair.token_b {
			(pair.token_a, pair.token_b)
		} else {
			(pair.token_b, pair.token_a)
		}
	}

	/// Moves the single `PhantomOrderConfiguration` value into the per-chain `PhantomOrderConfig`
	/// map, records the migrated chains in the `PhantomChains` set the hook walks, and lifts the
	/// interval every chain shared into its own `PhantomOrderInterval` value.
	///
	/// Generation is otherwise untouched: the chains, their pairs and their interval all survive,
	/// and `LastPhantomGeneration` keeps the shared cycle running. The legacy value lives at a key
	/// the map never occupies, so it is taken (and thereby cleared) rather than left to linger.
	///
	/// It also seeds BNB Chain's cNGN markets, so those pairs go live with the upgrade instead of
	/// waiting on a follow-up governance call.
	pub struct MigrateConfigToStorageMap<T>(PhantomData<T>);

	impl<T: Config> UncheckedOnRuntimeUpgrade for MigrateConfigToStorageMap<T> {
		fn on_runtime_upgrade() -> Weight {
			let mut chains = crate::PhantomChains::<T>::get();
			let mut writes = 0u64;

			match legacy::PhantomOrderConfig::<T>::take() {
				Some(old) => {
					let mut migrated = 0u64;
					for entry in old.chains.into_iter() {
						// The old value keyed entries by the full StateMachineId, so it could
						// carry one state machine twice under two consensus state ids — a shape
						// the map rejects, since both would generate the same commitment.
						if chains.iter().any(|id| id.state_id == entry.chain.state_id) {
							log::error!(
								target: "runtime::intents-coprocessor",
								"MigrateConfigToStorageMap: dropping {:?}, its state machine is already configured",
								entry.chain,
							);
							continue;
						}
						if chains.try_insert(entry.chain).is_err() {
							log::error!(
								target: "runtime::intents-coprocessor",
								"MigrateConfigToStorageMap: dropping {:?}, more than {} chains configured",
								entry.chain,
								MAX_PHANTOM_CHAINS,
							);
							continue;
						}

						crate::PhantomOrderConfig::<T>::insert(entry.chain, entry.token_pairs);
						migrated += 1;
					}

					crate::PhantomOrderInterval::<T>::put(old.interval_blocks);
					writes += migrated + 1;

					log::info!(
						target: "runtime::intents-coprocessor",
						"MigrateConfigToStorageMap: moved {} chain configurations into the storage map",
						migrated,
					);
				},
				None => log::info!(
					target: "runtime::intents-coprocessor",
					"MigrateConfigToStorageMap: no phantom order configuration to migrate",
				),
			}

			// BNB Chain is added rather than replaced: if it already carries a configuration
			// (under whatever consensus state id it was registered with), only the cNGN pairs it
			// is missing are appended.
			let mut bsc = chains.iter().find(|id| id.state_id == BSC.state_id).copied();
			if bsc.is_none() && chains.try_insert(BSC).is_ok() {
				bsc = Some(BSC);
			}

			match bsc {
				Some(chain) => {
					let mut token_pairs =
						crate::PhantomOrderConfig::<T>::get(chain).unwrap_or_default();
					for pair in bsc_cngn_pairs() {
						if token_pairs
							.iter()
							.any(|configured| unordered(configured) == unordered(&pair))
						{
							continue;
						}
						if token_pairs.try_push(pair).is_err() {
							log::error!(
								target: "runtime::intents-coprocessor",
								"MigrateConfigToStorageMap: {:?} has no room for another token pair",
								chain,
							);
							break;
						}
					}

					crate::PhantomOrderConfig::<T>::insert(chain, token_pairs);
					writes += 1;
				},
				None => log::error!(
					target: "runtime::intents-coprocessor",
					"MigrateConfigToStorageMap: cannot seed {:?}, more than {} chains configured",
					BSC,
					MAX_PHANTOM_CHAINS,
				),
			}

			crate::PhantomChains::<T>::put(chains);

			// The legacy value, the chain set and BNB Chain's entry, plus the interval and a
			// config write per migrated chain.
			T::DbWeight::get().reads_writes(3, writes + 1)
		}
	}
}

/// Migration that moves the phantom order configuration from a single storage value into the
/// per-chain `PhantomOrderConfig` map (v1 → v2), so chains can be configured independently.
pub type MigrateConfigToStorageMap<T> = VersionedMigration<
	1,
	2,
	v2::MigrateConfigToStorageMap<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		tests::{new_test_ext, Test},
		types::PhantomTokenPair,
	};
	use frame_support::BoundedVec;
	use ismp::{consensus::StateMachineId, host::StateMachine};
	use primitive_types::H160;

	fn chain_id(evm_id: u32) -> StateMachineId {
		StateMachineId { state_id: StateMachine::Evm(evm_id), consensus_state_id: *b"ETH0" }
	}

	fn pair(token: u8) -> PhantomTokenPair {
		PhantomTokenPair {
			token_a: H160::repeat_byte(token),
			token_b: H160::repeat_byte(token + 1),
			standard_amount: 1_000_000,
			standard_amount_b: 2_000_000,
		}
	}

	#[test]
	fn migration_moves_every_chain_into_the_storage_map() {
		new_test_ext().execute_with(|| {
			legacy::PhantomOrderConfig::<Test>::put(legacy::PhantomOrderConfiguration {
				chains: BoundedVec::try_from(vec![
					legacy::PhantomChainConfiguration {
						chain: chain_id(1),
						token_pairs: BoundedVec::try_from(vec![pair(1)]).unwrap(),
					},
					legacy::PhantomChainConfiguration {
						chain: chain_id(10),
						token_pairs: BoundedVec::try_from(vec![pair(5), pair(7)]).unwrap(),
					},
				])
				.unwrap(),
				interval_blocks: 200,
			});
			crate::LastPhantomGeneration::<Test>::put(7u64);

			v2::MigrateConfigToStorageMap::<Test>::on_runtime_upgrade();

			// Each chain keeps its own pairs, and the interval they shared is lifted out whole.
			assert_eq!(
				crate::PhantomOrderConfig::<Test>::get(chain_id(1)).unwrap().to_vec(),
				vec![pair(1)]
			);
			assert_eq!(
				crate::PhantomOrderConfig::<Test>::get(chain_id(10)).unwrap().to_vec(),
				vec![pair(5), pair(7)]
			);
			assert_eq!(crate::PhantomOrderInterval::<Test>::get(), 200);

			// The set the hook walks is populated — BNB Chain included, seeded alongside the
			// migrated chains — and the shared cycle carries on untouched.
			let chains = crate::PhantomChains::<Test>::get();
			assert_eq!(
				chains.iter().copied().collect::<Vec<_>>(),
				vec![chain_id(1), chain_id(10), v2::BSC]
			);
			assert_eq!(crate::LastPhantomGeneration::<Test>::get(), Some(7));

			// The legacy value is gone, so a re-run is a no-op rather than a second import.
			assert!(legacy::PhantomOrderConfig::<Test>::get().is_none());
		});
	}

	#[test]
	fn migration_seeds_bnb_chains_cngn_markets() {
		new_test_ext().execute_with(|| {
			v2::MigrateConfigToStorageMap::<Test>::on_runtime_upgrade();

			// cNGN against both Binance-pegged stables, each priced in one whole unit of its
			// side: 6 decimals for cNGN, 18 for the stables.
			let pairs = crate::PhantomOrderConfig::<Test>::get(v2::BSC).unwrap();
			assert_eq!(pairs.to_vec(), v2::bsc_cngn_pairs().to_vec());
			assert!(pairs.iter().all(|pair| pair.standard_amount == 1_000_000 &&
				pair.standard_amount_b == 1_000_000_000_000_000_000));
			assert!(crate::PhantomChains::<Test>::get().contains(&v2::BSC));
		});
	}

	#[test]
	fn migration_appends_only_the_cngn_pairs_bnb_chain_is_missing() {
		new_test_ext().execute_with(|| {
			// BNB Chain already configured, carrying one of the two pairs in reverse — both
			// directions ride in the same order, so that entry must not be duplicated.
			let [usdc, usdt] = v2::bsc_cngn_pairs();
			let reversed = PhantomTokenPair {
				token_a: usdc.token_b,
				token_b: usdc.token_a,
				standard_amount: usdc.standard_amount_b,
				standard_amount_b: usdc.standard_amount,
			};
			legacy::PhantomOrderConfig::<Test>::put(legacy::PhantomOrderConfiguration {
				chains: BoundedVec::try_from(vec![legacy::PhantomChainConfiguration {
					chain: v2::BSC,
					token_pairs: BoundedVec::try_from(vec![reversed.clone()]).unwrap(),
				}])
				.unwrap(),
				interval_blocks: 200,
			});

			v2::MigrateConfigToStorageMap::<Test>::on_runtime_upgrade();

			assert_eq!(
				crate::PhantomOrderConfig::<Test>::get(v2::BSC).unwrap().to_vec(),
				vec![reversed, usdt]
			);
			assert_eq!(crate::PhantomChains::<Test>::get().len(), 1);
		});
	}

	#[test]
	fn migration_is_a_no_op_without_a_stored_configuration() {
		new_test_ext().execute_with(|| {
			v2::MigrateConfigToStorageMap::<Test>::on_runtime_upgrade();

			// Only BNB Chain's seeded entry, and no interval: without a legacy value there is
			// nothing to lift, so governance sets the cadence.
			assert_eq!(
				crate::PhantomChains::<Test>::get().iter().copied().collect::<Vec<_>>(),
				vec![v2::BSC]
			);
			assert_eq!(crate::PhantomOrderConfig::<Test>::iter().count(), 1);
			assert_eq!(crate::PhantomOrderInterval::<Test>::get(), 0);
		});
	}
}
