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

//! IsmpRouter definition

use crate::{error::Error, host::StateMachine, module::IsmpModule, prelude::Vec};
use alloc::{boxed::Box, collections::BTreeMap, string::ToString};
use codec::{Decode, Encode};
use core::time::Duration;

/// The ISMP POST request.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Post {
    /// The source state machine of this request.
    pub source: StateMachine,
    /// The destination state machine of this request.
    pub dest: StateMachine,
    /// The nonce of this request on the source chain
    pub nonce: u64,
    /// Module Id of the sending module
    pub from: Vec<u8>,
    /// Module ID of the receiving module
    pub to: Vec<u8>,
    /// Timestamp which this request expires in seconds.
    pub timeout_timestamp: u64,
    /// Encoded Request.
    pub data: Vec<u8>,
    /// Gas limit for executing the request on destination
    /// This value should be zero if destination module is not a contract
    pub gas_limit: u64,
}

/// The ISMP GET request.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Get {
    /// The source state machine of this request.
    pub source: StateMachine,
    /// The destination state machine of this request.
    pub dest: StateMachine,
    /// The nonce of this request on the source chain
    pub nonce: u64,
    /// Module Id of the sending module
    pub from: Vec<u8>,
    /// Raw Storage keys that would be used to fetch the values from the counterparty
    /// For deriving storage keys for ink contract fields follow the guide in the link below
    /// https://use.ink/datastructures/storage-in-metadata#a-full-example
    /// The algorithms for calculating raw storage keys for different substrate pallet storage
    /// types are described in the following links
    /// https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/map.rs#L34-L42
    /// https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/double_map.rs#L34-L44
    /// https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/nmap.rs#L39-L48
    /// https://github.com/paritytech/substrate/blob/master/frame/support/src/storage/types/value.rs#L37
    /// For fetching keys from EVM contracts each key should be 52 bytes
    /// This should be a concatenation of contract address and slot hash
    pub keys: Vec<Vec<u8>>,
    /// Height at which to read the state machine.
    pub height: u64,
    /// Host timestamp at which this request expires in seconds
    pub timeout_timestamp: u64,
    /// Gas limit for executing the response to this get request
    /// This value should be zero if the sending module is not a contract
    pub gas_limit: u64,
}

/// The ISMP request.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum Request {
    /// A post request allows a module on a state machine to send arbitrary bytes to another module
    /// living in another state machine.
    Post(Post),
    /// A get request allows a module on a state machine to read the storage of another module
    /// living in another state machine.
    Get(Get),
}

impl Request {
    /// Get the source chain
    pub fn source_chain(&self) -> StateMachine {
        match self {
            Request::Get(get) => get.source,
            Request::Post(post) => post.source,
        }
    }

    /// Module where this request originated on source chain
    pub fn source_module(&self) -> Vec<u8> {
        match self {
            Request::Get(get) => get.from.clone(),
            Request::Post(post) => post.from.clone(),
        }
    }

    /// Module that this request will be routed to on destination chain
    pub fn destination_module(&self) -> Vec<u8> {
        match self {
            Request::Get(get) => get.from.clone(),
            Request::Post(post) => post.to.clone(),
        }
    }

    /// Get the destination chain
    pub fn dest_chain(&self) -> StateMachine {
        match self {
            Request::Get(get) => get.dest,
            Request::Post(post) => post.dest,
        }
    }

    /// Get the request nonce
    pub fn nonce(&self) -> u64 {
        match self {
            Request::Get(get) => get.nonce,
            Request::Post(post) => post.nonce,
        }
    }

    /// Get the POST request data
    pub fn data(&self) -> Option<Vec<u8>> {
        match self {
            Request::Get(_) => None,
            Request::Post(post) => Some(post.data.clone()),
        }
    }

    /// Get the GET request keys.
    pub fn keys(&self) -> Option<Vec<Vec<u8>>> {
        match self {
            Request::Post(_) => None,
            Request::Get(get) => Some(get.keys.clone()),
        }
    }

    /// Returns the timeout timestamp for a request
    pub fn timeout(&self) -> Duration {
        let timeout = match self {
            Request::Post(post) => post.timeout_timestamp,
            Request::Get(get) => get.timeout_timestamp,
        };

        // zero timeout means no timeout.
        if timeout == 0 {
            Duration::from_secs(u64::MAX)
        } else {
            Duration::from_secs(timeout)
        }
    }

    /// Returns true if the destination chain timestamp has exceeded the request timeout timestamp
    pub fn timed_out(&self, proof_timestamp: Duration) -> bool {
        proof_timestamp >= self.timeout()
    }

