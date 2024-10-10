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

//! Message dispatcher definitions

use crate::{host::StateMachine, router::PostResponse};
use alloc::vec::Vec;
use codec::{Decode, Encode};
use primitive_types::H256;

/// Simplified POST request, intended to be used for sending outgoing requests
#[derive(Clone)]
pub struct DispatchPost {
	/// The destination state machine of this request.
	pub dest: StateMachine,
	/// Module identifier of the sending module
	pub from: Vec<u8>,
	/// Module identifier of the receiving module
	pub to: Vec<u8>,
	/// Relative from the current timestamp at which this request expires in seconds.
	pub timeout: u64,
	/// Encoded request body
	pub body: Vec<u8>,
}

/// Simplified GET request, intended to be used for sending outgoing requests
#[derive(Clone)]
pub struct DispatchGet {
	/// The destination state machine of this request.
	pub dest: StateMachine,
	/// Module identifier of the sending module
	pub from: Vec<u8>,
	/// Raw Storage keys that would be used to fetch the values from the counterparty
	pub keys: Vec<Vec<u8>>,
	/// Height at which to read the state machine.
	pub height: u64,
	/// Some application-specific metadata relating to this request
	pub context: Vec<u8>,
	/// Relative from the current timestamp at which this request expires in seconds.
	pub timeout: u64,
}

/// Simplified request, intended to be used for sending outgoing requests
#[derive(Clone)]
pub enum DispatchRequest {
	/// The POST variant
	Post(DispatchPost),
	/// The GET variant
	Get(DispatchGet),
}

/// Fee metadata for a dispatched request. Contains the account who paid for the request and how
/// much was paid.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	Encode,
	Decode,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	derive_more::From,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct FeeMetadata<A, B> {
	/// The account which paid for this request
	pub payer: A,
	/// The fee that was paid for relayers.
	pub fee: B,
}

/// The Ismp dispatcher provides an [`IsmpModule`](crate::module::IsmpModule) with the required
/// interface for dispatching out outgoing [`Request`](crate::router::Request)s or
/// [`Response`](crate::router::Response)s.
///
/// An [`Event`](crate::events::Event) should be emitted after successful dispatch
pub trait IsmpDispatcher: Default {
	/// Sending account type
	type Account;

	/// The balance type
	type Balance;

	/// Dispatches an outgoing request, the dispatcher may commit the request to host's state trie
	/// or an overlay tree
	///
	/// The account `who` is needed as a way to identify the account which triggered this request.
	/// The `amount`. Returns the request commitment
	fn dispatch_request(
		&self,
		request: DispatchRequest,
		fee: FeeMetadata<Self::Account, Self::Balance>,
	) -> Result<H256, anyhow::Error>;

	/// Dispatches an outgoing response, the dispatcher should commit them to host's state trie or
	/// overlay tree. Returns the response commitment
	fn dispatch_response(
		&self,
		response: PostResponse,
		fee: FeeMetadata<Self::Account, Self::Balance>,
	) -> Result<H256, anyhow::Error>;
}
