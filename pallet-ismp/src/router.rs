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
use crate::{host::Host, Config, Event, Pallet, RequestAcks, ResponseAcks};
use alloc::{boxed::Box, string::ToString};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use ismp_primitives::mmr::Leaf;
use ismp_rs::{
    host::IsmpHost,
    router::{DispatchError, DispatchResult, DispatchSuccess, IsmpRouter, Request, Response},
    util::{hash_request, hash_response},
};
use sp_core::H256;

/// A receipt or an outgoing or incoming request or response
#[derive(Encode, Decode, scale_info::TypeInfo)]
pub enum Receipt {
    /// Ok
    Ok,
}

/// The proxy router, This router allows for routing requests & responses from a source chain
/// to a destination chain.
pub struct ProxyRouter<T> {
    /// Module router
    inner: Option<Box<dyn IsmpRouter>>,
    /// Phantom
    _phantom: PhantomData<T>,
}

impl<T> ProxyRouter<T> {
    /// Initialize the proxy router with an inner router.
    pub fn new<R>(router: R) -> Self
    where
        R: IsmpRouter + 'static,
    {
        Self { inner: Some(Box::new(router)), _phantom: PhantomData }
    }
}

impl<T> Default for ProxyRouter<T> {
    fn default() -> Self {
        Self { inner: None, _phantom: PhantomData }
    }
}

impl<T> IsmpRouter for ProxyRouter<T>
where
    T: Config,
    <T as frame_system::Config>::Hash: From<H256>,
{
    fn dispatch(&self, request: Request) -> DispatchResult {
        let host = Host::<T>::default();

        if host.host_state_machine() != request.dest_chain() {
            let commitment = hash_request::<Host<T>>(&request).0.to_vec();

            if RequestAcks::<T>::contains_key(commitment.clone()) {
                Err(DispatchError {
                    msg: "Duplicate request".to_string(),
                    nonce: request.nonce(),
                    source: request.source_chain(),
                    dest: request.dest_chain(),
                })?
            }

            let (dest_chain, source_chain, nonce) =
                (request.dest_chain(), request.source_chain(), request.nonce());
            Pallet::<T>::mmr_push(Leaf::Request(request)).ok_or_else(|| DispatchError {
                msg: "Failed to push request into mmr".to_string(),
                nonce,
                source: source_chain,
                dest: dest_chain,
            })?;
            // Deposit Event
            Pallet::<T>::deposit_event(Event::Request {
                request_nonce: nonce,
                source_chain,
                dest_chain,
            });
            // We have this step because we can't delete leaves from the mmr
            // So this helps us prevent processing of duplicate outgoing requests
            RequestAcks::<T>::insert(commitment, Receipt::Ok);
            Ok(DispatchSuccess { dest_chain, source_chain, nonce })
        } else if let Some(ref router) = self.inner {
            router.dispatch(request)
        } else {
            Err(DispatchError {
                msg: "Missing a module router".to_string(),
                nonce: request.nonce(),
                source: request.source_chain(),
                dest: request.dest_chain(),
            })?
        }
    }

    fn dispatch_timeout(&self, request: Request) -> DispatchResult {
        if let Some(ref router) = self.inner {
            router.dispatch(request)
        } else {
            Err(DispatchError {
                msg: "Missing a module router".to_string(),
                nonce: request.nonce(),
                source: request.source_chain(),
                dest: request.dest_chain(),
            })?
        }
    }

    fn write_response(&self, response: Response) -> DispatchResult {
        let host = Host::<T>::default();

        if host.host_state_machine() != response.dest_chain() {
            let commitment = hash_response::<Host<T>>(&response).0.to_vec();

            if ResponseAcks::<T>::contains_key(commitment.clone()) {
                Err(DispatchError {
                    msg: "Duplicate response".to_string(),
                    nonce: response.nonce(),
                    source: response.source_chain(),
                    dest: response.dest_chain(),
                })?
            }

            let (dest_chain, source_chain, nonce) =
                (response.dest_chain(), response.source_chain(), response.nonce());

            Pallet::<T>::mmr_push(Leaf::Response(response)).ok_or_else(|| DispatchError {
                msg: "Failed to push response into mmr".to_string(),
                nonce,
                source: source_chain,
                dest: dest_chain,
            })?;

            Pallet::<T>::deposit_event(Event::Response {
                request_nonce: nonce,
                dest_chain,
                source_chain,
            });
            ResponseAcks::<T>::insert(commitment, Receipt::Ok);
            Ok(DispatchSuccess { dest_chain, source_chain, nonce })
        } else if let Some(ref router) = self.inner {
            router.write_response(response)
        } else {
            Err(DispatchError {
                msg: "Missing a module router".to_string(),
                nonce: response.nonce(),
                source: response.source_chain(),
                dest: response.dest_chain(),
            })?
        }
    }
}
