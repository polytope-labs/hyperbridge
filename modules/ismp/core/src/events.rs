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

//! Event definitions

use crate::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
	router::{GetRequest, GetResponse, PostRequest, PostResponse, Request, Response},
};
use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode};
use primitive_types::H256;
use scale_info::TypeInfo;

/// Emitted when a state machine is successfully updated to a new height after the challenge period
/// has elapsed
#[derive(Clone, Debug, TypeInfo, Encode, Decode, serde::Deserialize, serde::Serialize)]
pub struct StateMachineUpdated {
	/// State machine id
	pub state_machine_id: StateMachineId,
	/// Latest height
	pub latest_height: u64,
}

/// Emitted when a `StateCommitment` has been successfully vetoed by a fisherman
#[derive(
	Clone,
	Debug,
	TypeInfo,
	Encode,
	Decode,
	DecodeWithMemTracking,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct StateCommitmentVetoed {
	/// The state commitment identifier
	pub height: StateMachineHeight,
	/// The account responsible
	#[serde(with = "serde_hex_utils::as_hex")]
	pub fisherman: Vec<u8>,
}

/// Emitted when a request or response is successfully handled.
#[derive(
	Clone,
	Debug,
	TypeInfo,
	Encode,
	Decode,
	DecodeWithMemTracking,
	serde::Deserialize,
	serde::Serialize,
	PartialEq,
	Eq,
)]
pub struct RequestResponseHandled {
	/// The commitment to the request or response
	pub commitment: H256,
	/// The address of the relayer responsible for relaying the request
	#[serde(with = "serde_hex_utils::as_hex")]
	pub relayer: Vec<u8>,
}

/// Emitted when a timeout is successfully handled.
#[derive(
	Clone,
	Debug,
	TypeInfo,
	Encode,
	Decode,
	DecodeWithMemTracking,
	serde::Deserialize,
	serde::Serialize,
	PartialEq,
	Eq,
)]
pub struct TimeoutHandled {
	/// The commitment to the request or response
	pub commitment: H256,
	/// The source chain of the message
	#[serde(with = "serde_hex_utils::as_string")]
	pub source: StateMachine,
	/// The destination chain of the message
	#[serde(with = "serde_hex_utils::as_string")]
	pub dest: StateMachine,
}

/// This represents events that should be emitted by ismp-rs wrappers
#[derive(Clone, Debug, TypeInfo, Encode, Decode, serde::Deserialize, serde::Serialize)]
pub enum Event {
	/// Emitted when a state machine is successfully updated to a new height after the challenge
	/// period has elapsed
	StateMachineUpdated(StateMachineUpdated),
	/// A `StateCommitment` (which is ideally still in it's challenge period) has been vetoed by
	/// a fisherman.
	StateCommitmentVetoed(StateCommitmentVetoed),
	/// An event that is emitted when a post request is dispatched
	PostRequest(PostRequest),
	/// An event that is emitted when a post response is dispatched
	PostResponse(PostResponse),
	/// An event that is emitted when a post response is dispatched
	GetResponse(GetResponse),
	/// An event that is emitted when a get request is dispatched
	GetRequest(GetRequest),
	/// Emitted when a post request is handled
	PostRequestHandled(RequestResponseHandled),
	/// Emitted when a post response is handled
	PostResponseHandled(RequestResponseHandled),
	/// Emitted when a post request timeout is handled
	PostRequestTimeoutHandled(TimeoutHandled),
	/// Emitted when a post response timeout is handled
	PostResponseTimeoutHandled(TimeoutHandled),
	/// Emitted when a get request is handled
	GetRequestHandled(RequestResponseHandled),
	/// Emitted when a get request timeout is handled
	GetRequestTimeoutHandled(TimeoutHandled),
}

/// Minimal version of requests and responses
#[derive(
	Clone, Debug, TypeInfo, Encode, Decode, serde::Deserialize, serde::Serialize, PartialEq, Eq,
)]
pub struct Meta {
	/// Request or response source chain
	pub source: StateMachine,
	/// Request or response dest chain
	pub dest: StateMachine,
	/// Request  nonce
	pub nonce: u64,
}

impl From<&Request> for Meta {
	fn from(value: &Request) -> Self {
		Self { source: value.source_chain(), dest: value.dest_chain(), nonce: value.nonce() }
	}
}

impl From<&Response> for Meta {
	fn from(value: &Response) -> Self {
		Self { source: value.source_chain(), dest: value.dest_chain(), nonce: value.nonce() }
	}
}

impl From<&PostResponse> for Meta {
	fn from(value: &PostResponse) -> Self {
		Self { source: value.source_chain(), dest: value.dest_chain(), nonce: value.nonce() }
	}
}

impl From<Request> for Meta {
	fn from(value: Request) -> Self {
		Self { source: value.source_chain(), dest: value.dest_chain(), nonce: value.nonce() }
	}
}

impl From<Response> for Meta {
	fn from(value: Response) -> Self {
		Self { source: value.source_chain(), dest: value.dest_chain(), nonce: value.nonce() }
	}
}
