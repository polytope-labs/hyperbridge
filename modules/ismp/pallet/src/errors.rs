// Copyright (C) 2023 Polytope Labs.
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

//! Ismp Errors conversions
use codec::{Decode, Encode};
use ismp::{
    consensus::{ConsensusClientId, StateMachineHeight, StateMachineId},
    error::Error as IsmpError,
    host::StateMachine,
    router::{Request, Response},
};
use sp_std::prelude::*;

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum HandlingError {
    ChallengePeriodNotElapsed {
        update_time: u64,
        current_time: u64,
        delay_period: Option<u64>,
        consensus_client_id: Option<ConsensusClientId>,
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
        nonce: u64,
        source: StateMachine,
        dest: StateMachine,
    },
    RequestVerificationFailed {
        nonce: u64,
        source: StateMachine,
        dest: StateMachine,
    },
    ResponseVerificationFailed {
        nonce: u64,
        source: StateMachine,
        dest: StateMachine,
    },
    ConsensusProofVerificationFailed {
        id: ConsensusClientId,
    },
    ExpiredConsensusClient {
        id: ConsensusClientId,
    },
    CannotHandleMessage,
    ImplementationSpecific {
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
        nonce: u64,
        source: StateMachine,
        dest: StateMachine,
        timeout_timestamp: u64,
        state_machine_time: u64,
    },
    RequestTimeoutVerificationFailed {
        nonce: u64,
        source: StateMachine,
        dest: StateMachine,
    },
    InsufficientProofHeight,
    ModuleNotFound(Vec<u8>),
    ModuleDispatchError {
        /// Descriptive error message
        msg: Vec<u8>,
        /// Request nonce
        nonce: u64,
        /// Source chain for request or response
        source_chain: StateMachine,
        /// Destination chain for request or response
        dest_chain: StateMachine,
    },
    /// Request commitment for a response does not exist
    UnsolicitedResponse {
        /// Unsolicited response
        res: Response,
    },
    /// Timed out request found in batch
    RequestTimeout {
        /// Timed out Request
        req: Request,
    },
    /// Timed out response found in batch
    ResponseTimeout {
        /// Timed out Response
        response: Response,
    },
    /// Duplicate request
    DuplicateRequest {
        /// Duplicate request
        req: Request,
    },
    /// Duplicate response
    DuplicateResponse {
        /// Duplicate response
        res: Response,
    },
    /// Request source does not match proof metadata
    RequestProofMetadataNotValid {
        /// The Request
        req: Request,
    },
    /// Response source does not match proof metadata
    ResponseProofMetadataNotValid {
        /// The Response
        res: Response,
    },
    /// Proxy cannot be used when a direct connection exists
    RequestProxyProhibited {
        /// The Request
        req: Request,
    },
    /// Proxy cannot be used when a direct connection exists
    ResponseProxyProhibited {
        /// The Response
        res: Response,
    },
    /// Host is not a proxy and destination chain does is not the host
    InvalidRequestDestination {
        /// The Request
        req: Request,
    },
    /// The response destination does not match
    InvalidResponseDestination {
        /// The response
        res: Response,
    },
    /// Expected get request found post
    InvalidResponseType {
        /// The request
        req: Request,
    },
}

