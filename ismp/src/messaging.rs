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
    consensus::{ConsensusClientId, IntermediateState, StateMachineHeight},
    error::Error,
    router::{Request, Response},
};
use alloc::{string::ToString, vec::Vec};
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
pub enum ResponseMessage {
    Post {
        /// Responses from sink chain
        responses: Vec<Response>,
        /// Membership batch proof for these responses
        proof: Proof,
    },
    Get {
        /// Request batch
        requests: Vec<Request>,
        /// State proof
        proof: Proof,
    },
}

impl ResponseMessage {
    pub fn requests(&self) -> Vec<Request> {
        match self {
            ResponseMessage::Post { responses, .. } => {
                responses.iter().map(|res| res.request()).collect()
            }
            ResponseMessage::Get { requests, .. } => requests.clone(),
        }
    }

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
    let check = requests.iter().any(|req| match req {
        Request::Get(get) => get.height > proof.height,
        _ => true,
    });
    if check {
        Err(Error::InsufficientProofHeight)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub enum TimeoutMessage {
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
    pub fn requests(&self) -> &[Request] {
        match self {
            TimeoutMessage::Post { requests, .. } => requests,
            TimeoutMessage::Get { requests } => requests,
        }
    }

    pub fn timeout_proof(&self) -> Result<&Proof, Error> {
        match self {
            TimeoutMessage::Post { timeout_proof, .. } => Ok(timeout_proof),
            _ => Err(Error::ImplementationSpecific(
                "Method should not be called on Get request".to_string(),
            )),
        }
    }
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
