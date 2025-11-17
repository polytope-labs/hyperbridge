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
use crate::{pallet, types::WeightInfo, Config};
use alloc::{boxed::Box, vec::Vec};
use codec::Decode;
use frame_support::{
	pallet_prelude::*,
	storage::{storage_prefix, unhashed, KeyPrefixIterator},
	traits::Get,
	weights::WeightMeter,
};
use ismp::host::StateMachine;
use polkadot_sdk::{sp_core::U256, *};
use sp_runtime::Saturating;

use frame_support::migrations::{MigrationId, SteppedMigration, SteppedMigrationError};

const PALLET_MIGRATIONS_ID: &[u8; 13] = b"MessagingFees";

pub mod v1 {
	use super::*;
	use pallet_migrations;

	pub struct Migration<T: Config>(PhantomData<T>);

	impl<T: Config + pallet_migrations::Config> SteppedMigration for Migration<T> {
		type Cursor = BoundedVec<u8, <T as pallet_migrations::Config>::CursorMaxLen>;
		type Identifier = MigrationId<13>;

		fn id() -> Self::Identifier {
			MigrationId { pallet_id: *PALLET_MIGRATIONS_ID, version_from: 0, version_to: 1 }
		}

		fn step(
			mut cursor: Option<Self::Cursor>,
			meter: &mut WeightMeter,
		) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
			let weight_per_item = <T as pallet::Config>::WeightInfo::migrate_evm_fees();
			if meter.remaining().any_lt(weight_per_item) {
				return Err(SteppedMigrationError::InsufficientWeight { required: weight_per_item });
			}

			let fee_storage_prefix = storage_prefix(b"Relayer", b"Fees");

			let mut iter: Box<dyn Iterator<Item = Vec<u8>>> = if let Some(c) = cursor.as_ref() {
				let cursor_vec = c.as_slice().to_vec();
				Box::new(
					KeyPrefixIterator::new(
						fee_storage_prefix.to_vec(),
						fee_storage_prefix.to_vec(),
						|k| Ok(k.to_vec()),
					)
					.skip_while(move |k_vec| k_vec != &cursor_vec)
					.skip(1),
				)
			} else {
				Box::new(KeyPrefixIterator::new(
					fee_storage_prefix.to_vec(),
					fee_storage_prefix.to_vec(),
					|k| Ok(k.to_vec()),
				))
			};

			if let Some(key_suffix) = iter.next() {
				meter.consume(weight_per_item);
				let full_key = [fee_storage_prefix.as_slice(), key_suffix.as_slice()].concat();
				let mut key_part = &key_suffix[16..];

				if let Ok(state_machine) = StateMachine::decode(&mut key_part) {
					if state_machine.is_evm() {
						if key_part.len() > 16 {
							let mut relayer_address_bytes = &key_part[16..];
							if let Ok(relayer_address) =
								Vec::<u8>::decode(&mut relayer_address_bytes)
							{
								if relayer_address.len() == 32 {
									if let Some(current_fee) = unhashed::get::<U256>(&full_key) {
										if let Some(decimals) =
											pallet_ismp_host_executive::FeeTokenDecimals::<T>::get(
												&state_machine,
											) {
											let decimals_u32 = decimals as u32;
											let scaling_power = 18u32.saturating_sub(decimals_u32);

											if scaling_power > 0 {
												let divisor = U256::from(10u128)
													.pow(U256::from(scaling_power));
												let new_fee = current_fee
													.checked_div(divisor)
													.unwrap_or(U256::zero());
												storage::unhashed::put(&full_key, &new_fee);
											}
										}
									}
								}
							}
						}
					}
				}

				let bounded_key: BoundedVec<_, _> = key_suffix.try_into().map_err(|_| {
					log::warn!(target: "ismp", "MIGRATION: key_suffix is too long for BoundedVec");
					SteppedMigrationError::Failed
				})?;
				cursor = Some(bounded_key);
			} else {
				cursor = None;
			}

			Ok(cursor)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, frame_support::sp_runtime::TryRuntimeError> {
			use crate::Pallet;
			log::info!(target: "ismp", "MessagingFees migration: pre-upgrade check");
			assert_eq!(StorageVersion::get::<Pallet<T>>(), 0, "Expected storage version 0");
			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_state: Vec<u8>) -> Result<(), frame_support::sp_runtime::TryRuntimeError> {
			use crate::Pallet;
			log::info!(target: "ismp", "MessagingFees migration: post-upgrade check");
			assert_eq!(StorageVersion::get::<Pallet<T>>(), 1, "Expected storage version 1");
			Ok(())
		}
	}
}
