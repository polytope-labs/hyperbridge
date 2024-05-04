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
        dispatcher::{DispatchPost, DispatchRequest, FeeMetadata, IsmpDispatcher},
        host::StateMachine,
    };
    use pallet_hyperbridge::{Message, PALLET_HYPERBRIDGE};
    use pallet_ismp::ModuleId;

    /// ISMP module identifier
    pub const PALLET_ID: ModuleId = ModuleId::Pallet(PalletId(*b"hostexec"));

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The config trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The [`IsmpDispatcher`] implementation to use for dispatching requests
        type IsmpHost: IsmpDispatcher<Account = Self::AccountId, Balance = Self::Balance>;
    }

    /// Host Manager Addresses on different chains
    #[pallet::storage]
    #[pallet::getter(fn host_params)]
    pub type HostParams<T: Config> = StorageMap<
        _,
        Twox64Concat,
        StateMachine,
        HostParam<<T as pallet_ismp::Config>::Balance>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Hyperbridge governance has initiated a host parameter update to the mentioned state
        /// machine
        HostParamsUpdated {
            /// State machine's whose host params should be updated
            state_machine: StateMachine,
            /// The old host params
            old: HostParam<<T as pallet_ismp::Config>::Balance>,
            /// The new host params
            new: HostParam<<T as pallet_ismp::Config>::Balance>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Could not commit the outgoing request
        DispatchFailed,
        /// The requested state machine was unrecognized
        UnknownStateMachine,
        /// Mismatched state machine and HostParams
        MismatchedHostParams,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        T::AccountId: From<[u8; 32]>,
    {
        /// Initialize the host params for all the different state machines
        #[pallet::weight(T::DbWeight::get().writes(1))]
        #[pallet::call_index(0)]
        pub fn set_host_params(
            origin: OriginFor<T>,
            params: BTreeMap<StateMachine, HostParam<<T as pallet_ismp::Config>::Balance>>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            for (state_machine, params) in params {
                HostParams::<T>::insert(state_machine, params);
            }

            Ok(())
        }

        /// Update the host params for the provided state machine
        #[pallet::weight(T::DbWeight::get().writes(1))]
        #[pallet::call_index(1)]
        pub fn update_host_params(
            origin: OriginFor<T>,
            state_machine: StateMachine,
            update: HostParamUpdate<T::Balance>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let params = HostParams::<T>::get(&state_machine)
                .ok_or_else(|| Error::<T>::UnknownStateMachine)?;

            let (post, updated) = match (params.clone(), update) {
                (HostParam::EvmHostParam(mut inner), HostParamUpdate::EvmHostParam(update)) => {
                    inner.update(update);

                    let mut body = vec![1u8]; // enum variant for the host manager
                    EvmHostParamRlp::try_from(inner.clone())
                        .expect("u128 will always fit inside a U256; qed")
                        .encode(&mut body);

                    let post = DispatchPost {
                        dest: state_machine,
                        from: PALLET_ID.to_bytes(),
                        to: inner.host_manager.0.to_vec(),
                        timeout: 0,
                        body,
                    };

                    (post, HostParam::EvmHostParam(inner))
                },
                (HostParam::SubstrateHostParam(_), HostParamUpdate::SubstrateHostParam(update)) => {
                    let body =
                        Message::<T::AccountId, T::Balance>::UpdateHostParams(update.clone())
                            .encode();

                    let post = DispatchPost {
                        dest: state_machine,
                        from: PALLET_ID.to_bytes(),
                        to: PALLET_HYPERBRIDGE.0.to_vec(),
                        timeout: 0,
                        body,
                    };

                    (post, HostParam::SubstrateHostParam(update))
                },
                _ => return Err(Error::<T>::MismatchedHostParams.into()),
            };

            let dispatcher = <T as Config>::IsmpHost::default();
            dispatcher
                .dispatch_request(
                    DispatchRequest::Post(post),
                    FeeMetadata { payer: [0u8; 32].into(), fee: Default::default() },
                )
                .map_err(|_| Error::<T>::DispatchFailed)?;

            HostParams::<T>::insert(state_machine, updated.clone());

            Self::deposit_event(Event::<T>::HostParamsUpdated {
                state_machine,
                old: params,
                new: updated,
            });

            Ok(())
        }
    }
}
