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

use crate::{
    consensus::{ConsensusClientId, ConsensusStateId, StateMachineHeight},
    host::StateMachine,
};
use alloc::{string::String, vec::Vec};
use core::time::Duration;

/// Errors that may be encountered by the ISMP module
#[derive(Debug)]
pub enum Error {
    /// The unbonding period for the given consensus client has elapsed and can no longer process
    /// consensus updates.
    UnbondingPeriodElapsed {
        /// The consensus client identifier
        consensus_state_id: ConsensusStateId,
    },
    /// The challange period for the given consensus client has not yet elapsed and cannot process
    /// new consensus updates in the mean time.
    ChallengePeriodNotElapsed {
        /// The consensus client identifier
        consensus_state_id: ConsensusStateId,
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
    /// The given state machine has been frozen
    FrozenStateMachine {
        /// The given state machine height
        height: StateMachineHeight,
    },
    /// The given request was not found
    RequestCommitmentNotFound {
        /// The request nonce
        nonce: u64,
        /// The source state machine
        source: StateMachine,
        /// The destination state machine
        dest: StateMachine,
    },
    /// The given request has failed state proof verification
    RequestVerificationFailed {
        /// The request nonce
        nonce: u64,
        /// The source state machine
        source: StateMachine,
        /// The destination state machine
        dest: StateMachine,
    },
    /// The given request has not yet timed-out
    RequestTimeoutNotElapsed {
        /// The request nonce
        nonce: u64,
        /// The source state machine
        source: StateMachine,
        /// The destination state machine
        dest: StateMachine,
        /// The timestamp at which the timeout elapses
        timeout_timestamp: Duration,
        /// The current time on the state machine
        state_machine_time: Duration,
    },
    /// The given request has failed non-membership state proof verification
    RequestTimeoutVerificationFailed {
        /// The request nonce
        nonce: u64,
        /// The source state machine
        source: StateMachine,
        /// The destination state machine
        dest: StateMachine,
    },
    /// The given response has failed membership state proof verification
    ResponseVerificationFailed {
        /// The request nonce
        nonce: u64,
        /// The source state machine
        source: StateMachine,
        /// The destination state machine
        dest: StateMachine,
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
    /// Membership proof verification failed
    MembershipProofVerificationFailed(String),
    /// Non-membership proof verification failed
    NonMembershipProofVerificationFailed(String),
    /// Some implementation specific error
    ImplementationSpecific(String),
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
        /// Consensus state Id
        consensus_state_id: ConsensusStateId,
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
}