impl From<ismp::error::Error> for HandlingError {
    fn from(value: ismp::error::Error) -> Self {
        match value {
            IsmpError::ChallengePeriodNotElapsed {
                consensus_state_id,
                current_time,
                update_time,
            } => HandlingError::ChallengePeriodNotElapsed {
                update_time: update_time.as_secs(),
                current_time: current_time.as_secs(),
                delay_period: None,
                consensus_client_id: Some(consensus_state_id),
            },
            IsmpError::ConsensusStateNotFound { consensus_state_id } =>
                HandlingError::ConsensusStateNotFound { id: consensus_state_id },
            IsmpError::StateCommitmentNotFound { height } =>
                HandlingError::StateCommitmentNotFound { height },
            IsmpError::FrozenConsensusClient { consensus_state_id } =>
                HandlingError::FrozenConsensusClient { id: consensus_state_id },
            IsmpError::FrozenStateMachine { id } => HandlingError::FrozenStateMachine { id },
            IsmpError::RequestCommitmentNotFound { nonce, source, dest } =>
                HandlingError::RequestCommitmentNotFound { nonce, source, dest },
            IsmpError::RequestVerificationFailed { nonce, source, dest } =>
                HandlingError::ResponseVerificationFailed { nonce, source, dest },
            IsmpError::ResponseVerificationFailed { nonce, source, dest } =>
                HandlingError::ResponseVerificationFailed { nonce, source, dest },
            IsmpError::ConsensusProofVerificationFailed { id } =>
                HandlingError::ConsensusProofVerificationFailed { id },
            IsmpError::ExpiredConsensusClient { id } =>
                HandlingError::ExpiredConsensusClient { id },
            IsmpError::CannotHandleMessage => HandlingError::CannotHandleMessage,
            IsmpError::ImplementationSpecific(msg) =>
                HandlingError::ImplementationSpecific { msg: msg.as_bytes().to_vec() },
            IsmpError::UnbondingPeriodElapsed { consensus_state_id } =>
                HandlingError::UnbondingPeriodElapsed { id: consensus_state_id },
            IsmpError::MembershipProofVerificationFailed(msg) =>
                HandlingError::MembershipProofVerificationFailed { msg: msg.as_bytes().to_vec() },
            IsmpError::NonMembershipProofVerificationFailed(msg) =>
                HandlingError::NonMembershipProofVerificationFailed { msg: msg.as_bytes().to_vec() },
            IsmpError::CannotCreateAlreadyExistingConsensusClient { id } =>
                HandlingError::CannotCreateAlreadyExistingConsensusClient { id },
            IsmpError::RequestTimeoutNotElapsed {
                nonce,
                source,
                dest,
                timeout_timestamp,
                state_machine_time,
            } => HandlingError::RequestTimeoutNotElapsed {
                nonce,
                source,
                dest,
                timeout_timestamp: timeout_timestamp.as_secs(),
                state_machine_time: state_machine_time.as_secs(),
            },
            IsmpError::RequestTimeoutVerificationFailed { nonce, source, dest } =>
                HandlingError::RequestTimeoutVerificationFailed { nonce, source, dest },
            IsmpError::InsufficientProofHeight => HandlingError::InsufficientProofHeight,
            IsmpError::ModuleNotFound(id) => HandlingError::ModuleNotFound(id),
            IsmpError::ConsensusStateIdNotRecognized { .. } =>
                HandlingError::InsufficientProofHeight,
            IsmpError::ChallengePeriodNotConfigured { .. } =>
                HandlingError::InsufficientProofHeight,
            IsmpError::DuplicateConsensusStateId { .. } => HandlingError::InsufficientProofHeight,
            IsmpError::UnnbondingPeriodNotConfigured { .. } =>
                HandlingError::InsufficientProofHeight,
            IsmpError::ModuleDispatchError { msg, nonce, source_chain, dest_chain } =>
                HandlingError::ModuleDispatchError {
                    msg: msg.as_bytes().to_vec(),
                    nonce,
                    source_chain,
                    dest_chain,
                },
            IsmpError::UnsolicitedResponse { res } => HandlingError::UnsolicitedResponse { res },
            IsmpError::RequestTimeout { req } => HandlingError::RequestTimeout { req },
            IsmpError::ResponseTimeout { response } => HandlingError::ResponseTimeout { response },
            IsmpError::DuplicateRequest { req } => HandlingError::DuplicateRequest { req },
            IsmpError::DuplicateResponse { res } => HandlingError::DuplicateResponse { res },
            IsmpError::RequestProofMetadataNotValid { req } =>
                HandlingError::RequestProofMetadataNotValid { req },
            IsmpError::ResponseProofMetadataNotValid { res } =>
                HandlingError::ResponseProofMetadataNotValid { res },
            IsmpError::RequestProxyProhibited { req } =>
                HandlingError::RequestProxyProhibited { req },
            IsmpError::ResponseProxyProhibited { res } =>
                HandlingError::ResponseProxyProhibited { res },
            IsmpError::InvalidRequestDestination { req } =>
                HandlingError::InvalidRequestDestination { req },
            IsmpError::InvalidResponseDestination { res } =>
                HandlingError::InvalidResponseDestination { res },
            IsmpError::InvalidResponseType { req } => HandlingError::InvalidResponseType { req },
        }
    }
}
