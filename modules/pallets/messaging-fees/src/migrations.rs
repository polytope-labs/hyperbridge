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
    use polkadot_sdk::*;
    use crate::Config;
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use codec::Decode;
    use frame_support::{
        pallet_prelude::*,
        storage::{storage_prefix, KeyPrefixIterator},
        traits::OnRuntimeUpgrade,
    };
    use ismp::host::StateMachine;
    use polkadot_sdk::sp_core::U256;
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

            let mut iter: Box<dyn Iterator<Item = Vec<u8>>> =
                if let Some(key) = start_key.clone() {
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
            let weight_per_item = Weight::from_parts(200_000_000, 0);
            weight.saturating_accrue(T::DbWeight::get().reads(1));

            while let Some(key_suffix) = iter.next() {
                let full_key = [fee_storage_prefix.as_slice(), key_suffix.as_slice()].concat();

                if weight.saturating_add(weight_per_item).ref_time() >= max_weight.ref_time() {
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

                    if state_machine.is_substrate() {
                        log::info!(target: "ismp", "MIGRATION: Is Substrate. Resetting fee for key {:?}", &full_key);
                        storage::unhashed::put(&full_key, &U256::zero());
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

