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

//! ISMPModule definition

use crate::{
    error::Error,
    host::StateMachine,
    router::{Post as PostRequest, Request, Response},
};
use alloc::string::String;

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
    pub source_chain: StateMachine,
    /// Destination chain for request or response
    pub dest_chain: StateMachine,
}

/// A type alias for dispatch results
pub type DispatchResult = Result<DispatchSuccess, DispatchError>;

/// Individual modules which live on a state machine must conform to this interface in order to send
/// and receive ISMP requests and responses
pub trait IsmpModule {
    /// Called by the message handler on a module, to notify module of a new POST request
    /// the module may choose to respond immediately, or in a later block
    fn on_accept(&self, request: PostRequest) -> Result<(), Error>;

    /// Called by the message handler on a module, to notify module of a response to a previously
    /// sent out request
    fn on_response(&self, response: Response) -> Result<(), Error>;

    /// Called by the message handler on a module, to notify module of requests that were previously
    /// sent but have now timed-out
    fn on_timeout(&self, request: Request) -> Result<(), Error>;
}
