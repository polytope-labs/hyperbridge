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

//! ISMP message types

// Messages are processed in batches, all messages in a batch should
// originate from the same chain

use alloc::collections::BTreeMap;

use crate::{
	consensus::{
		ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId,
	},
	error::Error,
	host::StateMachine,
	router::{GetResponse, PostRequest, PostResponse, Request, RequestResponse, Response},
};
use alloc::{string::ToString, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode};
use primitive_types::H256;
use sp_weights::Weight;

/// A consensus message is used to update the state of a consensus client and its children state
/// machines.
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct ConsensusMessage {
	/// Scale Encoded Consensus Proof
	pub consensus_proof: Vec<u8>,
	/// The consensus state Id
	pub consensus_state_id: ConsensusStateId,
	/// Public key of the sender
	pub signer: Vec<u8>,
}

/// A fraud proof message is used to report byzantine misbehaviour in a consensus system.
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct FraudProofMessage {
	/// The first consensus Proof
	pub proof_1: Vec<u8>,
	/// The second consensus Proof
	pub proof_2: Vec<u8>,
	/// The consensus state Id
	pub consensus_state_id: ConsensusStateId,
	/// Public key of the sender
	pub signer: Vec<u8>,
}

/// Identifies a state commitment at a given height
#[derive(
	Debug,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct StateCommitmentHeight {
	/// The state machine identifier
	pub commitment: StateCommitment,
	/// the corresponding block height
	pub height: u64,
}

/// Used for creating the initial consensus state for a given consensus client.
#[derive(
	Debug,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct CreateConsensusState {
	/// Scale encoded consensus state
	#[serde(with = "serde_hex_utils::as_hex")]
	pub consensus_state: Vec<u8>,
	/// Consensus client id
	#[serde(with = "serde_hex_utils::as_utf8_string")]
	pub consensus_client_id: ConsensusClientId,
	/// The consensus state Id
	#[serde(with = "serde_hex_utils::as_utf8_string")]
	pub consensus_state_id: ConsensusStateId,
	/// Unbonding period for this consensus state.
	pub unbonding_period: u64,
	/// Challenge period for the supported state machines
	pub challenge_periods: BTreeMap<StateMachine, u64>,
	/// State machine commitments
	pub state_machine_commitments: Vec<(StateMachineId, StateCommitmentHeight)>,
}

/// A request message holds a batch of requests to be dispatched from a source state machine
#[derive(
	Debug, Clone, Encode, DecodeWithMemTracking, Decode, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct RequestMessage {
	/// Requests from source chain
	pub requests: Vec<PostRequest>,
	/// Membership batch proof for these requests
	pub proof: Proof,
	/// Signer information. Ideally should be their account identifier
	pub signer: Vec<u8>,
}

/// A request message holds a batch of responses to be dispatched from a source state machine
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct ResponseMessage {
	/// A set of either POST requests or responses to be handled
	pub datagram: RequestResponse,
	/// Membership batch proof for these req/res
	pub proof: Proof,
	/// Signer information. Ideally should be their account identifier
	pub signer: Vec<u8>,
}

impl ResponseMessage {
	/// Returns the requests in this message.
	pub fn requests(&self) -> Vec<Request> {
		match &self.datagram {
			RequestResponse::Response(responses) =>
				responses.iter().map(|res| res.request()).collect(),
			RequestResponse::Request(requests) => requests.clone(),
		}
	}

	/// Retuns the associated proof
	pub fn proof(&self) -> &Proof {
		&self.proof
	}
}

/// A timeout message holds a batch of messages to be timed-out
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub enum TimeoutMessage {
	/// A non memership proof for POST requests
	Post {
		/// Request timeouts
		requests: Vec<Request>,
		/// Non membership batch proof for these requests
		timeout_proof: Proof,
	},
	/// A non memership proof for POST requests
	PostResponse {
		/// Request timeouts
		responses: Vec<PostResponse>,
		/// Non membership batch proof for these requests
		timeout_proof: Proof,
	},
	/// There are no proofs for Get timeouts, we only need to
	/// ensure that the timeout timestamp has elapsed on the host
	Get {
		/// Requests that have timed out
		requests: Vec<Request>,
	},
}

impl TimeoutMessage {
	/// Get all the inner requests
	pub fn requests(&self) -> Vec<Request> {
		match self {
			TimeoutMessage::Post { requests, .. } | TimeoutMessage::Get { requests, .. } =>
				requests.clone(),
			TimeoutMessage::PostResponse { responses, .. } =>
				responses.clone().into_iter().map(|res| res.request()).collect(),
		}
	}
	/// Returns the associated proof
	pub fn timeout_proof(&self) -> Result<&Proof, Error> {
		match self {
			TimeoutMessage::Post { timeout_proof, .. } => Ok(timeout_proof),
			_ => Err(Error::Custom("Method should not be called on Get request".to_string())),
		}
	}
}

/// Proof holds the relevant proof data for the context in which it's used.
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Proof {
	/// State machine height
	pub height: StateMachineHeight,
	/// Scale encoded proof
	pub proof: Vec<u8>,
}

/// The Overaching ISMP message type.
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub enum Message {
	/// A consensus update message
	#[codec(index = 0)]
	Consensus(ConsensusMessage),
	/// A fraud proof message
	#[codec(index = 1)]
	FraudProof(FraudProofMessage),
	/// A request message
	#[codec(index = 2)]
	Request(RequestMessage),
	/// A response message
	#[codec(index = 3)]
	Response(ResponseMessage),
	/// A request timeout message
	#[codec(index = 4)]
	Timeout(TimeoutMessage),
}

/// The ISMP Message with Weight consumed by the message
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct MessageWithWeight {
	/// The message itself
	pub message: Message,
	/// The weight consumed by the message
	pub weight: Weight,
}

/// A trait that returns a 256 bit keccak has of some bytes
pub trait Keccak256 {
	/// Returns a keccak256 hash of a byte slice
	fn keccak256(bytes: &[u8]) -> H256
	where
		Self: Sized;
}

/// Return the keccak256 hash of a request
pub fn hash_request<H: Keccak256>(req: &Request) -> H256 {
	let encoded = req.encode();
	H::keccak256(&encoded)
}

/// Return the keccak256 of a response
pub fn hash_response<H: Keccak256>(res: &Response) -> H256 {
	match res {
		Response::Post(res) => hash_post_response::<H>(res),
		Response::Get(res) => hash_get_response::<H>(res),
	}
}

/// Return the keccak256 of a post response
pub fn hash_post_response<H: Keccak256>(res: &PostResponse) -> H256 {
	H::keccak256(&res.encode())
}

/// Return the keccak256 of a get response
pub fn hash_get_response<H: Keccak256>(res: &GetResponse) -> H256 {
	H::keccak256(&res.encode())
}
