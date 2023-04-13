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

//! The ISMPHost definition

use crate::{
    consensus_client::{
        ConsensusClient, ConsensusClientId, StateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error,
    prelude::Vec,
    router::{ISMPRouter, Request, Response},
};
use alloc::boxed::Box;
use codec::{Decode, Encode};
use core::time::Duration;
use derive_more::Display;
use primitive_types::U256;

pub trait ISMPHost {
    fn host(&self) -> ChainID;

    // Storage Read functions

    /// Returns the latest height of the state machine
    fn latest_commitment_height(&self, id: StateMachineId) -> Result<StateMachineHeight, Error>;
    /// Returns the state machine at the give height
    fn state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<StateCommitment, Error>;
    /// Returns the host timestamp when this consensus client was last updated
    fn consensus_update_time(&self, id: ConsensusClientId) -> Result<Duration, Error>;
    /// Returns the scale encoded consensus state for a consensus client
    fn consensus_state(&self, id: ConsensusClientId) -> Result<Vec<u8>, Error>;
    /// Return the host timestamp in nanoseconds
    fn timestamp(&self) -> Duration;
    /// Checks if a state machine is frozen at the provided height
    fn is_frozen(&self, height: StateMachineHeight) -> Result<bool, Error>;
    /// Fetch commitment of a request from storage
    fn request_commitment(&self, req: &Request) -> Result<Vec<u8>, Error>;

    // Storage Write functions

    /// Store a scale encoded consensus state
    fn store_consensus_state(&self, id: ConsensusClientId, state: Vec<u8>) -> Result<(), Error>;
    /// Store the timestamp when the consensus client was updated
    fn store_consensus_update_time(
        &self,
        id: ConsensusClientId,
        timestamp: Duration,
    ) -> Result<(), Error>;
    /// Store the timestamp when the state machine was updated
    fn store_state_machine_commitment(
        &self,
        height: StateMachineHeight,
        state: StateCommitment,
    ) -> Result<(), Error>;
    /// Freeze a state machine at the given height
    fn freeze_state_machine(&self, height: StateMachineHeight) -> Result<(), Error>;
    /// Store latest height for a state machine
    fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error>;

    /// Return the keccak256 hash of a request
    /// Commitment is the hash of the concatenation of the data below
    /// request.source_chain + request.dest_chain + request.nonce + request.data
    fn get_request_commitment(&self, req: &Request) -> Vec<u8> {
        let req = match req {
            Request::Post(post) => post,
            _ => unimplemented!(),
        };

        let mut buf = Vec::new();
        let mut source_chain = [0u8; 32];
        let mut dest_chain = [0u8; 32];
        let mut nonce = [0u8; 32];
        let mut timestamp = [0u8; 32];
        U256::from(req.source_chain as u8).to_big_endian(&mut source_chain);
        U256::from(req.dest_chain as u8).to_big_endian(&mut dest_chain);
        U256::from(req.nonce).to_big_endian(&mut nonce);
        U256::from(req.timeout_timestamp).to_big_endian(&mut timestamp);
        buf.extend_from_slice(&source_chain);
        buf.extend_from_slice(&dest_chain);
        buf.extend_from_slice(&nonce);
        buf.extend_from_slice(&timestamp);
        buf.extend_from_slice(&req.data);
        buf.extend_from_slice(&req.from);
        buf.extend_from_slice(&req.to);
        self.keccak256(&buf[..]).to_vec()
    }

    /// Return the keccak256 of a response
    fn get_response_commitment(&self, res: &Response) -> Vec<u8> {
        let req = match res.request {
            Request::Post(ref post) => post,
            _ => unimplemented!(),
        };
        let mut buf = Vec::new();
        let mut source_chain = [0u8; 32];
        let mut dest_chain = [0u8; 32];
        let mut nonce = [0u8; 32];
        let mut timestamp = [0u8; 32];
        U256::from(req.source_chain as u8).to_big_endian(&mut source_chain);
        U256::from(req.dest_chain as u8).to_big_endian(&mut dest_chain);
        U256::from(req.nonce as u8).to_big_endian(&mut nonce);
        U256::from(req.timeout_timestamp).to_big_endian(&mut timestamp);
        buf.extend_from_slice(&source_chain);
        buf.extend_from_slice(&dest_chain);
        buf.extend_from_slice(&nonce);
        buf.extend_from_slice(&timestamp);
        buf.extend_from_slice(&req.data);
        buf.extend_from_slice(&req.from);
        buf.extend_from_slice(&req.to);
        buf.extend_from_slice(&res.response);
        self.keccak256(&buf[..]).to_vec()
    }

    /// Should return a handle to the consensus client based on the id
    fn consensus_client(&self, id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error>;

    // Hashing
    /// Returns a keccak256 hash of a byte slice
    fn keccak256(&self, bytes: &[u8]) -> [u8; 32];

    /// Returns the configured delay period for a consensus client
    fn challenge_period(&self, id: ConsensusClientId) -> Duration;

    /// Check if the client has expired since the last update
    fn is_expired(&self, consensus_id: ConsensusClientId) -> Result<(), Error> {
        let host_timestamp = self.timestamp();
        let unbonding_period = self.consensus_client(consensus_id)?.unbonding_period();
        let last_update = self.consensus_update_time(consensus_id)?;
        if host_timestamp.saturating_sub(last_update) > unbonding_period {
            Err(Error::UnbondingPeriodElapsed { consensus_id })?
        }

        Ok(())
    }

    /// Return a handle to the router
    fn ismp_router(&self) -> Box<dyn ISMPRouter>;
}

#[derive(Clone, Debug, Copy, Encode, Decode, Display, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum ChainID {
    #[codec(index = 0)]
    ETHEREUM = 0,
    #[codec(index = 1)]
    GNOSIS = 1,
    #[codec(index = 2)]
    ARBITRUM = 2,
    #[codec(index = 3)]
    OPTIMISM = 3,
    #[codec(index = 4)]
    BASE = 4,
    #[codec(index = 5)]
    MOONBEAM = 5,
    #[codec(index = 6)]
    ASTAR = 6,
    #[codec(index = 7)]
    HYPERSPACE = 7,
}
