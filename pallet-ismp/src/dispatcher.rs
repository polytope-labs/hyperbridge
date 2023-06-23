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
use crate::{host::Host, Config, Event, Pallet, RequestCommitments, ResponseCommitments};
use alloc::string::ToString;
use codec::{Decode, Encode};
use core::marker::PhantomData;
use ismp_primitives::{mmr::Leaf, LeafIndexQuery};
use ismp_rs::{
    error::Error as IsmpError,
    host::IsmpHost,
    router::{DispatchRequest, Get, IsmpDispatcher, Post, PostResponse, Request, Response},
    util::{hash_request, hash_response},
};
use sp_core::H256;

/// A receipt or an outgoing or incoming request or response
#[derive(Encode, Decode, scale_info::TypeInfo)]
pub enum Receipt {
    /// Ok
    Ok,
}

/// The dispatcher commits outgoing requests and responses to the mmr
pub struct Dispatcher<T>(PhantomData<T>);

impl<T> Default for Dispatcher<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> IsmpDispatcher for Dispatcher<T>
where
    T: Config,
    <T as frame_system::Config>::Hash: From<H256>,
{
    fn dispatch_request(&self, request: DispatchRequest) -> Result<(), IsmpError> {
        let host = Host::<T>::default();
        let request = match request {
            DispatchRequest::Get(dispatch_get) => {
                let get = Get {
                    source_chain: host.host_state_machine(),
                    dest_chain: dispatch_get.dest_chain,
                    nonce: host.next_nonce(),
                    from: dispatch_get.from,
                    keys: dispatch_get.keys,
                    height: dispatch_get.height,
                    timeout_timestamp: dispatch_get.timeout_timestamp,
                };
                Request::Get(get)
            }
            DispatchRequest::Post(dispatch_post) => {
                let post = Post {
                    source_chain: host.host_state_machine(),
                    dest_chain: dispatch_post.dest_chain,
                    nonce: host.next_nonce(),
                    from: dispatch_post.from,
                    to: dispatch_post.to,
                    timeout_timestamp: dispatch_post.timeout_timestamp,
                    data: dispatch_post.data,
                };
                Request::Post(post)
            }
        };

        let commitment = hash_request::<Host<T>>(&request).0.to_vec();

        let (dest_chain, source_chain, nonce) =
            (request.dest_chain(), request.source_chain(), request.nonce());
        Pallet::<T>::mmr_push(Leaf::Request(request)).ok_or_else(|| {
            IsmpError::ImplementationSpecific("Failed to push request into mmr".to_string())
        })?;
        // Deposit Event
        Pallet::<T>::deposit_event(Event::Request {
            request_nonce: nonce,
            source_chain,
            dest_chain,
        });
        // We need this step since it's not trivial to check the mmr for commitments on chain
        RequestCommitments::<T>::insert(
            commitment,
            LeafIndexQuery { source_chain, dest_chain, nonce },
        );
        Ok(())
    }

    fn dispatch_response(&self, response: PostResponse) -> Result<(), IsmpError> {
        let response = Response::Post(response);

        let commitment = hash_response::<Host<T>>(&response).0.to_vec();

        if ResponseCommitments::<T>::contains_key(commitment.clone()) {
            Err(IsmpError::ImplementationSpecific("Duplicate response".to_string()))?
        }

        let (dest_chain, source_chain, nonce) =
            (response.dest_chain(), response.source_chain(), response.nonce());

        Pallet::<T>::mmr_push(Leaf::Response(response)).ok_or_else(|| {
            IsmpError::ImplementationSpecific("Failed to push response into mmr".to_string())
        })?;

        Pallet::<T>::deposit_event(Event::Response {
            request_nonce: nonce,
            dest_chain,
            source_chain,
        });
        ResponseCommitments::<T>::insert(commitment, Receipt::Ok);
        Ok(())
    }
}
