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

use crate::{
    consensus::{ConsensusClientId, IntermediateState, StateMachineHeight},
    router::{Request, Response},
};
use alloc::vec::Vec;
use codec::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct ConsensusMessage {
    /// Scale Encoded Consensus Proof
    pub consensus_proof: Vec<u8>,
    /// Consensus client id
    pub consensus_client_id: ConsensusClientId,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct CreateConsensusClient {
    /// Scale encoded consensus state
    pub consensus_state: Vec<u8>,
    /// Consensus client id
    pub consensus_client_id: ConsensusClientId,
    /// State machine commitments
    pub state_machine_commitments: Vec<IntermediateState>,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct RequestMessage {
    /// Requests from source chain
    pub requests: Vec<Request>,
    /// Membership batch proof for these requests
    pub proof: Proof,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct ResponseMessage {
    /// Responses from sink chain
    pub responses: Vec<Response>,
    /// Membership batch proof for these responses
    pub proof: Proof,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct TimeoutMessage {
    /// Request timeouts
    pub requests: Vec<Request>,
    /// Non membership batch proof for these requests
    pub timeout_proof: Proof,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Proof {
    /// State machine height
    pub height: StateMachineHeight,
    /// Scale encoded proof
    pub proof: Vec<u8>,
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub enum Message {
    #[codec(index = 0)]
    Consensus(ConsensusMessage),
    #[codec(index = 1)]
    Request(RequestMessage),
    #[codec(index = 2)]
    Response(ResponseMessage),
    #[codec(index = 3)]
    Timeout(TimeoutMessage),
}
