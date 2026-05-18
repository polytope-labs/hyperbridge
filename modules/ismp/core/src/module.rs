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
	events::Event,
	router::{GetResponse, PostRequest, Request},
	Error,
};
use sp_weights::Weight;
/// A type alias for dispatch results
pub type DispatchResult = Result<Event, anyhow::Error>;

/// Individual modules which live on a state machine must conform to this interface in order to send
/// and receive ISMP requests and responses
pub trait IsmpModule {
	/// Called by the message handler on a module, to notify module of a new POST request
	/// the module may choose to respond immediately, or in a later block
	fn on_accept(&self, _request: PostRequest) -> Result<Weight, anyhow::Error> {
		Err(Error::CannotHandleMessage)?
	}

	/// Called by the message handler on a module, to notify module of a response to a previously
	/// sent out request
	fn on_response(&self, _response: GetResponse) -> Result<Weight, anyhow::Error> {
		Err(Error::CannotHandleMessage)?
	}

	/// Called by the message handler on a module, to notify module of requests that were previously
	/// sent but have now timed-out.
	///
	/// `meta` carries the host's encoded request metadata, as returned by
	/// [`IsmpHost::delete_request_commitment`]. The commitment is removed
	/// before this callback runs, so post deletion bookkeeping (refunding an
	/// escrowed relayer fee, for example) should decode it from here rather
	/// than reach back into host storage.
	fn on_timeout(&self, _request: Request, _meta: Option<&[u8]>) -> Result<Weight, anyhow::Error> {
		Err(Error::CannotHandleMessage)?
	}
}
