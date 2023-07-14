// Copyright (C) 2023 Polytope Labs.
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

//! ISMP Assets
//! Simple Demo for Asset transfer over ISMP
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::string::ToString;
use frame_support::{traits::fungible::Mutate, PalletId};
use ismp::{
    error::Error as IsmpError,
    module::IsmpModule,
    router::{Post, Request, Response},
};
pub use pallet::*;
use pallet_ismp::primitives::ModuleId;

/// Constant Pallet ID
pub const PALLET_ID: ModuleId = ModuleId::Pallet(PalletId(*b"ismp-ast"));

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use alloc::{vec, vec::Vec};
    use frame_support::{
        pallet_prelude::*,
        traits::{
            fungible::{Inspect, Mutate},
            tokens::{Balance, Fortitude, Precision},
        },
    };
    use frame_system::pallet_prelude::*;
    use ismp::{
        host::StateMachine,
        router::{DispatchGet, DispatchPost, DispatchRequest, IsmpDispatcher},
    };

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Pallet Configuration
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config + pallet_ismp::Config {
        /// Overarching event
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Native balance
        type Balance: Balance + Into<<Self::NativeCurrency as Inspect<Self::AccountId>>::Balance>;
        /// Native currency implementation
        type NativeCurrency: Mutate<Self::AccountId>;
        /// Ismp message disptacher
        type IsmpDispatcher: IsmpDispatcher + Default;
    }

    /// Pallet events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Some balance has been transferred
        BalanceTransferred {
            /// Source account
            from: T::AccountId,
            /// Destination account
            to: T::AccountId,
            /// Amount being transferred
            amount: <T as Config>::Balance,
            /// Destination chain's Id
            dest_chain: StateMachine,
        },
        /// Some balance has been received
        BalanceReceived {
            /// Source account
            from: T::AccountId,
            /// Receiving account
            to: T::AccountId,
            /// Amount that was received
            amount: <T as Config>::Balance,
            /// Source chain's Id
            source_chain: StateMachine,
        },

        /// Get response recieved
        GetResponse(Vec<Option<Vec<u8>>>),
    }

    /// Pallet Errors
    #[pallet::error]
    pub enum Error<T> {
        /// Error encountered when initializing transfer
        TransferFailed,
        /// Failed to dispatch get request
        GetDispatchFailed,
    }

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfer some funds over ISMP
        #[pallet::weight(Weight::from_parts(1_000_000, 0))]
        #[pallet::call_index(0)]
        pub fn transfer(
            origin: OriginFor<T>,
            params: TransferParams<T::AccountId, <T as Config>::Balance>,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;

            // first, burn the requested amount
            <T::NativeCurrency as Mutate<T::AccountId>>::burn_from(
                &origin,
                params.amount.into(),
                Precision::Exact,
                Fortitude::Force,
            )?;

            // next, construct the request to be sent out
            let payload = Payload { to: params.to, from: origin.clone(), amount: params.amount };
            let dest = match T::StateMachine::get() {
                StateMachine::Kusama(_) => StateMachine::Kusama(params.para_id),
                StateMachine::Polkadot(_) => StateMachine::Polkadot(params.para_id),
                _ => Err(DispatchError::Other("Pallet only supports parachain hosts"))?,
            };
            let post = DispatchPost {
                dest,
                from: PALLET_ID.encode(),
                to: PALLET_ID.encode(),
                timeout_timestamp: params.timeout,
                data: payload.encode(),
            };

            // dispatch the request
            let dispatcher = T::IsmpDispatcher::default();
            dispatcher
                .dispatch_request(DispatchRequest::Post(post))
                .map_err(|_| Error::<T>::TransferFailed)?;

            // let the user know, they've successfully sent the funds
            Self::deposit_event(Event::<T>::BalanceTransferred {
                from: payload.from,
                to: payload.to,
                amount: payload.amount,
                dest_chain: dest,
            });

            Ok(())
        }

        /// Get the total issuance of the native token in a counterparty
        /// parachain
        #[pallet::weight(Weight::from_parts(1_000_000, 0))]
        #[pallet::call_index(1)]
        pub fn get_request(origin: OriginFor<T>, params: GetRequest) -> DispatchResult {
            ensure_signed(origin)?;
            let dest = match T::StateMachine::get() {
                StateMachine::Kusama(_) => StateMachine::Kusama(params.para_id),
                StateMachine::Polkadot(_) => StateMachine::Polkadot(params.para_id),
                _ => Err(DispatchError::Other("Pallet only supports parachain hosts"))?,
            };

            let get = DispatchGet {
                dest,
                from: PALLET_ID.encode(),
                keys: params.keys,
                height: params.height as u64,
                timeout_timestamp: params.timeout,
            };

            let dispatcher = T::IsmpDispatcher::default();
            dispatcher
                .dispatch_request(DispatchRequest::Get(get))
                .map_err(|_| Error::<T>::GetDispatchFailed)?;
            Ok(())
        }
    }

    /// Transfer payload
    /// This would be encoded to bytes as the request data
    #[derive(
        Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug,
    )]
    pub struct Payload<AccountId, Balance> {
        /// Destination account
        pub to: AccountId,
        /// Source account
        pub from: AccountId,
        /// Amount to be transferred
        pub amount: Balance,
    }

    /// The get request payload
    #[derive(
        Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug,
    )]
    pub struct GetRequest {
        /// Destination parachain
        pub para_id: u32,
        /// Height at which to read state
        pub height: u32,
        /// request timeout
        pub timeout: u64,
        /// Storage keys to read
        pub keys: Vec<Vec<u8>>,
    }

    /// Extrinsic Parameters for initializing a cross chain transfer
    #[derive(
        Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug,
    )]
    pub struct TransferParams<AccountId, Balance> {
        /// Destination account
        pub to: AccountId,

        /// Amount to transfer
        pub amount: Balance,

        /// Destination parachain Id
        pub para_id: u32,

        /// Timeout timestamp on destination chain in seconds
        pub timeout: u64,
    }
}

