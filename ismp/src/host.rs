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

//! The IsmpHost definition

use crate::{
    consensus::{
        ConsensusClient, ConsensusClientId, StateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error,
    prelude::Vec,
    router::{IsmpRouter, Request, Response},
};
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
};
use codec::{Decode, Encode};
use core::{str::FromStr, time::Duration};
use primitive_types::H256;

/// Defines the necessary interfaces that must be satisfied by a state machine for it be ISMP
/// compatible.
pub trait IsmpHost {
    /// Should return the state machine type for the host.
    fn host_state_machine(&self) -> StateMachine;

    /// Should return the latest height of the state machine
    fn latest_commitment_height(&self, id: StateMachineId) -> Result<u64, Error>;

    /// Should return the state machine at the given height
    fn state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<StateCommitment, Error>;

    /// Should return the host timestamp when this consensus client was last updated
    fn consensus_update_time(&self, id: ConsensusClientId) -> Result<Duration, Error>;

    /// Should return the encoded consensus state for a consensus client
    fn consensus_state(&self, id: ConsensusClientId) -> Result<Vec<u8>, Error>;

    /// Should return the current timestamp on the host
    fn timestamp(&self) -> Duration;

    /// Checks if a state machine is frozen at the provided height, should return Ok(()) if it isn't
    /// or [`Error::FrozenStateMachine`] if it is.
    fn is_state_machine_frozen(&self, machine: StateMachineHeight) -> Result<(), Error>;

    /// Checks if a state machine is frozen at the provided height
    fn is_consensus_client_frozen(&self, client: ConsensusClientId) -> Result<(), Error>;

    /// Fetch commitment of an outgoing request from storage
    fn request_commitment(&self, req: &Request) -> Result<H256, Error>;

    /// Increment and return the next available nonce for an outgoing request.
    fn next_nonce(&self) -> u64;

    /// Get an incoming request receipt, should return Some(()) if a receipt exists in storage
    fn request_receipt(&self, req: &Request) -> Option<()>;

    /// Get an incoming response receipt, should return Some(()) if a receipt exists in storage
    fn response_receipt(&self, res: &Response) -> Option<()>;

    /// Store an encoded consensus state
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

    /// Freeze a consensus client with the given identifier
    fn freeze_consensus_client(&self, client: ConsensusClientId) -> Result<(), Error>;

    /// Store latest height for a state machine
    fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error>;

    /// Delete a request commitment from storage, used when a request is timed out
    fn delete_request_commitment(&self, req: &Request) -> Result<(), Error>;

    /// Stores a receipt for an incoming request after it is successfully routed to a module.
    /// Prevents duplicate incoming requests from being processed.
    fn store_request_receipt(&self, req: &Request) -> Result<(), Error>;

    /// Stores a receipt for an incoming response after it is successfully routed to a module.
    /// Prevents duplicate incoming responses from being processed.
    fn store_response_receipt(&self, req: &Response) -> Result<(), Error>;

    /// Should return a handle to the consensus client based on the id
    fn consensus_client(&self, id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error>;

    /// Returns a keccak256 hash of a byte slice
    fn keccak256(bytes: &[u8]) -> H256
    where
        Self: Sized;

    /// Should return the configured delay period for a consensus client
    fn challenge_period(&self, id: ConsensusClientId) -> Duration;

    /// Check if the client has expired since the last update
    fn is_expired(&self, consensus_id: ConsensusClientId) -> Result<(), Error> {
        let host_timestamp = self.timestamp();
        let unbonding_period = self.consensus_client(consensus_id)?.unbonding_period();
        let last_update = self.consensus_update_time(consensus_id)?;
        if host_timestamp.saturating_sub(last_update) >= unbonding_period {
            Err(Error::UnbondingPeriodElapsed { consensus_id })?
        }

        Ok(())
    }

    /// Return a handle to the router
    fn ismp_router(&self) -> Box<dyn IsmpRouter>;
}

/// Currently supported state machines.
#[derive(
    Clone, Debug, Copy, Encode, Decode, PartialOrd, Ord, PartialEq, Eq, Hash, scale_info::TypeInfo,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum StateMachine {
    /// Ethereum Execution layer
    #[codec(index = 0)]
    Ethereum,
    /// Arbitrum Optimistic L2
    #[codec(index = 1)]
    Arbitrum,
    /// Optimism Optimistic L2
    #[codec(index = 2)]
    Optimism,
    /// Base Optimistic L2
    #[codec(index = 3)]
    Base,
    /// Polkadot parachains
    #[codec(index = 4)]
    Polkadot(u32),
    /// Kusama parachains
    #[codec(index = 5)]
    Kusama(u32),
}

impl ToString for StateMachine {
    fn to_string(&self) -> String {
        match self {
            StateMachine::Ethereum => "ETHEREUM".to_string(),
            StateMachine::Arbitrum => "ARBITRUM".to_string(),
            StateMachine::Optimism => "OPTIMISM".to_string(),
            StateMachine::Base => "BASE".to_string(),
            StateMachine::Polkadot(id) => format!("POLKADOT-{id}"),
            StateMachine::Kusama(id) => format!("KUSAMA-{id}"),
        }
    }
}

impl FromStr for StateMachine {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = match s {
            "ETHEREUM" => StateMachine::Ethereum,
            "ARBITRUM" => StateMachine::Arbitrum,
            "OPTIMISM" => StateMachine::Optimism,
            "BASE" => StateMachine::Base,
            name if name.starts_with("POLKADOT-") => {
                let id = name
                    .split('-')
                    .last()
                    .and_then(|id| u32::from_str(id).ok())
                    .ok_or_else(|| format!("invalid state machine: {name}"))?;
                StateMachine::Polkadot(id)
            }
            name if name.starts_with("KUSAMA-") => {
                let id = name
                    .split('-')
                    .last()
                    .and_then(|id| u32::from_str(id).ok())
                    .ok_or_else(|| format!("invalid state machine: {name}"))?;
                StateMachine::Kusama(id)
            }
            name => Err(format!("Unkown state machine: {name}"))?,
        };

        Ok(s)
    }
}
