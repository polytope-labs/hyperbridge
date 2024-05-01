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

//! Implementation for the low-level ISMP Dispatcher

use crate::{child_trie::RequestReceipts, host::Host, mmr::LeafIndexAndPos, Pallet};
use frame_support::{
    traits::{Currency, ExistenceRequirement, UnixTime},
    PalletId,
};
use ismp::{
    dispatcher,
    dispatcher::{DispatchRequest, IsmpDispatcher},
    error::Error as IsmpError,
    events::Meta,
    host::IsmpHost,
    messaging::hash_request,
    router::{Get, Post, PostResponse, Request, Response},
};
use sp_runtime::traits::{AccountIdConversion, Zero};

/// Metadata about an outgoing request
#[derive(codec::Encode, codec::Decode, scale_info::TypeInfo, Clone)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[scale_info(skip_type_params(T))]
pub struct RequestMetadata<T: crate::Config> {
    /// Information about where it's stored in the offchain db
    pub mmr: LeafIndexAndPos,
    /// Other metadata about the request
    pub fee: FeeMetadata<T>,
    /// Has fee been claimed?
    pub claimed: bool,
}

/// This is used for tracking user fee payments for requests
pub type FeeMetadata<T> = dispatcher::FeeMetadata<
    <T as frame_system::Config>::AccountId,
    <T as pallet_balances::Config>::Balance,
>;

/// [`PalletId`] for collecting relayer fees
pub const RELAYER_FEE_ACCOUNT: PalletId = PalletId(*b"ISMPFEES");

impl<T> IsmpDispatcher for Host<T>
where
    T: crate::Config,
{
    type Account = T::AccountId;
    type Balance = T::Balance;

    fn dispatch_request(
        &self,
        request: DispatchRequest,
        fee: FeeMetadata<T>,
    ) -> Result<(), IsmpError> {
        // collect payment for the request
        if fee.fee != Zero::zero() {
            T::Currency::transfer(
                &fee.payer,
                &RELAYER_FEE_ACCOUNT.into_account_truncating(),
                fee.fee,
                ExistenceRequirement::AllowDeath,
            )
            .map_err(|err| {
                IsmpError::ImplementationSpecific(format!(
                    "Error withdrawing request fees: {err:?}"
                ))
            })?;
        }

        let host = Host::<T>::default();
        let request = match request {
            DispatchRequest::Get(dispatch_get) => {
                let get = Get {
                    source: host.host_state_machine(),
                    dest: dispatch_get.dest,
                    nonce: host.next_nonce(),
                    from: dispatch_get.from,
                    keys: dispatch_get.keys,
                    height: dispatch_get.height,
                    timeout_timestamp: if dispatch_get.timeout_timestamp == 0 {
                        0
                    } else {
                        <T::TimestampProvider as UnixTime>::now().as_secs() +
                            dispatch_get.timeout_timestamp
                    },
                };
                Request::Get(get)
            },
            DispatchRequest::Post(dispatch_post) => {
                let post = Post {
                    source: host.host_state_machine(),
                    dest: dispatch_post.dest,
                    nonce: host.next_nonce(),
                    from: dispatch_post.from,
                    to: dispatch_post.to,
                    timeout_timestamp: if dispatch_post.timeout_timestamp == 0 {
                        0
                    } else {
                        <T::TimestampProvider as UnixTime>::now().as_secs() +
                            dispatch_post.timeout_timestamp
                    },
                    data: dispatch_post.data,
                };
                Request::Post(post)
            },
        };

        Pallet::<T>::dispatch_request(request, fee)?;

        Ok(())
    }

    fn dispatch_response(
        &self,
        response: PostResponse,
        fee: FeeMetadata<T>,
    ) -> Result<(), IsmpError> {
        // collect payment for the response
        if fee.fee != Zero::zero() {
            T::Currency::transfer(
                &fee.payer,
                &RELAYER_FEE_ACCOUNT.into_account_truncating(),
                fee.fee,
                ExistenceRequirement::AllowDeath,
            )
            .map_err(|err| {
                IsmpError::ImplementationSpecific(format!(
                    "Error withdrawing request fees: {err:?}"
                ))
            })?;
        }

        let req_commitment = hash_request::<Host<T>>(&response.request());
        if !RequestReceipts::<T>::contains_key(req_commitment) {
            Err(IsmpError::UnknownRequest {
                meta: Meta {
                    source: response.request().source_chain(),
                    dest: response.request().dest_chain(),
                    nonce: response.request().nonce(),
                },
            })?
        }

        let response = Response::Post(response);
        Pallet::<T>::dispatch_response(response, fee)?;

        Ok(())
    }
}
