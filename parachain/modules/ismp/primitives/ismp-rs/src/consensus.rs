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

//! Consensus client definitions

use crate::{
    error::Error,
    host::{IsmpHost, StateMachine},
    messaging::{Proof, StateCommitmentHeight},
    prelude::Vec,
    router::{Request, RequestResponse},
};
use alloc::{boxed::Box, collections::BTreeMap};
use codec::{Decode, Encode};
use core::time::Duration;
use primitive_types::H256;

/// An identifier for a consensus states
pub type ConsensusStateId = [u8; 4];

/// An identifier for Consensus client implementations
pub type ConsensusClientId = [u8; 4];

/// The state commitment represents a commitment to the state machine's state (trie) at a given
/// height. Optionally holds a commitment to the ISMP request/response trie if supported by the
/// state machine.
#[derive(Debug, Clone, Copy, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct StateCommitment {
    /// Timestamp in seconds
    pub timestamp: u64,
    /// Root hash of the request/response overlay trie if the state machine supports it.
    pub overlay_root: Option<H256>,
    /// Root hash of the global state trie.
    pub state_root: H256,
}

impl StateCommitment {
    /// Returns the timestamp
    pub fn timestamp(&self) -> Duration {
        Duration::from_secs(self.timestamp)
    }
}

/// We define the intermediate state as the commitment to the global state trie at a given height
#[derive(Debug, Clone, Copy, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct IntermediateState {
    /// The state machine height holds the state mahine identifier and a block height
    pub height: StateMachineHeight,
    /// The corresponding state commitment for the state machine at the given block height.
    pub commitment: StateCommitment,
}

/// Since consensus systems may come to conensus about the state of multiple state machines, we
/// identify each state machine individually.
#[derive(
    Debug, Clone, Copy, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq, Hash, Ord, PartialOrd,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct StateMachineId {
    /// The state machine identifier
    pub state_id: StateMachine,
    /// It's consensus state identifier
    pub consensus_state_id: ConsensusStateId,
}

/// Identifies a state machine at a given height
#[derive(
    Debug, Clone, Copy, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq, Hash, Ord, PartialOrd,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct StateMachineHeight {
    /// The state machine identifier
    pub id: StateMachineId,
    /// the corresponding block height
    pub height: u64,
}

/// A map of state machine to verified state commitments
pub type VerifiedCommitments = BTreeMap<StateMachine, Vec<StateCommitmentHeight>>;

/// We define the consensus client as a module that handles logic for consensus proof verification,
/// and State-Proof verification as well.
pub trait ConsensusClient {
    /// Verify the associated consensus proof, using the trusted consensus state.
    fn verify_consensus(
        &self,
        host: &dyn IsmpHost,
        consensus_state_id: ConsensusStateId,
        trusted_consensus_state: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(Vec<u8>, VerifiedCommitments), Error>;

    /// Given two distinct consensus proofs, verify that they're both valid and represent
    /// conflicting views of the network. returns Ok(()) if they're both valid.
    fn verify_fraud_proof(
        &self,
        host: &dyn IsmpHost,
        trusted_consensus_state: Vec<u8>,
        proof_1: Vec<u8>,
        proof_2: Vec<u8>,
    ) -> Result<(), Error>;

    /// Return an implementation of a [`StateMachineClient`] for the given state machine.
    /// Return an error if the identifier is unknown.
    fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error>;
}

/// A state machine client. An abstraction for the mechanism of state proof verification for state
/// machines
pub trait StateMachineClient {
    /// Verify the overlay membership proof of a batch of requests/responses.
    fn verify_membership(
        &self,
        host: &dyn IsmpHost,
        item: RequestResponse,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<(), Error>;

    /// Transform the requests/responses into their equivalent key in the state trie.
    fn state_trie_key(&self, request: Vec<Request>) -> Vec<Vec<u8>>;

    /// Verify the state of proof of some arbitrary data. Should return the verified data
    fn verify_state_proof(
        &self,
        host: &dyn IsmpHost,
        keys: Vec<Vec<u8>>,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error>;
}
