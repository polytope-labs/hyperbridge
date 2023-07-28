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
use crate::{host::Host, Config, Pallet};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use ismp_rs::{
    error::Error as IsmpError,
    host::IsmpHost,
    router::{DispatchRequest, Get, IsmpDispatcher, Post, PostResponse, Request, Response},
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
                    source: host.host_state_machine(),
                    dest: dispatch_get.dest,
                    nonce: host.next_nonce(),
                    from: dispatch_get.from,
                    keys: dispatch_get.keys,
                    height: dispatch_get.height,
                    timeout_timestamp: dispatch_get.timeout_timestamp,
                    gas_limit: dispatch_get.gas_limit,
                };
                Request::Get(get)
            }
            DispatchRequest::Post(dispatch_post) => {
                let post = Post {
                    source: host.host_state_machine(),
                    dest: dispatch_post.dest,
                    nonce: host.next_nonce(),
                    from: dispatch_post.from,
                    to: dispatch_post.to,
                    timeout_timestamp: dispatch_post.timeout_timestamp,
                    data: dispatch_post.data,
                    gas_limit: dispatch_post.gas_limit,
                };
                Request::Post(post)
            }
        };

        Pallet::<T>::dispatch_request(request)?;

        Ok(())
    }

    fn dispatch_response(&self, response: PostResponse) -> Result<(), IsmpError> {
        let response = Response::Post(response);

        Pallet::<T>::dispatch_response(response)?;

        Ok(())
    }
}
