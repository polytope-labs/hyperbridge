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
    error::Error, host::ISMPHost, messaging::Proof, prelude::Vec, router::RequestResponse,
};
use codec::{Decode, Encode};
use core::time::Duration;

pub type ConsensusClientId = u64;

/// Fixed size hash type
pub type Hash = [u8; 32];

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct StateCommitment {
    /// Timestamp in seconds
    pub timestamp: u64,
    /// Root hash of the request/response merkle mountain range tree.
    pub ismp_root: Hash,
    /// Root hash of the global state trie.
    pub state_root: Hash,
}

/// We define the intermediate state as the commitment to the global state trie at a given height
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct IntermediateState {
    pub height: StateMachineHeight,
    pub commitment: StateCommitment,
}

/// Since consensus systems may come to conensus about the state of multiple state machines, we
/// identify each state machine individually.
#[derive(
    Debug, Clone, Copy, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq, Ord, PartialOrd,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct StateMachineId {
    pub state_id: u64,
    pub consensus_client: ConsensusClientId,
}

#[derive(
    Debug, Clone, Copy, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq, Ord, PartialOrd,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct StateMachineHeight {
    pub id: StateMachineId,
    pub height: u64,
}

/// We define the consensus client as a module that handles logic for consensus proof verification,
/// and State-Proof verification as well.
pub trait ConsensusClient {
    /// Should decode the scale encoded trusted consensus state and new consensus proof, verifying
    /// that:
    /// - check for byzantine behaviour
    /// - verify the consensus proofs
    /// - finally return the new consensusState and verified state commitments.
    fn verify_consensus(
        &self,
        host: &dyn ISMPHost,
        trusted_consensus_state: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<IntermediateState>), Error>;

    /// Return unbonding period
    fn unbonding_period(&self) -> Duration;

    /// Verify membership of proof of a commitment
    fn verify_membership(
        &self,
        host: &dyn ISMPHost,
        item: RequestResponse,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<(), Error>;

    /// Verify the state of proof of some arbitrary data. Should return the verified data
    fn verify_state_proof(
        &self,
        host: &dyn ISMPHost,
        key: Vec<u8>,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<Vec<u8>, Error>;

    /// Verify non-membership of proof of a commitment
    fn verify_non_membership(
        &self,
        host: &dyn ISMPHost,
        item: RequestResponse,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<(), Error>;

    /// Decode trusted state and check if consensus client is frozen
    fn is_frozen(&self, trusted_consensus_state: &[u8]) -> Result<(), Error>;
}
