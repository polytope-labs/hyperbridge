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

//! ISMP error definitions
#![allow(unused)]

use crate::{
	consensus::{ConsensusClientId, ConsensusStateId, StateMachineHeight, StateMachineId},
	events::Meta,
};
use alloc::{
	string::{String, ToString},
	vec::Vec,
};
use codec::{Decode, Encode};
use core::time::Duration;
use scale_info::TypeInfo;

/// Errors that may be encountered by the ISMP module
#[derive(Debug, Eq, PartialEq, Encode, Decode, TypeInfo, displaydoc::Display, thiserror::Error)]
pub enum Error {
	/**
	 * The unbonding period for the given consensus client has elapsed and can no longer
	 * process consensus updates.
	 */
	UnbondingPeriodElapsed {
		/// The consensus client identifier
		consensus_state_id: ConsensusStateId,
	},
	/**
	 * The challange period for the given consensus client has not yet elapsed and cannot
	 * process new consensus updates in the mean time.
	 */
	ChallengePeriodNotElapsed {
		/// The consensus client identifier
		state_machine_id: StateMachineId,
		/// The last time the consensus client was updated
		update_time: Duration,
		/// The current time
		current_time: Duration,
	},
	/// A consensus state was not found for the given consensus client.
	ConsensusStateNotFound {
		/// The consensus client identifier
		consensus_state_id: ConsensusStateId,
	},
	/// A state commitment was not found for the given consensus client.
	StateCommitmentNotFound {
		/// The given state machine height
		height: StateMachineHeight,
	},
	/// The given consensus client has been frozen
	FrozenConsensusClient {
		/// The consensus client identifier
		consensus_state_id: ConsensusStateId,
	},
	/// The given request was not found
	RequestCommitmentNotFound {
		/// The request metadata
		meta: Meta,
	},
	/// The given request has failed state proof verification
	RequestVerificationFailed {
		/// The request metadata
		meta: Meta,
	},
	/// The given request has not yet timed-out
	RequestTimeoutNotElapsed {
		/// The request metadata
		meta: Meta,
		/// The timestamp at which the timeout elapses
		timeout_timestamp: Duration,
		/// The current time on the state machine
		state_machine_time: Duration,
	},
	/// The given request has failed non-membership state proof verification
	RequestTimeoutVerificationFailed {
		/// The request metadata
		meta: Meta,
	},
	/// The given response has failed membership state proof verification
	ResponseVerificationFailed {
		/// The response metadata
		meta: Meta,
	},
	/// Failed to verify the consensus proof for the given consensus client
	ConsensusProofVerificationFailed {
		/// The consensus client identifier
		id: ConsensusClientId,
	},
	/// The given consensus client has expired
	ExpiredConsensusClient {
		/// The consensus client identifier
		id: ConsensusClientId,
	},
	/// Cannot handle the given message
	CannotHandleMessage,
	/// Membership proof verification failed: {0}
	MembershipProofVerificationFailed(String),
	/// Non-membership proof verification failed: {0}
	NonMembershipProofVerificationFailed(String),
	/// Custom error: {0}
	Custom(String),
	/// A consensus client with the given identifier already exists
	CannotCreateAlreadyExistingConsensusClient {
		/// The consensus client identifier
		id: ConsensusClientId,
	},
	/// Supplied proof height is invalid
	InsufficientProofHeight,
	/// An Ismp Module was not found for the given raw id
	ModuleNotFound(Vec<u8>),
	/// Unknown consensus state id
	ConsensusStateIdNotRecognized {
		/// Consensus state Id
		consensus_state_id: ConsensusStateId,
	},

	/// Challenge period has not been configured for this consensus state
	ChallengePeriodNotConfigured {
		/// State Machine Id
		state_machine: StateMachineId,
	},
	/// Consensus state id already exists
	DuplicateConsensusStateId {
		/// Consensus state Id
		consensus_state_id: ConsensusStateId,
	},
	/// Unbonding period has not been configured for this consensus state
	UnnbondingPeriodNotConfigured {
		/// Consensus state Id
		consensus_state_id: ConsensusStateId,
	},
	/// Error from dispatching a request to a module
	ModuleDispatchError {
		/// Descriptive error message
		msg: String,
		/// the request metadata
		meta: Meta,
	},
	/// Attempted to respond to/timeout an unknown request
	UnknownRequest {
		/// Unknown request metadata
		meta: Meta,
	},
	/// Attempted to time-out an unknown response
	UnknownResponse {
		/// Unknown response metadata
		meta: Meta,
	},
	/// Request commitment for a response does not exist
	UnsolicitedResponse {
		/// Unsolicited response metadata
		meta: Meta,
	},
	/// Timed out request found in batch
	RequestTimeout {
		/// Timed out Request metadata
		meta: Meta,
	},
	/// Timed out response found in batch
	ResponseTimeout {
		/// Timed out Response metadata
		response: Meta,
	},
	/// Duplicate request
	DuplicateRequest {
		/// Duplicate request metadata
		meta: Meta,
	},
	/// Duplicate response
	DuplicateResponse {
		/// Duplicate response metadata
		meta: Meta,
	},
	/// Request source does not match proof metadata
	RequestProofMetadataNotValid {
		/// The Request metadata
		meta: Meta,
	},
	/// Proxy cannot be used when a direct connection exists
	RequestProxyProhibited {
		/// The Request metadata
		meta: Meta,
	},
	/// Proxy cannot be used when a direct connection exists
	ResponseProxyProhibited {
		/// The Response metadata
		meta: Meta,
	},
	/// Host is not a proxy and destination chain does is not the host
	InvalidRequestDestination {
		/// The Request metadata
		meta: Meta,
	},
	/// The response destination does not match
	InvalidResponseDestination {
		/// The response metadata
		meta: Meta,
	},
	/// Expected get request found post
	InvalidResponseType {
		/// The request metadata
		meta: Meta,
	},
	/// Error decoding signature
	SignatureDecodingFailed,
	/// Anyhow error: {0}
	AnyHow(AnyhowError),
}

/// SCALE-compatible wrapper around [`anyhow::Error`].
#[derive(Debug)]
pub struct AnyhowError(pub anyhow::Error);

impl core::fmt::Display for AnyhowError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		core::fmt::Display::fmt(&self.0, f)
	}
}

impl PartialEq for AnyhowError {
	fn eq(&self, other: &Self) -> bool {
		self.0.to_string() == other.0.to_string()
	}
}

impl Eq for AnyhowError {}

impl Encode for AnyhowError {
	fn encode_to<W: codec::Output + ?Sized>(&self, dest: &mut W) {
		self.0.to_string().encode_to(dest)
	}
}

impl Decode for AnyhowError {
	fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
		let s = String::decode(input)?;
		Ok(Self(anyhow::Error::msg(s)))
	}
}

impl TypeInfo for AnyhowError {
	type Identity = String;

	fn type_info() -> scale_info::Type {
		String::type_info()
	}
}

impl From<anyhow::Error> for AnyhowError {
	fn from(e: anyhow::Error) -> Self {
		Self(e)
	}
}
