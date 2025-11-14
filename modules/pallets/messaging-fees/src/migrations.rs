// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
pub use migration_v0::*;

pub mod migration_v0 {
	use crate::{storage::unhashed, types::WeightInfo, Config};
	use alloc::{boxed::Box, vec::Vec};
	use codec::Decode;
	use frame_support::{
		pallet_prelude::*,
		storage::{storage_prefix, KeyPrefixIterator},
		traits::OnRuntimeUpgrade,
	};
	use ismp::host::StateMachine;
	use polkadot_sdk::{sp_core::U256, *};
	use sp_runtime::Saturating;

	pub struct MigrationV0<T: Config>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrationV0<T> {
		fn on_runtime_upgrade() -> Weight {
			let mut weight = Weight::zero();
			let current_version = StorageVersion::get::<crate::Pallet<T>>();

			if current_version == 0 {
				if !crate::MigrationInProgress::<T>::get() {
					crate::MigrationInProgress::<T>::put(true);
					crate::Pallet::<T>::deposit_event(crate::Event::FeeMigrationStarted);
					log::info!(target: "ismp", "Fee migration started.");
				}
			} else {
				log::info!(target: "ismp", "Migration already completed, skipping.");
				return weight;
			}

			let fee_storage_prefix = storage_prefix(b"Relayer", b"Fees");
			let start_key = crate::LastProcessedMigrationKey::<T>::get();

			let mut iter: Box<dyn Iterator<Item = Vec<u8>>> = if let Some(key) = start_key.clone() {
				Box::new(
					KeyPrefixIterator::new(
						fee_storage_prefix.to_vec(),
						fee_storage_prefix.to_vec(),
						|k| Ok(k.to_vec()),
					)
					.skip_while(move |k_vec| k_vec != &key)
					.skip(1),
				)
			} else {
				Box::new(KeyPrefixIterator::new(
					fee_storage_prefix.to_vec(),
					fee_storage_prefix.to_vec(),
					|k| Ok(k.to_vec()),
				))
			};

			let mut items_processed = 0u32;
			let max_weight = T::BlockWeights::get().max_block;
			let weight_per_item = T::WeightInfo::migrate_evm_fees();
			weight.saturating_accrue(T::DbWeight::get().reads(1));

			while let Some(key_suffix) = iter.next() {
				let full_key = [fee_storage_prefix.as_slice(), key_suffix.as_slice()].concat();

				if weight.saturating_add(weight_per_item).ref_time() >= (max_weight.ref_time() / 2)
				{
					crate::LastProcessedMigrationKey::<T>::put(key_suffix);
					weight.saturating_accrue(T::DbWeight::get().writes(1));
					log::info!(target: "ismp", "Fee migration paused. Processed {} items.", items_processed);
					return weight;
				}
				weight.saturating_accrue(weight_per_item);
				items_processed.saturating_inc();

				let mut state_machine_key_part = &key_suffix[16..];

				log::info!(target: "ismp", "MIGRATION: Attempting to decode key part: {:?}", &state_machine_key_part);

				if let Ok(state_machine) = StateMachine::decode(&mut state_machine_key_part) {
					if state_machine.is_evm() {
						if state_machine_key_part.len() > 16 {
							let mut relayer_address_bytes = &state_machine_key_part[16..];

							if let Ok(relayer_address) =
								Vec::<u8>::decode(&mut relayer_address_bytes)
							{
								if relayer_address.len() == 32 {
									let current_fee = match unhashed::get::<U256>(&full_key) {
										Some(f) if !f.is_zero() => f,
										_ => {
											log::info!(target: "ismp", "MIGRATION: Fee is zero or not found. Skipping key {:?}", &full_key);
											continue;
										},
									};

									if let Some(decimals) =
										pallet_ismp_host_executive::FeeTokenDecimals::<T>::get(
											&state_machine,
										) {
										let decimals_u32 = decimals as u32;
										let scaling_power = 18u32.saturating_sub(decimals_u32);

										if scaling_power > 0 {
											let divisor =
												U256::from(10u128).pow(U256::from(scaling_power));
											let new_fee = current_fee
												.checked_div(divisor)
												.unwrap_or(U256::zero());

											log::info!(target: "ismp", "MIGRATION: Scaling fee for {:?}. Decimals: {}. Old fee: {}. New fee: {}.", &state_machine, decimals, current_fee, new_fee);
											storage::unhashed::put(&full_key, &new_fee);
										} else {
											log::info!(target: "ismp", "MIGRATION: Is EVM, 32-byte addr. Decimals are 18 or more ({}), no scaling needed. Skipping key {:?}", decimals, &full_key);
										}
									} else {
										log::warn!(target: "ismp", "MIGRATION: Is EVM, 32-byte addr, but could not find decimals for {:?}. Skipping key {:?}", &state_machine, &full_key);
									}
								} else {
									log::info!(target: "ismp", "MIGRATION: Is EVM, but address len is {}. Skipping.", relayer_address.len());
								}
							} else {
								log::warn!(target: "ismp", "MIGRATION: Is EVM but failed to decode RelayerAddress from key: {:?}", &full_key);
							}
						}
					}
				} else {
					log::warn!(
						target: "ismp",
						"MIGRATION: Failed to decode StateMachine from key: {:?}",
						full_key
					);
				}

				crate::LastProcessedMigrationKey::<T>::put(&key_suffix);

				if weight.ref_time() >= (T::BlockWeights::get().max_block.ref_time() / 2) {
					log::info!(
						target: "ismp",
						"Migration paused, processed {} items.",
						items_processed
					);
					return weight;
				}
			}

			log::info!(
				target: "ismp",
				"Fee migration completed. Processed {} items in final batch.",
				items_processed
			);
			crate::MigrationInProgress::<T>::put(false);
			crate::LastProcessedMigrationKey::<T>::kill();
			crate::Pallet::<T>::deposit_event(crate::Event::FeeMigrationCompleted);
			StorageVersion::new(1).put::<crate::Pallet<T>>();
			weight
		}
	}
}
