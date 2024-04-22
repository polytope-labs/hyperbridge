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

//! Implementation for the ISMP Router
use crate::{
    child_trie::RequestReceipts, host::Host, mmr::Leaf, primitives::LeafIndexAndPos, Pallet,
};
use alloc::string::ToString;
use codec::{Decode, Encode};
use core::marker::PhantomData;
use frame_support::traits::UnixTime;
use ismp::{
    error::Error as IsmpError,
    host::IsmpHost,
    router::{DispatchRequest, Get, IsmpDispatcher, Post, PostResponse, Request, Response},
    util::hash_request,
};
use sp_core::H256;

/// A receipt or an outgoing or incoming request or response
#[derive(Encode, Decode, scale_info::TypeInfo)]
pub enum Receipt {
    /// Ok
    Ok,
}

/// Queries a request leaf in the mmr
#[derive(codec::Encode, codec::Decode, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[scale_info(skip_type_params(T))]
pub struct LeafMetadata<T: crate::Config> {
    /// Information about where it's stored in the offchain db
    pub mmr: LeafIndexAndPos,
    /// Other metadata about the request
    pub meta: FeeMetadata<T>,
}

/// This is used for tracking user fee payments for requests
#[derive(codec::Encode, codec::Decode, scale_info::TypeInfo)]
#[scale_info(skip_type_params(T))]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct FeeMetadata<T: crate::Config> {
    /// The user who paid for this fee
    pub origin: T::AccountId,
    /// The amount they paid
    pub fee: T::Balance,
}

/// The dispatcher commits outgoing requests and responses to the mmr
/// This dispatcher charges no fees, use only if you intend to self-relay.
pub struct Dispatcher<T>(PhantomData<T>);

impl<T> Default for Dispatcher<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> IsmpDispatcher for Dispatcher<T>
where
    T: crate::Config,
    <T as pallet_mmr::Config>::Leaf: From<Leaf>,
    <<T as pallet_mmr::Config>::Hashing as sp_runtime::traits::Hash>::Output: Into<H256>,
{
    type Account = T::AccountId;
    type Balance = T::Balance;

    fn dispatch_request(
        &self,
        request: DispatchRequest,
        origin: Self::Account,
        fee: Self::Balance,
    ) -> Result<(), IsmpError> {
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
                        <T::TimeProvider as UnixTime>::now().as_secs() +
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
                        <T::TimeProvider as UnixTime>::now().as_secs() +
                            dispatch_post.timeout_timestamp
                    },
                    data: dispatch_post.data,
                };
                Request::Post(post)
            },
        };

        Pallet::<T>::dispatch_request(request, FeeMetadata { origin, fee })?;

        Ok(())
    }

    fn dispatch_response(
        &self,
        response: PostResponse,
        origin: Self::Account,
        fee: Self::Balance,
    ) -> Result<(), IsmpError> {
        let req_commitment = hash_request::<Host<T>>(&response.request());
        if !RequestReceipts::<T>::contains_key(req_commitment) {
            Err(IsmpError::ImplementationSpecific("Unknown request for response".to_string()))?
        }

        let response = Response::Post(response);
        Pallet::<T>::dispatch_response(response, FeeMetadata { origin, fee })?;

        Ok(())
    }
}
