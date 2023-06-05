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
use ismp_rs::{
    consensus::{ConsensusClientId, StateMachineHeight},
    error::Error as IsmpError,
    host::StateMachine,
    router::DispatchResult,
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
        height: StateMachineHeight,
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
        consensus_id: ConsensusClientId,
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
}

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct ModuleDispatchError {
    /// Descriptive error message
    pub msg: Vec<u8>,
    /// Request nonce
    pub nonce: u64,
    /// Source chain for request or response
    pub source_chain: StateMachine,
    /// Destination chain for request or response
    pub dest_chain: StateMachine,
}

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct ModuleDispatchSuccess {
    /// Destination chain for request or response
    pub dest_chain: StateMachine,
    /// Source chain for request or response
    pub source_chain: StateMachine,
    /// Request nonce
    pub nonce: u64,
}

#[derive(Clone, Debug, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub enum ModuleCallbackResult {
    Response(Result<ModuleDispatchSuccess, ModuleDispatchError>),
    Request(Result<ModuleDispatchSuccess, ModuleDispatchError>),
    Timeout(Result<ModuleDispatchSuccess, ModuleDispatchError>),
}

pub fn to_response_results(values: Vec<DispatchResult>) -> Vec<ModuleCallbackResult> {
    values
        .into_iter()
        .map(|res| match res {
            Ok(res) => ModuleCallbackResult::Response(Ok(ModuleDispatchSuccess {
                dest_chain: res.dest_chain,
                source_chain: res.source_chain,
                nonce: res.nonce,
            })),
            Err(res) => ModuleCallbackResult::Response(Err(ModuleDispatchError {
                msg: res.msg.as_bytes().to_vec(),
                dest_chain: res.dest,
                source_chain: res.source,
                nonce: res.nonce,
            })),
        })
        .collect()
}

pub fn to_request_results(values: Vec<DispatchResult>) -> Vec<ModuleCallbackResult> {
    values
        .into_iter()
        .map(|res| match res {
            Ok(res) => ModuleCallbackResult::Request(Ok(ModuleDispatchSuccess {
                dest_chain: res.dest_chain,
                source_chain: res.source_chain,
                nonce: res.nonce,
            })),
            Err(res) => ModuleCallbackResult::Request(Err(ModuleDispatchError {
                msg: res.msg.as_bytes().to_vec(),
                dest_chain: res.dest,
                source_chain: res.source,
                nonce: res.nonce,
            })),
        })
        .collect()
}

pub fn to_timeout_results(values: Vec<DispatchResult>) -> Vec<ModuleCallbackResult> {
    values
        .into_iter()
        .map(|res| match res {
            Ok(res) => ModuleCallbackResult::Timeout(Ok(ModuleDispatchSuccess {
                dest_chain: res.dest_chain,
                source_chain: res.source_chain,
                nonce: res.nonce,
            })),
            Err(res) => ModuleCallbackResult::Timeout(Err(ModuleDispatchError {
                msg: res.msg.as_bytes().to_vec(),
                dest_chain: res.dest,
                source_chain: res.source,
                nonce: res.nonce,
            })),
        })
        .collect()
}

impl From<ismp_rs::error::Error> for HandlingError {
    fn from(value: ismp_rs::error::Error) -> Self {
        match value {
            IsmpError::ChallengePeriodNotElapsed { consensus_id, current_time, update_time } => {
                HandlingError::ChallengePeriodNotElapsed {
                    update_time: update_time.as_secs(),
                    current_time: current_time.as_secs(),
                    delay_period: None,
                    consensus_client_id: Some(consensus_id),
                }
            }
            IsmpError::ConsensusStateNotFound { id } => {
                HandlingError::ConsensusStateNotFound { id }
            }
            IsmpError::StateCommitmentNotFound { height } => {
                HandlingError::StateCommitmentNotFound { height }
            }
            IsmpError::FrozenConsensusClient { id } => HandlingError::FrozenConsensusClient { id },
            IsmpError::FrozenStateMachine { height } => {
                HandlingError::FrozenStateMachine { height }
            }
            IsmpError::RequestCommitmentNotFound { nonce, source, dest } => {
                HandlingError::RequestCommitmentNotFound { nonce, source, dest }
            }
            IsmpError::RequestVerificationFailed { nonce, source, dest } => {
                HandlingError::ResponseVerificationFailed { nonce, source, dest }
            }
            IsmpError::ResponseVerificationFailed { nonce, source, dest } => {
                HandlingError::ResponseVerificationFailed { nonce, source, dest }
            }
            IsmpError::ConsensusProofVerificationFailed { id } => {
                HandlingError::ConsensusProofVerificationFailed { id }
            }
            IsmpError::ExpiredConsensusClient { id } => {
                HandlingError::ExpiredConsensusClient { id }
            }
            IsmpError::CannotHandleMessage => HandlingError::CannotHandleMessage,
            IsmpError::ImplementationSpecific(msg) => {
                HandlingError::ImplementationSpecific { msg: msg.as_bytes().to_vec() }
            }
            IsmpError::UnbondingPeriodElapsed { consensus_id } => {
                HandlingError::UnbondingPeriodElapsed { consensus_id }
            }
            IsmpError::MembershipProofVerificationFailed(msg) => {
                HandlingError::MembershipProofVerificationFailed { msg: msg.as_bytes().to_vec() }
            }
            IsmpError::NonMembershipProofVerificationFailed(msg) => {
                HandlingError::NonMembershipProofVerificationFailed { msg: msg.as_bytes().to_vec() }
            }
            IsmpError::CannotCreateAlreadyExistingConsensusClient { id } => {
                HandlingError::CannotCreateAlreadyExistingConsensusClient { id }
            }
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
            IsmpError::RequestTimeoutVerificationFailed { nonce, source, dest } => {
                HandlingError::RequestTimeoutVerificationFailed { nonce, source, dest }
            }
            IsmpError::InsufficientProofHeight => HandlingError::InsufficientProofHeight,
        }
    }
}
