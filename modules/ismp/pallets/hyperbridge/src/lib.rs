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

//! Pallet Hyperbridge

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use codec::{Decode, Encode};
use frame_support::{
    sp_runtime::{testing::H256, traits::AccountIdConversion},
    traits::{Currency, ExistenceRequirement, Get},
};
use ismp::{
    dispatcher::{DispatchRequest, FeeMetadata, IsmpDispatcher},
    module::IsmpModule,
    router::{Post, PostResponse, Response, Timeout},
};
use pallet_ismp::{host::Host, RELAYER_FEE_ACCOUNT};

pub use pallet::*;

/// Parameters that govern the working operations of this module. Versioned for ease of migration.
#[derive(
    Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq, codec::MaxEncodedLen,
)]
pub enum VersionedHostParams<Balance> {
    /// The per-byte fee that hyperbridge charges for outgoing requests and responses.
    V1(Balance),
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{pallet_prelude::*, PalletId};
    use frame_system::pallet_prelude::*;

    /// [`PalletId`] where protocol fees will be collected
    pub const PALLET_HYPERBRIDGE: PalletId = PalletId(*b"HYPR-FEE");

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_ismp::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The host parameters of the pallet-hyperbridge.
    #[pallet::storage]
    #[pallet::getter(fn host_params)]
    pub type HostParams<T> =
        StorageValue<_, VersionedHostParams<<T as pallet_balances::Config>::Balance>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Hyperbridge governance has now updated it's per-byte fee.
        PerByteFeeChanged {
            /// The old per-byte fee
            old: T::Balance,
            /// The new per-byte fee
            new: T::Balance,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Error names should be descriptive.
        NoneValue,
        /// Errors should have helpful documentation associated with them.
        StorageOverflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            // todo: how to initialize this pallet?
            Weight::zero()
        }
    }


    // Hack for implementing the [`Default`] bound
    impl<T> Default for Pallet<T> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }
}

/// [`IsmpDispatcher`] implementation for dispatching requests to the hyperbridge coprocessor.
/// Charges the hyperbridge protocol fee on a per-byte basis.
///
/// **NOTE** Hyperbridge WILL NOT accept requests that were not dispatched through this
/// implementation.
impl<T> IsmpDispatcher for Pallet<T>
where
    T: Config,
    T::Balance: Into<u128> + From<u128>,
{
    type Account = T::AccountId;
    type Balance = T::Balance;

    fn dispatch_request(
        &self,
        request: DispatchRequest,
        fee: FeeMetadata<Self::Account, Self::Balance>,
    ) -> Result<H256, ismp::Error> {
        if let DispatchRequest::Post(ref post) = request {
            let Some(VersionedHostParams::V1(per_byte_fee)) = Self::host_params() else {
                // can't dispatch requests unfortunately.
                Err(ismp::Error::ImplementationSpecific(
                    "HostParams is not yet configured!".to_string(),
                ))?
            };
            let fees = per_byte_fee.into() * post.data.len() as u128;
            // collect protocol fees
            if fees != 0 {
                T::Currency::transfer(
                    &fee.payer,
                    &PALLET_HYPERBRIDGE.into_account_truncating(),
                    fees.into(),
                    ExistenceRequirement::AllowDeath,
                )
                .map_err(|err| {
                    ismp::Error::ImplementationSpecific(format!(
                        "Error withdrawing request fees: {err:?}"
                    ))
                })?;
            }
        };

        let host = Host::<T>::default();
        host.dispatch_request(request, fee)

        // commit the request commitment to child-trie
    }

    fn dispatch_response(
        &self,
        response: PostResponse,
        fee: FeeMetadata<Self::Account, Self::Balance>,
    ) -> Result<H256, ismp::Error> {
        // collect protocol fees
        let Some(VersionedHostParams::V1(per_byte_fee)) = Self::host_params() else {
            // can't dispatch responses unfortunately.
            Err(ismp::Error::ImplementationSpecific(
                "HostParams is not yet configured!".to_string(),
            ))?
        };
        let fees = per_byte_fee.into() * response.response.len() as u128;

        if fees != 0 {
            T::Currency::transfer(
                &fee.payer,
                &PALLET_HYPERBRIDGE.into_account_truncating(),
                fees.into(),
                ExistenceRequirement::AllowDeath,
            )
            .map_err(|err| {
                ismp::Error::ImplementationSpecific(format!(
                    "Error withdrawing request fees: {err:?}"
                ))
            })?;
        }

        let host = Host::<T>::default();
        host.dispatch_response(response, fee)

        // commit the response commitment to child-trie
    }
}

/// A request to withdraw some funds. Could either be for protocol revenue or relayer fees.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct WithdrawalRequest<Account, Amount> {
    /// The amount to be withdrawn
    pub amount: Amount,
    /// The withdrawal beneficiary
    pub account: Account,
}

/// Cross-chain messages to this module. This module will only accept messages from the hyperbridge
/// chain. Assumed to be configured in [`pallet_ismp::Config`]
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub enum Message<Account, Balance> {
    /// Set some new host params
    UpdateHostParams(VersionedHostParams<Balance>),
    /// Withdraw the hyperbridge protocol reveneue
    WithdrawProtocolFees(WithdrawalRequest<Account, Balance>),
    /// Withdraw the fees owed to a relayer
    WithdrawRelayerFees(WithdrawalRequest<Account, Balance>),
}

impl<T> IsmpModule for Pallet<T>
where
    T: Config,
    T::Balance: Into<u128> + From<u128>,
{
    fn on_accept(&self, request: Post) -> Result<(), ismp::Error> {
        // this of course assumes that hyperbridge is configured as the coprocessor.
        // any attempts to fool this module into setting a lower per-byte fee will be moot
        // as hyperbridge will check the amount paid for every request before accepting them
        let source = request.source;
        if Some(source) != T::Coprocessor::get() {
            Err(ismp::Error::ImplementationSpecific(format!("Invalid request source: {source}")))?
        }

        let message =
            Message::<T::AccountId, T::Balance>::decode(&mut &request.data[..]).map_err(|err| {
                ismp::Error::ImplementationSpecific(format!(
                    "Failed to decode per-byte fee: {err:?}"
                ))
            })?;

        match message {
            Message::UpdateHostParams(params) => {
                // todo: events
                HostParams::<T>::put(params)
            },
            Message::WithdrawProtocolFees(withdrawal) => {
                // todo: events

                T::Currency::transfer(
                    &PALLET_HYPERBRIDGE.into_account_truncating(),
                    &withdrawal.account,
                    withdrawal.amount,
                    ExistenceRequirement::AllowDeath,
                )
                .map_err(|err| {
                    ismp::Error::ImplementationSpecific(format!(
                        "Error withdrawing protocol fees: {err:?}"
                    ))
                })?;
            },
            Message::WithdrawRelayerFees(withdrawal) => {
                // todo: events

                T::Currency::transfer(
                    &RELAYER_FEE_ACCOUNT.into_account_truncating(),
                    &withdrawal.account,
                    withdrawal.amount,
                    ExistenceRequirement::AllowDeath,
                )
                .map_err(|err| {
                    ismp::Error::ImplementationSpecific(format!(
                        "Error withdrawing protocol fees: {err:?}"
                    ))
                })?;
            },
        };

        Ok(())
    }

    fn on_response(&self, _response: Response) -> Result<(), ismp::Error> {
        // this module does not expect responses
        Err(ismp::Error::CannotHandleMessage)
    }

    fn on_timeout(&self, _request: Timeout) -> Result<(), ismp::Error> {
        // this module does not dispatch requests
        Err(ismp::Error::CannotHandleMessage)
    }
}