    /// Returns a get request or an error
    pub fn get_request(&self) -> Result<Get, Error> {
        match self {
            Request::Post(_) =>
                Err(Error::ImplementationSpecific("Expected Get request".to_string())),
            Request::Get(get) => Ok(get.clone()),
        }
    }

    /// Returns true if request is a get request
    pub fn is_type_get(&self) -> bool {
        match self {
            Request::Post(_) => false,
            Request::Get(_) => true,
        }
    }
}

/// The response to a POST request
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct PostResponse {
    /// The request that triggered this response.
    pub post: Post,
    /// The response message.
    pub response: Vec<u8>,
}

/// The response to a POST request
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct GetResponse {
    /// The Get request that triggered this response.
    pub get: Get,
    /// Values derived from the state proof
    pub values: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
}

/// The ISMP response
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum Response {
    /// The response to a POST request
    Post(PostResponse),
    /// The response to a GET request
    Get(GetResponse),
}

impl Response {
    /// Return the underlying request in the response
    pub fn request(&self) -> Request {
        match self {
            Response::Post(res) => Request::Post(res.post.clone()),
            Response::Get(res) => Request::Get(res.get.clone()),
        }
    }

    /// Module that this response will be routed to on destination chain
    pub fn destination_module(&self) -> Vec<u8> {
        match self {
            Response::Get(get) => get.get.from.clone(),
            Response::Post(post) => post.post.from.clone(),
        }
    }

    /// Get the source chain for this response
    pub fn source_chain(&self) -> StateMachine {
        match self {
            Response::Get(res) => res.get.dest,
            Response::Post(res) => res.post.dest,
        }
    }

    /// Get the destination chain for this response
    pub fn dest_chain(&self) -> StateMachine {
        match self {
            Response::Get(res) => res.get.source,
            Response::Post(res) => res.post.source,
        }
    }

    /// Get the request nonce
    pub fn nonce(&self) -> u64 {
        match self {
            Response::Get(res) => res.get.nonce,
            Response::Post(res) => res.post.nonce,
        }
    }
}

/// Convenience enum for membership verification.
pub enum RequestResponse {
    /// A batch of requests
    Request(Vec<Request>),
    /// A batch of responses
    Response(Vec<Response>),
}

/// The Ismp router dictates how messsages are routed to [`IsmpModules`]
pub trait IsmpRouter {
    /// Get module handler by id
    /// Should decode the module id and return a handler to the appropriate `IsmpModule`
    /// implementation
    fn module_for_id(&self, bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error>;
}

/// Simplified POST request, intended to be used for sending outgoing requests
#[derive(Clone)]
pub struct DispatchPost {
    /// The destination state machine of this request.
    pub dest: StateMachine,
    /// Module Id of the sending module
    pub from: Vec<u8>,
    /// Module ID of the receiving module
    pub to: Vec<u8>,
    /// Timestamp which this request expires in seconds.
    pub timeout_timestamp: u64,
    /// Encoded Request.
    pub data: Vec<u8>,
    /// Gas limit for executing request on destination chain
    /// This should be zero if the destination module is not a contract
    pub gas_limit: u64,
}

/// Simplified GET request, intended to be used for sending outgoing requests
#[derive(Clone)]
pub struct DispatchGet {
    /// The destination state machine of this request.
    pub dest: StateMachine,
    /// Module Id of the sending module
    pub from: Vec<u8>,
    /// Raw Storage keys that would be used to fetch the values from the counterparty
    pub keys: Vec<Vec<u8>>,
    /// Height at which to read the state machine.
    pub height: u64,
    /// Host timestamp at which this request expires in seconds
    pub timeout_timestamp: u64,
    /// Gas limit for executing the response to this get request
    /// This value should be zero if the dispatching module is not a contract
    pub gas_limit: u64,
}

/// Simplified request, intended to be used for sending outgoing requests
#[derive(Clone)]
pub enum DispatchRequest {
    /// The POST variant
    Post(DispatchPost),
    /// The GET variant
    Get(DispatchGet),
}

/// The Ismp dispatcher allows [`IsmpModules`] to send out outgoing [`Request`] or [`Response`]
/// [`Event`] should be emitted after successful dispatch
pub trait IsmpDispatcher {
    /// Dispatches an outgoing request, the dispatcher should commit them to host state trie
    fn dispatch_request(&self, request: DispatchRequest) -> Result<(), Error>;

    /// Dispatches an outgoing response, the dispatcher should commit them to host state trie
    fn dispatch_response(&self, response: PostResponse) -> Result<(), Error>;
}
