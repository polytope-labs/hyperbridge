// Copyright (c) 2024 Polytope Labs.
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

//! Pallet error definitions and conversions

use codec::{Decode, Encode};
use ismp::{
	consensus::{ConsensusClientId, StateMachineHeight, StateMachineId},
	error::Error as IsmpError,
	events::Meta,
};
use sp_std::prelude::*;

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum HandlingError {
	ChallengePeriodNotElapsed {
		update_time: u64,
		current_time: u64,
		delay_period: Option<u64>,
		consensus_client_id: Option<StateMachineId>,
	},
	ConsensusStateNotFound {
		id: ConsensusClientId,
	},
	StateCommitmentNotFound {
		height: StateMachineHeight,
	},
	FrozenConsensusClient {
		id: ConsensusClientId,
	},
	FrozenStateMachine {
		id: StateMachineId,
	},
	RequestCommitmentNotFound {
		/// The Request metadata
		meta: Meta,
	},
	RequestVerificationFailed {
		/// The Request metadata
		meta: Meta,
	},
	ResponseVerificationFailed {
		/// The response metadata
		meta: Meta,
	},
	ConsensusProofVerificationFailed {
		id: ConsensusClientId,
	},
	ExpiredConsensusClient {
		id: ConsensusClientId,
	},
	CannotHandleMessage,
	Custom {
		msg: Vec<u8>,
	},
	UnbondingPeriodElapsed {
		id: ConsensusClientId,
	},
	MembershipProofVerificationFailed {
		msg: Vec<u8>,
	},
	NonMembershipProofVerificationFailed {
		msg: Vec<u8>,
	},
	CannotCreateAlreadyExistingConsensusClient {
		id: ConsensusClientId,
	},
	RequestTimeoutNotElapsed {
		/// The Request metadata
		meta: Meta,
		timeout_timestamp: u64,
		state_machine_time: u64,
	},
	RequestTimeoutVerificationFailed {
		/// The Request metadata
		meta: Meta,
	},
	InsufficientProofHeight,
	ModuleNotFound(Vec<u8>),
	ModuleDispatchError {
		/// Descriptive error message
		msg: Vec<u8>,
		/// the request metadata metadata
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
		/// Timed out Response metdata
		meta: Meta,
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
	/// Response source does not match proof metadata
	ResponseProofMetadataNotValid {
		/// The Response metadata
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
}

impl From<ismp::error::Error> for HandlingError {
	fn from(value: ismp::error::Error) -> Self {
		match value {
			IsmpError::ChallengePeriodNotElapsed {
				state_machine_id,
				current_time,
				update_time,
			} => HandlingError::ChallengePeriodNotElapsed {
				update_time: update_time.as_secs(),
				current_time: current_time.as_secs(),
				delay_period: None,
				consensus_client_id: Some(state_machine_id),
			},
			IsmpError::ConsensusStateNotFound { consensus_state_id } =>
				HandlingError::ConsensusStateNotFound { id: consensus_state_id },
			IsmpError::StateCommitmentNotFound { height } =>
				HandlingError::StateCommitmentNotFound { height },
			IsmpError::FrozenConsensusClient { consensus_state_id } =>
				HandlingError::FrozenConsensusClient { id: consensus_state_id },
			IsmpError::RequestCommitmentNotFound { meta } =>
				HandlingError::RequestCommitmentNotFound { meta },
			IsmpError::RequestVerificationFailed { meta } =>
				HandlingError::ResponseVerificationFailed { meta },
			IsmpError::ResponseVerificationFailed { meta } =>
				HandlingError::ResponseVerificationFailed { meta },
			IsmpError::ConsensusProofVerificationFailed { id } =>
				HandlingError::ConsensusProofVerificationFailed { id },
			IsmpError::ExpiredConsensusClient { id } =>
				HandlingError::ExpiredConsensusClient { id },
			IsmpError::CannotHandleMessage => HandlingError::CannotHandleMessage,
			IsmpError::Custom(msg) => HandlingError::Custom { msg: msg.as_bytes().to_vec() },
			IsmpError::UnbondingPeriodElapsed { consensus_state_id } =>
				HandlingError::UnbondingPeriodElapsed { id: consensus_state_id },
			IsmpError::MembershipProofVerificationFailed(msg) =>
				HandlingError::MembershipProofVerificationFailed { msg: msg.as_bytes().to_vec() },
			IsmpError::NonMembershipProofVerificationFailed(msg) =>
				HandlingError::NonMembershipProofVerificationFailed { msg: msg.as_bytes().to_vec() },
			IsmpError::CannotCreateAlreadyExistingConsensusClient { id } =>
				HandlingError::CannotCreateAlreadyExistingConsensusClient { id },
			IsmpError::RequestTimeoutNotElapsed { meta, timeout_timestamp, state_machine_time } =>
				HandlingError::RequestTimeoutNotElapsed {
					meta,
					timeout_timestamp: timeout_timestamp.as_secs(),
					state_machine_time: state_machine_time.as_secs(),
				},
			IsmpError::RequestTimeoutVerificationFailed { meta } =>
				HandlingError::RequestTimeoutVerificationFailed { meta },
			IsmpError::InsufficientProofHeight => HandlingError::InsufficientProofHeight,
			IsmpError::ModuleNotFound(id) => HandlingError::ModuleNotFound(id),
			IsmpError::ConsensusStateIdNotRecognized { .. } =>
				HandlingError::InsufficientProofHeight,
			IsmpError::ChallengePeriodNotConfigured { .. } =>
				HandlingError::InsufficientProofHeight,
			IsmpError::DuplicateConsensusStateId { .. } => HandlingError::InsufficientProofHeight,
			IsmpError::UnnbondingPeriodNotConfigured { .. } =>
				HandlingError::InsufficientProofHeight,
			IsmpError::ModuleDispatchError { msg, meta } =>
				HandlingError::ModuleDispatchError { msg: msg.as_bytes().to_vec(), meta },
			IsmpError::UnsolicitedResponse { meta } => HandlingError::UnsolicitedResponse { meta },
			IsmpError::RequestTimeout { meta } => HandlingError::RequestTimeout { meta },
			IsmpError::ResponseTimeout { response } =>
				HandlingError::ResponseTimeout { meta: response },
			IsmpError::DuplicateRequest { meta } => HandlingError::DuplicateRequest { meta },
			IsmpError::DuplicateResponse { meta } => HandlingError::DuplicateResponse { meta },
			IsmpError::RequestProofMetadataNotValid { meta } =>
				HandlingError::RequestProofMetadataNotValid { meta },
			IsmpError::RequestProxyProhibited { meta } =>
				HandlingError::RequestProxyProhibited { meta },
			IsmpError::ResponseProxyProhibited { meta } =>
				HandlingError::ResponseProxyProhibited { meta },
			IsmpError::InvalidRequestDestination { meta } =>
				HandlingError::InvalidRequestDestination { meta },
			IsmpError::InvalidResponseDestination { meta } =>
				HandlingError::InvalidResponseDestination { meta },
			IsmpError::InvalidResponseType { meta } => HandlingError::InvalidResponseType { meta },
			IsmpError::UnknownRequest { meta } => HandlingError::UnknownRequest { meta },
			IsmpError::UnknownResponse { meta } => HandlingError::UnknownResponse { meta },
		}
	}
}
