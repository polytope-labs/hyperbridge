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

//! ISMPRouter definition

use crate::{consensus_client::StateMachineHeight, error::Error, host::ChainID, prelude::Vec};
use codec::{Decode, Encode};
use core::time::Duration;

/// The ISMP POST request.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Post {
    /// The source state machine of this request.
    pub source_chain: ChainID,
    /// The destination state machine of this request.
    pub dest_chain: ChainID,
    /// The nonce of this request on the source chain
    pub nonce: u64,
    /// Moudle Id of the sending module
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
    pub source_chain: ChainID,
    /// The destination state machine of this request.
    pub dest_chain: ChainID,
    /// The nonce of this request on the source chain
    pub nonce: u64,
    /// Moudle Id of the sending module
    pub from: Vec<u8>,
    /// Storage keys that this request is interested in.
    pub keys: Vec<Vec<u8>>,
    /// Height at which to read the state machine.
    pub height: StateMachineHeight,
    /// Timestamp which this request expires in seconds
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
    pub fn source_chain(&self) -> ChainID {
        match self {
            Request::Get(get) => get.source_chain,
            Request::Post(post) => post.source_chain,
        }
    }

    /// Get the destination chain
    pub fn dest_chain(&self) -> ChainID {
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
        proof_timestamp > self.timeout()
    }
}

/// The ISMP response
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Response {
    /// The request that triggered this response.
    pub request: Request,
    /// The response message.
    pub response: Vec<u8>,
}

/// This is the concrete type for Get requests
pub type GetResponse = Vec<(Vec<u8>, Vec<u8>)>;

/// Convenience enum for membership verification.
pub enum RequestResponse {
    Request(Request),
    Response(Response),
}

pub trait ISMPRouter {
    /// Dispatch a request from a module to the ISMP router.
    /// If request source chain is the host, it should be committed in state as a sha256 hash
    fn dispatch(&self, request: Request) -> Result<(), Error>;

    /// Dispatch a request timeout from a module to the ISMP router.
    /// If request source chain is the host, it should be committed in state as a sha256 hash
    fn dispatch_timeout(&self, request: Request) -> Result<(), Error>;

    /// Provide a response to a previously received request.
    /// If response source chain is the host, it should be committed in state as a sha256 hash
    fn write_response(&self, response: Response) -> Result<(), Error>;
}
