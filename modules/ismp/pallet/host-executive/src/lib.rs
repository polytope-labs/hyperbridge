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

mod params;

extern crate alloc;

pub use pallet::*;
pub use params::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use alloc::{collections::BTreeMap, vec};
    use alloy_rlp::Encodable;
    use frame_support::{
        pallet_prelude::{OptionQuery, *},
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use ismp::{
        host::StateMachine,
        router::{DispatchPost, DispatchRequest, IsmpDispatcher},
    };
    use pallet_ismp::{dispatcher::Dispatcher, primitives::ModuleId};

    /// ISMP module identifier
    pub const PALLET_ID: ModuleId = ModuleId::Pallet(PalletId(*b"hostexec"));

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {}

    /// Host Manager Addresses on different chains
    #[pallet::storage]
    #[pallet::getter(fn host_params)]
    pub type HostParams<T: Config> =
        StorageMap<_, Twox64Concat, StateMachine, HostParam, OptionQuery>;

    #[pallet::error]
    pub enum Error<T> {
        /// Could not commit the outgoing request
        DispatchFailed,
        /// The requested state machine was unrecognized
        UnknownStateMachine,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        T::AccountId: From<[u8; 32]>,
    {
        /// Set the host params for all the different state machines
        #[pallet::weight(T::DbWeight::get().writes(1))]
        #[pallet::call_index(0)]
        pub fn set_host_params(
            origin: OriginFor<T>,
            params: BTreeMap<StateMachine, HostParam>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            for (state_machine, params) in params {
                HostParams::<T>::insert(state_machine, params);
            }

            Ok(())
        }

        /// Set the host params for the provided state machine
        #[pallet::weight(T::DbWeight::get().writes(1))]
        #[pallet::call_index(1)]
        pub fn update_host_params(
            origin: OriginFor<T>,
            state_machine: StateMachine,
            update: HostParamUpdate,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let mut params = HostParams::<T>::get(&state_machine)
                .ok_or_else(|| Error::<T>::UnknownStateMachine)?;

            params.update(update);

            let mut data = vec![1u8]; // enum variant for the host manager
            HostParamRlp::try_from(params.clone())
                .expect("u128 will always fit inside a U256; qed")
                .encode(&mut data);
            let dispatcher = Dispatcher::<T>::default();
            dispatcher
                .dispatch_request(
                    DispatchRequest::Post(DispatchPost {
                        dest: state_machine,
                        from: PALLET_ID.to_bytes(),
                        to: params.host_manager.0.to_vec(),
                        timeout_timestamp: 0,
                        gas_limit: 0,
                        data,
                    }),
                    [0u8; 32].into(),
                    Default::default(),
                )
                .map_err(|_| Error::<T>::DispatchFailed)?;

            HostParams::<T>::insert(state_machine, params);

            Ok(())
        }
    }
}
