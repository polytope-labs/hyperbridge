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

//! The host executive is tasked with managing the ISMP hosts on all connected chains.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use alloc::{collections::BTreeMap, vec::Vec};
    use frame_support::pallet_prelude::{OptionQuery, *};
    use frame_system::pallet_prelude::*;
    use ismp::{consensus::StateMachineId, host::StateMachine};

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {
        /// The runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    /// Host Manager Addresses on different chains
    #[pallet::storage]
    #[pallet::getter(fn host_manager)]
    pub type HostManagers<T: Config> =
        StorageMap<_, Twox64Concat, StateMachine, Vec<u8>, OptionQuery>;

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as frame_system::Config>::AccountId: From<[u8; 32]>,
    {
        /// Set the host manager addresses for different state machines
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(addresses.len() as u64))]
        #[pallet::call_index(4)]
        pub fn set_host_manger_addresses(
            origin: OriginFor<T>,
            addresses: BTreeMap<StateMachine, Vec<u8>>,
        ) -> DispatchResult {
            <T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;

            for (state_machine, address) in addresses {
                HostManagers::<T>::insert(state_machine, address);
            }

            Ok(())
        }
    }
}
