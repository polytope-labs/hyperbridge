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

use crate::{
    consensus::{
        ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error,
    router::{Post, Request, Response},
};
use alloc::{string::ToString, vec::Vec};
use codec::{Decode, Encode};

/// A consensus message is used to update the state of a consensus client and its children state
/// machines.
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct ConsensusMessage {
    /// Scale Encoded Consensus Proof
    pub consensus_proof: Vec<u8>,
    /// The consensus state Id
    pub consensus_state_id: ConsensusStateId,
}

/// A fraud proof message is used to report byzantine misbehaviour in a consensus system.
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct FraudProofMessage {
    /// The first consensus Proof
    pub proof_1: Vec<u8>,
    /// The second consensus Proof
    pub proof_2: Vec<u8>,
    /// The consensus state Id
    pub consensus_state_id: ConsensusStateId,
}

/// Identifies a state commitment at a given height
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct StateCommitmentHeight {
    /// The state machine identifier
    pub commitment: StateCommitment,
    /// the corresponding block height
    pub height: u64,
}

/// Used for creating the initial consensus state for a given consensus client.
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct CreateConsensusState {
    /// Scale encoded consensus state
    pub consensus_state: Vec<u8>,
    /// Consensus client id
    pub consensus_client_id: ConsensusClientId,
    /// The consensus state Id
    pub consensus_state_id: ConsensusStateId,
    /// Unbonding period for this consensus state.
    pub unbonding_period: u64,
    /// Challenge period for this consensus state
    pub challenge_period: u64,
    /// State machine commitments
    pub state_machine_commitments: Vec<(StateMachineId, StateCommitmentHeight)>,
}

/// A request message holds a batch of requests to be dispatched from a source state machine
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct RequestMessage {
    /// Requests from source chain
    pub requests: Vec<Post>,
    /// Membership batch proof for these requests
    pub proof: Proof,
}

/// A request message holds a batch of responses to be dispatched from a source state machine
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub enum ResponseMessage {
    /// A POST request for sending data
    Post {
        /// Responses from sink chain
        responses: Vec<Response>,
        /// Membership batch proof for these responses
        proof: Proof,
    },
    /// A GET request for querying data
    Get {
        /// Request batch
        requests: Vec<Request>,
        /// State proof
        proof: Proof,
    },
}

impl ResponseMessage {
    /// Returns the requests in this message.
    pub fn requests(&self) -> Vec<Request> {
        match self {
            ResponseMessage::Post { responses, .. } =>
                responses.iter().map(|res| res.request()).collect(),
            ResponseMessage::Get { requests, .. } => requests.clone(),
        }
    }

    /// Retuns the associated proof
    pub fn proof(&self) -> &Proof {
        match self {
            ResponseMessage::Post { proof, .. } => proof,
            ResponseMessage::Get { proof, .. } => proof,
        }
    }
}

/// Returns an error if the proof height is less than any of the retrieval heights specified in the
/// get requests
pub fn sufficient_proof_height(requests: &[Request], proof: &Proof) -> Result<(), Error> {
    let check = requests.iter().all(|req| match req {
        Request::Get(get) => get.height == proof.height.height,
        _ => false,
    });
    if !check {
        Err(Error::InsufficientProofHeight)
    } else {
        Ok(())
    }
}

/// A request message holds a batch of requests to be timed-out
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub enum TimeoutMessage {
    /// A non memership proof for POST requests
    Post {
        /// Request timeouts
        requests: Vec<Request>,
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
    /// Returns the requests in this message.
    pub fn requests(&self) -> &[Request] {
        match self {
            TimeoutMessage::Post { requests, .. } => requests,
            TimeoutMessage::Get { requests } => requests,
        }
    }

    /// Returns the associated proof
    pub fn timeout_proof(&self) -> Result<&Proof, Error> {
        match self {
            TimeoutMessage::Post { timeout_proof, .. } => Ok(timeout_proof),
            _ => Err(Error::ImplementationSpecific(
                "Method should not be called on Get request".to_string(),
            )),
        }
    }
}

/// Proof holds the relevant proof data for the context in which it's used.
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Proof {
    /// State machine height
    pub height: StateMachineHeight,
    /// Scale encoded proof
    pub proof: Vec<u8>,
}

/// The Overaching ISMP message type.
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
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
