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
        ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
        StateMachineId,
    },
    error::Error,
    prelude::Vec,
    router::{IsmpRouter, Request},
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
    fn consensus_update_time(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Result<Duration, Error>;

    /// Should return the registered consensus client id for this consensus state id
    fn consensus_client_id(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Option<ConsensusClientId>;

    /// Should return the encoded consensus state for a consensus state id provided
    fn consensus_state(&self, consensus_state_id: ConsensusStateId) -> Result<Vec<u8>, Error>;

    /// Should return the current timestamp on the host
    fn timestamp(&self) -> Duration;

    /// Checks if a state machine is frozen at the provided height, should return Ok(()) if it isn't
    /// or [`Error::FrozenStateMachine`] if it is.
    fn is_state_machine_frozen(&self, machine: StateMachineHeight) -> Result<(), Error>;

    /// Checks if a consensus state is frozen at the provided height
    fn is_consensus_client_frozen(&self, consensus_state_id: ConsensusStateId)
        -> Result<(), Error>;

    /// Should return an error if request commitment does not exist in storage
    fn request_commitment(&self, req: H256) -> Result<(), Error>;

    /// Increment and return the next available nonce for an outgoing request.
    fn next_nonce(&self) -> u64;

    /// Should return Some(()) if a receipt for this request exists in storage
    fn request_receipt(&self, req: &Request) -> Option<()>;

    /// Should return Some(()) if a response has been received for the given request
    fn response_receipt(&self, res: &Request) -> Option<()>;

    /// Store a map of consensus_state_id to the consensus_client_id
    /// Should return an error if the consensus_state_id already exists
    fn store_consensus_state_id(
        &self,
        consensus_state_id: ConsensusStateId,
        client_id: ConsensusClientId,
    ) -> Result<(), Error>;

    /// Store an encoded consensus state
    fn store_consensus_state(
        &self,
        consensus_state_id: ConsensusStateId,
        consensus_state: Vec<u8>,
    ) -> Result<(), Error>;

    /// Store the timestamp when the consensus client was updated
    fn store_consensus_update_time(
        &self,
        consensus_state_id: ConsensusStateId,
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

    /// Freeze a consensus state with the given identifier
    fn freeze_consensus_client(&self, consensus_state_id: ConsensusStateId) -> Result<(), Error>;

    /// Store latest height for a state machine
    fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error>;

    /// Delete a request commitment from storage, used when a request is timed out
    fn delete_request_commitment(&self, req: &Request) -> Result<(), Error>;

    /// Stores a receipt for an incoming request after it is successfully routed to a module.
    /// Prevents duplicate incoming requests from being processed.
    fn store_request_receipt(&self, req: &Request) -> Result<(), Error>;

    /// Stores a receipt that shows that the given request has received a response
    fn store_response_receipt(&self, req: &Request) -> Result<(), Error>;

    /// Should return a handle to the consensus client based on the id
    fn consensus_client(&self, id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error>;

    /// Returns a keccak256 hash of a byte slice
    fn keccak256(bytes: &[u8]) -> H256
    where
        Self: Sized;

    /// Should return the configured delay period for a consensus state
    fn challenge_period(&self, consensus_state_id: ConsensusStateId) -> Option<Duration>;

    /// Check if the client has expired since the last update
    fn is_expired(&self, consensus_state_id: ConsensusStateId) -> Result<(), Error> {
        let host_timestamp = self.timestamp();
        let unbonding_period = self
            .unbonding_period(consensus_state_id)
            .ok_or(Error::UnnbondingPeriodNotConfigured { consensus_state_id })?;
        let last_update = self.consensus_update_time(consensus_state_id)?;
        if host_timestamp.saturating_sub(last_update) >= unbonding_period {
            Err(Error::UnbondingPeriodElapsed { consensus_state_id })?
        }

        Ok(())
    }

    /// Return the unbonding period (i.e the time it takes for a validator's deposit to be unstaked
    /// from the network)
    fn unbonding_period(&self, consensus_state_id: ConsensusStateId) -> Option<Duration>;

    /// Return a handle to the router
    fn ismp_router(&self) -> Box<dyn IsmpRouter>;
}

/// Currently supported ethereum state machines.
#[derive(
    Clone, Debug, Copy, Encode, Decode, PartialOrd, Ord, PartialEq, Eq, Hash, scale_info::TypeInfo,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum Ethereum {
    /// Ethereum Execution layer
    ExecutionLayer,
    /// The optimism state machine
    Optimism,
    /// The Arbitrum state machine
    Arbitrum,
    /// The Base state machine
    Base,
}

/// Currently supported state machines.
#[derive(
    Clone, Debug, Copy, Encode, Decode, PartialOrd, Ord, PartialEq, Eq, Hash, scale_info::TypeInfo,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum StateMachine {
    /// Ethereum state machines
    #[codec(index = 0)]
    Ethereum(Ethereum),
    /// Polkadot parachains
    #[codec(index = 1)]
    Polkadot(u32),
    /// Kusama parachains
    #[codec(index = 2)]
    Kusama(u32),
    /// We identify
    #[codec(index = 3)]
    Grandpa(ConsensusStateId),
    /// State machines chains running on beefy consensus client
    #[codec(index = 4)]
    Beefy(ConsensusStateId),
}

impl ToString for StateMachine {
    fn to_string(&self) -> String {
        match self {
            StateMachine::Ethereum(ethereum) => match ethereum {
                Ethereum::ExecutionLayer => "ETHEREUM".to_string(),
                Ethereum::Arbitrum => "ARBITRUM".to_string(),
                Ethereum::Optimism => "OPTIMISM".to_string(),
                Ethereum::Base => "BASE".to_string(),
            },
            StateMachine::Polkadot(id) => format!("POLKADOT-{id}"),
            StateMachine::Kusama(id) => format!("KUSAMA-{id}"),
            StateMachine::Grandpa(id) => format!(
                "GRANDPA-{}",
                serde_json::to_string(id).expect("Array to string is infallible")
            ),
            StateMachine::Beefy(id) => format!(
                "BEEFY-{}",
                serde_json::to_string(id).expect("Array to string is infallible")
            ),
        }
    }
}

impl FromStr for StateMachine {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = match s {
            "ETHEREUM" => StateMachine::Ethereum(Ethereum::ExecutionLayer),
            "ARBITRUM" => StateMachine::Ethereum(Ethereum::Arbitrum),
            "OPTIMISM" => StateMachine::Ethereum(Ethereum::Optimism),
            "BASE" => StateMachine::Ethereum(Ethereum::Base),
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
            name if name.starts_with("GRANDPA-") => {
                let id = name
                    .split('-')
                    .last()
                    .and_then(|id| serde_json::from_str(id).ok())
                    .ok_or_else(|| format!("invalid state machine: {name}"))?;
                StateMachine::Grandpa(id)
            }
            name if name.starts_with("BEEFY-") => {
                let id = name
                    .split('-')
                    .last()
                    .and_then(|id| serde_json::from_str(id).ok())
                    .ok_or_else(|| format!("invalid state machine: {name}"))?;
                StateMachine::Beefy(id)
            }
            name => Err(format!("Unknown state machine: {name}"))?,
        };

        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use crate::host::StateMachine;
    use alloc::string::ToString;
    use core::str::FromStr;

    #[test]
    fn state_machine_conversions() {
        let grandpa = StateMachine::Grandpa(*b"hybr");
        let beefy = StateMachine::Beefy(*b"hybr");

        let grandpa_string = grandpa.to_string();
        let beefy_string = beefy.to_string();

        assert_eq!(grandpa, StateMachine::from_str(&grandpa_string).unwrap());
        assert_eq!(beefy, StateMachine::from_str(&beefy_string).unwrap());
    }
}
