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
    consensus::{ConsensusClientId, StateMachineHeight},
    host::StateMachine,
};
use alloc::string::String;
use core::time::Duration;

#[derive(Debug)]
pub enum Error {
    UnbondingPeriodElapsed {
        consensus_id: ConsensusClientId,
    },
    ChallengePeriodNotElapsed {
        consensus_id: ConsensusClientId,
        update_time: Duration,
        current_time: Duration,
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
    RequestTimeoutNotElapsed {
        nonce: u64,
        source: StateMachine,
        dest: StateMachine,
        timeout_timestamp: Duration,
        state_machine_time: Duration,
    },
    RequestTimeoutVerificationFailed {
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
    MembershipProofVerificationFailed(String),
    NonMembershipProofVerificationFailed(String),
    ImplementationSpecific(String),
    CannotCreateAlreadyExistingConsensusClient {
        id: ConsensusClientId,
    },
    InsufficientProofHeight,
}
