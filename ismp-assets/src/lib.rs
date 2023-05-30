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
    module::IsmpModule,
    router::{Request, Response},
};
pub use pallet::*;

/// Constant Pallet ID
pub const PALLET_ID: PalletId = PalletId(*b"ismp-ast");

#[frame_support::pallet]
pub mod pallet {
    use super::*;
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
        router::{DispatchPost, DispatchRequest, IsmpDispatcher},
    };

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Pallet Configuration
    #[pallet::config]
    pub trait Config: frame_system::Config {
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
            amount: T::Balance,
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
            amount: T::Balance,
            /// Source chain's Id
            source_chain: StateMachine,
        },
    }

    /// Pallet Errors
    #[pallet::error]
    pub enum Error<T> {
        /// Error encountered when initializing transfer
        TransferFailed,
    }

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfer some funds over ISMP
        #[pallet::weight(1_000_000)]
        #[pallet::call_index(0)]
        pub fn transfer(
            origin: OriginFor<T>,
            params: TransferParams<T::AccountId, T::Balance>,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;
            let payload = Payload { to: params.to, from: origin.clone(), amount: params.amount };
            let post = DispatchPost {
                dest_chain: params.dest_chain,
                from: PALLET_ID.0.to_vec(),
                to: PALLET_ID.0.to_vec(),
                timeout_timestamp: params.timeout,
                data: payload.encode(),
            };

            let dispatcher = T::IsmpDispatcher::default();
            dispatcher
                .dispatch_request(DispatchRequest::Post(post))
                .map_err(|_| Error::<T>::TransferFailed)?;
            <T::NativeCurrency as Mutate<T::AccountId>>::burn_from(
                &origin,
                params.amount.into(),
                Precision::Exact,
                Fortitude::Force,
            )?;
            Self::deposit_event(Event::<T>::BalanceTransferred {
                from: payload.from,
                to: payload.to,
                amount: payload.amount,
                dest_chain: params.dest_chain,
            });
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

    /// Extrinsic Parameters for initializing a cross chain transfer
    #[derive(
        Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug,
    )]
    pub struct TransferParams<AccountId, Balance> {
        /// Destination account
        pub to: AccountId,
        /// Amount to transfer
        pub amount: Balance,
        /// Destination chain's Id
        pub dest_chain: StateMachine,
        /// Timeout timestamp on destination chain in seconds
        pub timeout: u64,
    }
}

/// Ismp dispatch error
fn ismp_dispatch_error(msg: &'static str) -> ismp::error::Error {
    ismp::error::Error::ImplementationSpecific(msg.to_string())
}

impl<T: Config> IsmpModule for Pallet<T> {
    fn on_accept(request: Request) -> Result<(), ismp::error::Error> {
        let source_chain = request.source_chain();
        let data = match request {
            Request::Post(post) => post.data,
            _ => Err(ismp_dispatch_error("Only Post requests allowed, found Get"))?,
        };

        let payload = <Payload<T::AccountId, T::Balance> as codec::Decode>::decode(&mut &*data)
            .map_err(|_| ismp_dispatch_error("Failed to decode request data"))?;
        <T::NativeCurrency as Mutate<T::AccountId>>::mint_into(&payload.to, payload.amount.into())
            .map_err(|_| ismp_dispatch_error("Failed to mint funds"))?;
        Pallet::<T>::deposit_event(Event::<T>::BalanceReceived {
            from: payload.from,
            to: payload.to,
            amount: payload.amount,
            source_chain,
        });
        Ok(())
    }

    fn on_response(_response: Response) -> Result<(), ismp::error::Error> {
        Err(ismp_dispatch_error("Balance transfer protocol does not accept responses"))
    }

    fn on_timeout(request: Request) -> Result<(), ismp::error::Error> {
        let source_chain = request.source_chain();
        let data = match request {
            Request::Post(post) => post.data,
            _ => Err(ismp_dispatch_error("Only Post requests allowed, found Get"))?,
        };
        let payload = <Payload<T::AccountId, T::Balance> as codec::Decode>::decode(&mut &*data)
            .map_err(|_| ismp_dispatch_error("Failed to decode request data"))?;
        <T::NativeCurrency as Mutate<T::AccountId>>::mint_into(
            &payload.from,
            payload.amount.into(),
        )
        .map_err(|_| ismp_dispatch_error("Failed to mint funds"))?;
        Pallet::<T>::deposit_event(Event::<T>::BalanceReceived {
            from: payload.from,
            to: payload.to,
            amount: payload.amount,
            source_chain,
        });
        Ok(())
    }
}