/// Module callback for the pallet
pub struct IsmpModuleCallback<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> Default for IsmpModuleCallback<T> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<T: Config> IsmpModule for IsmpModuleCallback<T> {
    fn on_accept(&self, request: Post) -> Result<(), IsmpError> {
        let source_chain = request.source;

        let payload = <Payload<T::AccountId, <T as Config>::Balance> as codec::Decode>::decode(
            &mut &*request.data,
        )
        .map_err(|_| {
            IsmpError::ImplementationSpecific("Failed to decode request data".to_string())
        })?;
        <T::NativeCurrency as Mutate<T::AccountId>>::mint_into(&payload.to, payload.amount.into())
            .map_err(|_| IsmpError::ImplementationSpecific("Failed to mint funds".to_string()))?;
        Pallet::<T>::deposit_event(Event::<T>::BalanceReceived {
            from: payload.from,
            to: payload.to,
            amount: payload.amount,
            source_chain,
        });
        Ok(())
    }

    fn on_response(&self, response: Response) -> Result<(), IsmpError> {
        match response {
            Response::Post(_) => Err(IsmpError::ImplementationSpecific(
                "Balance transfer protocol does not accept post responses".to_string(),
            ))?,
            Response::Get(res) => Pallet::<T>::deposit_event(Event::<T>::GetResponse(
                res.values.into_values().collect(),
            )),
        };

        Ok(())
    }

    fn on_timeout(&self, request: Request) -> Result<(), IsmpError> {
        let source_chain = request.source_chain();
        let data = match request {
            Request::Post(post) => post.data,
            _ => Err(IsmpError::ImplementationSpecific(
                "Only Post requests allowed, found Get".to_string(),
            ))?,
        };
        let payload =
            <Payload<T::AccountId, <T as Config>::Balance> as codec::Decode>::decode(&mut &*data)
                .map_err(|_| {
                IsmpError::ImplementationSpecific("Failed to decode request data".to_string())
            })?;
        <T::NativeCurrency as Mutate<T::AccountId>>::mint_into(
            &payload.from,
            payload.amount.into(),
        )
        .map_err(|_| IsmpError::ImplementationSpecific("Failed to mint funds".to_string()))?;
        Pallet::<T>::deposit_event(Event::<T>::BalanceReceived {
            from: payload.from,
            to: payload.to,
            amount: payload.amount,
            source_chain,
        });
        Ok(())
    }
}
