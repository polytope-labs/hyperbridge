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
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use alloc::{collections::BTreeMap, vec::Vec};
    use frame_support::pallet_prelude::{OptionQuery, *};
    use frame_system::pallet_prelude::*;
    use ismp::{
        consensus::StateMachineId,
        host::{IsmpHost, StateMachine},
    };

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {
        /// The runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    /// Whitelisted Account allowed for freezing state machine
    #[pallet::storage]
    #[pallet::getter(fn whitelist)]
    pub type WhitelistedAccount<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, (), OptionQuery>;

    /// Host Manager Address on different chains
    #[pallet::storage]
    #[pallet::getter(fn host_manager)]
    pub type HostManager<T: Config> =
        StorageMap<_, Twox64Concat, StateMachine, Vec<u8>, OptionQuery>;

    #[pallet::error]
    pub enum Error<T> {
        /// Account Already Whitelisted
        AccountAlreadyWhitelisted,
        /// Account not whitelisted to freeze state machine
        AccountNotWhitelisted,
        /// State Machine Already Frozen
        StateMachineAlreadyFrozen,
        /// State Machine Not Frozen
        StateMachineNotFrozen,
        /// Error Freezing State Machine
        ErrorFreezingStateMachine,
        /// Error Unfreezing State Machine
        ErrorUnFreezingStateMachine,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An account `account` has been whitelisted
        AccountWhitelisted { account: T::AccountId },
        /// An account `account` has been removed from whitelisted accounts
        AccountRemovedFromWhitelistedAccount { account: T::AccountId },
        /// `state_machine` is frozen
        StateMachineFrozen { state_machine: StateMachineId },
        ///  `state_machine` is unfrozen
        StateMachineUnFrozen { state_machine: StateMachineId },
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as frame_system::Config>::AccountId: From<[u8; 32]>,
    {
        #[pallet::call_index(0)]
        #[pallet::weight({1_000_000})]
        pub fn add_whitelist_account(
            origin: OriginFor<T>,
            account: T::AccountId,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            ensure!(
                !WhitelistedAccount::<T>::contains_key(&account),
                Error::<T>::AccountAlreadyWhitelisted
            );
            WhitelistedAccount::<T>::insert(&account, ());

            Self::deposit_event(Event::AccountWhitelisted { account });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight({1_000_000})]
        pub fn remove_whitelist_account(
            origin: OriginFor<T>,
            account: T::AccountId,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            ensure!(
                WhitelistedAccount::<T>::contains_key(&account),
                Error::<T>::AccountNotWhitelisted
            );
            WhitelistedAccount::<T>::remove(&account);

            Self::deposit_event(Event::AccountRemovedFromWhitelistedAccount { account });
            Ok(())
        }

        // #[pallet::call_index(2)]
        // #[pallet::weight({1_000_000})]
        // pub fn freeze_state_machine(
        //     origin: OriginFor<T>,
        //     state_machine: StateMachineId,
        // ) -> DispatchResult {
        //     let account = ensure_signed(origin)?;
        //     ensure!(
        //         WhitelistedAccount::<T>::contains_key(&account),
        //         Error::<T>::AccountNotWhitelisted
        //     );
        //
        //     let ismp_host = Host::<T>::default();
        //     ismp_host
        //         .is_state_machine_frozen(state_machine.clone())
        //         .map_err(|_| Error::<T>::StateMachineAlreadyFrozen)?;
        //     ismp_host
        //         .delete_state_commitment(state_machine)
        //         .map_err(|_| Error::<T>::ErrorFreezingStateMachine)?;
        //
        //     Self::deposit_event(Event::StateMachineFrozen { state_machine });
        //     Ok(())
        // }

        // #[pallet::call_index(3)]
        // #[pallet::weight({1_000_000})]
        // pub fn unfreeze_state_machine(
        //     origin: OriginFor<T>,
        //     state_machine: StateMachineId,
        // ) -> DispatchResult {
        //     T::AdminOrigin::ensure_origin(origin)?;
        //
        //     let ismp_host = Host::<T>::default();
        //     let result = ismp_host.is_state_machine_frozen(state_machine.clone());
        //     ensure!(result.is_err(), Error::<T>::StateMachineNotFrozen);
        //     ismp_host
        //         .unfreeze_state_machine(state_machine)
        //         .map_err(|_| Error::<T>::ErrorUnFreezingStateMachine)?;
        //
        //     Self::deposit_event(Event::StateMachineUnFrozen { state_machine });
        //
        //     Ok(())
        // }

        /// Set the host manager addresses for different state machines
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(addresses.len() as u64))]
        #[pallet::call_index(4)]
        pub fn set_host_manger_addresses(
            origin: OriginFor<T>,
            addresses: BTreeMap<StateMachine, Vec<u8>>,
        ) -> DispatchResult {
            <T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;

            for (state_machine, address) in addresses {
                HostManager::<T>::insert(state_machine, address);
            }

            Ok(())
        }
    }
}
