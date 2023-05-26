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

use crate::{error::Error, host::StateMachine, prelude::Vec};
use alloc::string::{String, ToString};
use codec::{Decode, Encode};
use core::time::Duration;

/// The ISMP POST request.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Post {
    /// The source state machine of this request.
    pub source_chain: StateMachine,
    /// The destination state machine of this request.
    pub dest_chain: StateMachine,
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
}

/// The ISMP GET request.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Get {
    /// The source state machine of this request.
    pub source_chain: StateMachine,
    /// The destination state machine of this request.
    pub dest_chain: StateMachine,
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
    pub keys: Vec<Vec<u8>>,
    /// Height at which to read the state machine.
    pub height: u64,
    /// Host timestamp at which this request expires in seconds
    pub timeout_timestamp: u64,
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
            Request::Get(get) => get.source_chain,
            Request::Post(post) => post.source_chain,
        }
    }

    /// Get the destination chain
    pub fn dest_chain(&self) -> StateMachine {
        match self {
            Request::Get(get) => get.dest_chain,
            Request::Post(post) => post.dest_chain,
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
        match self {
            Request::Post(post) => Duration::from_secs(post.timeout_timestamp),
            Request::Get(get) => Duration::from_secs(get.timeout_timestamp),
        }
    }

    /// Returns true if the destination chain timestamp has exceeded the request timeout timestamp
    pub fn timed_out(&self, proof_timestamp: Duration) -> bool {
        proof_timestamp >= self.timeout()
    }

    /// Returns a get request or an error
    pub fn get_request(&self) -> Result<Get, Error> {
        match self {
            Request::Post(_) => {
                Err(Error::ImplementationSpecific("Expected Get request".to_string()))
            }
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

/// The ISMP response
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum Response {
    /// The response to a POST request
    Post(PostResponse),
    /// The response to a GET request
    Get {
        /// The Get request that triggered this response.
        get: Get,
        /// Values derived from the state proof
        values: Vec<(Vec<u8>, Option<Vec<u8>>)>,
    },
}

impl Response {
    /// Return the underlying request in the response
    pub fn request(&self) -> Request {
        match self {
            Response::Post(res) => Request::Post(res.post.clone()),
            Response::Get { get, .. } => Request::Get(get.clone()),
        }
    }

    /// Get the source chain for this response
    pub fn source_chain(&self) -> StateMachine {
        match self {
            Response::Get { get, .. } => get.dest_chain,
            Response::Post(res) => res.post.dest_chain,
        }
    }

    /// Get the destination chain for this response
    pub fn dest_chain(&self) -> StateMachine {
        match self {
            Response::Get { get, .. } => get.source_chain,
            Response::Post(res) => res.post.source_chain,
        }
    }

    /// Get the request nonce
    pub fn nonce(&self) -> u64 {
        match self {
            Response::Get { get, .. } => get.nonce,
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

/// The result of successfully dispatching a request or response
#[derive(Debug, PartialEq, Eq)]
pub struct DispatchSuccess {
    /// Destination chain for request or response
    pub dest_chain: StateMachine,
    /// Source chain for request or response
    pub source_chain: StateMachine,
    /// Request nonce
    pub nonce: u64,
}

/// The result of unsuccessfully dispatching a request or response
#[derive(Debug, PartialEq, Eq)]
pub struct DispatchError {
    /// Descriptive error message
    pub msg: String,
    /// Request nonce
    pub nonce: u64,
    /// Source chain for request or response
    pub source: StateMachine,
    /// Destination chain for request or response
    pub dest: StateMachine,
}

/// A type alias for dispatch results
pub type DispatchResult = Result<DispatchSuccess, DispatchError>;

/// The Ismp router dictates how messsages are route to [`IsmpModules`]
pub trait IsmpRouter {
    /// Dispatch some requests to the Ismp router.
    /// For outgoing requests, they should be committed in state as a keccak256 hash
    /// For incoming requests, they should be dispatched to destination modules
    fn handle_request(&self, request: Request) -> DispatchResult;

    /// Dispatch request timeouts to the router which should dispatch them to modules
    fn handle_timeout(&self, request: Request) -> DispatchResult;

    /// Dispatch some responses to the Ismp router.
    /// For incoming responses, they should be dispatched to destination modules
    fn handle_response(&self, response: Response) -> DispatchResult;
}

/// Simplified POST request, intended to be used for sending outgoing requests
#[derive(Clone)]
pub struct DispatchPost {
    /// The destination state machine of this request.
    pub dest_chain: StateMachine,
    /// Module Id of the sending module
    pub from: Vec<u8>,
    /// Module ID of the receiving module
    pub to: Vec<u8>,
    /// Timestamp which this request expires in seconds.
    pub timeout_timestamp: u64,
    /// Encoded Request.
    pub data: Vec<u8>,
}

/// Simplified GET request, intended to be used for sending outgoing requests
#[derive(Clone)]
pub struct DispatchGet {
    /// The destination state machine of this request.
    pub dest_chain: StateMachine,
    /// Module Id of the sending module
    pub from: Vec<u8>,
    /// Raw Storage keys that would be used to fetch the values from the counterparty
    pub keys: Vec<Vec<u8>>,
    /// Height at which to read the state machine.
    pub height: u64,
    /// Host timestamp at which this request expires in seconds
    pub timeout_timestamp: u64,
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
pub trait IsmpDispatcher {
    /// Dispatches an outgoing request, the dispatcher should commit them to host state trie
    fn dispatch_request(&self, request: DispatchRequest) -> DispatchResult;

    /// Dispatches an outgoing response, the dispatcher should commit them to host state trie
    fn dispatch_response(&self, response: PostResponse) -> DispatchResult;
}
