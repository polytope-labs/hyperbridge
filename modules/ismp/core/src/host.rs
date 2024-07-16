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
	messaging::Keccak256,
	prelude::Vec,
	router::{IsmpRouter, PostResponse, Request, Response},
};
use alloc::{
	boxed::Box,
	format,
	string::{String, ToString},
};
use codec::{Decode, Encode};
use core::{fmt::Display, str::FromStr, time::Duration};
use primitive_types::H256;

/// Defines the necessary interfaces that must be satisfied by a state machine for it be ISMP
/// compatible.
pub trait IsmpHost: Keccak256 {
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

	/// Should return the host timestamp when this state machine height was committed
	fn state_machine_update_time(
		&self,
		state_machine_height: StateMachineHeight,
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

	/// Checks if a consensus state is frozen should return Ok(()) if it isn't
	/// or [`Error::FrozenConsensusClient`] if it is.
	fn is_consensus_client_frozen(&self, consensus_state_id: ConsensusStateId)
		-> Result<(), Error>;

	/// Should return an error if request commitment does not exist in storage
	fn request_commitment(&self, req: H256) -> Result<(), Error>;

	/// Should return an error if request commitment does not exist in storage
	fn response_commitment(&self, req: H256) -> Result<(), Error>;

	/// Increment and return the next available nonce for an outgoing request.
	fn next_nonce(&self) -> u64;

	/// Should return Some(()) if a receipt for this request exists in storage
	fn request_receipt(&self, req: &Request) -> Option<()>;

	/// Should return Some(()) if a response has been received for the given request
	/// Implementors should store both the request and response objects
	fn response_receipt(&self, res: &Response) -> Option<()>;

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

	/// Store the unbonding period for a consensus state.
	fn store_unbonding_period(
		&self,
		consensus_state_id: ConsensusStateId,
		period: u64,
	) -> Result<(), Error>;

	/// Store the timestamp when the consensus client was updated
	fn store_consensus_update_time(
		&self,
		consensus_state_id: ConsensusStateId,
		timestamp: Duration,
	) -> Result<(), Error>;

	/// Store the timestamp when the state machine height was committed
	fn store_state_machine_update_time(
		&self,
		state_machine_height: StateMachineHeight,
		timestamp: Duration,
	) -> Result<(), Error>;

	/// Store the timestamp when the state machine was updated
	fn store_state_machine_commitment(
		&self,
		height: StateMachineHeight,
		state: StateCommitment,
	) -> Result<(), Error>;

	/// Deletes a state commitment, ideally because it was invalid.
	fn delete_state_commitment(&self, height: StateMachineHeight) -> Result<(), Error>;

	/// Freeze a consensus state with the given identifier
	fn freeze_consensus_client(&self, consensus_state_id: ConsensusStateId) -> Result<(), Error>;

	/// Store latest height for a state machine
	fn store_latest_commitment_height(&self, height: StateMachineHeight) -> Result<(), Error>;

	/// Delete a request commitment from storage, used when a request is timed out.
	/// Make sure to refund the user their relayer fee here.
	/// Returns the scale encoded commitment metadata
	fn delete_request_commitment(&self, req: &Request) -> Result<Vec<u8>, Error>;

	/// Delete a request commitment from storage, used when a response is timed out.
	/// Make sure to refund the user their relayer fee here.
	/// Also delete the request from the responded map.
	/// Returns the scale encoded commitment metadata
	fn delete_response_commitment(&self, res: &PostResponse) -> Result<Vec<u8>, Error>;

	/// Delete a request receipt from storage, used when a request is timed out.
	/// Should only ever be called by a routing state machine
	/// Returns the signer
	fn delete_request_receipt(&self, req: &Request) -> Result<Vec<u8>, Error>;

	/// Delete a response receipt from storage, used when a response is timed out.
	/// Should only ever be called by a routing state machine
	/// Returns the signer
	fn delete_response_receipt(&self, res: &Response) -> Result<Vec<u8>, Error>;

	/// Stores a receipt for an incoming request after it is successfully routed to a module.
	/// Prevents duplicate incoming requests from being processed. Includes the relayer account
	fn store_request_receipt(&self, req: &Request, signer: &Vec<u8>) -> Result<(), Error>;

	/// Stores a receipt that shows that the given request has received a response. Includes the
	/// relayer account
	/// Implementors should map the request commitment to the response object commitment.
	fn store_response_receipt(&self, req: &Response, signer: &Vec<u8>) -> Result<(), Error>;

	/// Stores a commitment for an outgoing request alongside some scale encoded metadata
	fn store_request_commitment(&self, req: &Request, meta: Vec<u8>) -> Result<(), Error>;

	/// Stores a commitment for an outgoing response alongside some scale encoded metadata
	fn store_response_commitment(&self, res: &PostResponse, meta: Vec<u8>) -> Result<(), Error>;

	/// Should return a handle to the consensus client based on the id
	fn consensus_client(&self, id: ConsensusClientId) -> Result<Box<dyn ConsensusClient>, Error> {
		self.consensus_clients()
			.into_iter()
			.find(|client| client.consensus_client_id() == id)
			.ok_or_else(|| Error::Custom(format!("Consensus client for id {id:?} not found")))
	}

	/// Should return the list of all configured consensus clients
	fn consensus_clients(&self) -> Vec<Box<dyn ConsensusClient>>;

	/// Should return the configured delay period for a consensus state
	fn challenge_period(&self, consensus_state_id: ConsensusStateId) -> Option<Duration>;

	/// Set the challenge period in seconds for a consensus state.
	fn store_challenge_period(
		&self,
		consensus_state_id: ConsensusStateId,
		period: u64,
	) -> Result<(), Error>;

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

	/// return the coprocessor state machine that is allowed to proxy requests.
	fn allowed_proxy(&self) -> Option<StateMachine>;

	/// Checks if the host allows this state machine to proxy requests.
	fn is_allowed_proxy(&self, source: &StateMachine) -> bool {
		self.allowed_proxy().map(|proxy| proxy == *source).unwrap_or(false)
	}

	/// Return the unbonding period (i.e the time it takes for a validator's deposit to be unstaked
	/// from the network)
	fn unbonding_period(&self, consensus_state_id: ConsensusStateId) -> Option<Duration>;

	/// Return a handle to the router
	fn ismp_router(&self) -> Box<dyn IsmpRouter>;

	/// Is the current host playing the role of router?
	fn is_router(&self) -> bool {
		self.allowed_proxy()
			.map(|proxy| proxy == self.host_state_machine())
			.unwrap_or(false)
	}
}

/// Currently supported ethereum state machines.
///
/// # IMPORTANT
/// DO NOT REMOVE OR CHANGE THE ORDER OF ANY VARIANTS, THIS WILL BREAK SCALE ENCODING
#[derive(
	Clone,
	Debug,
	Copy,
	Encode,
	Decode,
	PartialOrd,
	Ord,
	PartialEq,
	Eq,
	Hash,
	scale_info::TypeInfo,
	serde::Deserialize,
	serde::Serialize,
)]
pub enum Ethereum {
	/// Ethereum Execution layer
	ExecutionLayer,
	/// The optimism state machine
	Optimism,
	/// The Arbitrum state machine
	Arbitrum,
	/// The Base state machine
	Base,
	/// The Blast state machine
	Blast,
	/// The Mantle state machine
	Mantle,
	/// The Manta state machine
	Manta,
	/// The Build on Bitcoin state machine
	Bob,
}

/// Currently supported state machines.
#[derive(
	Clone,
	Debug,
	Copy,
	Encode,
	Decode,
	PartialOrd,
	Ord,
	PartialEq,
	Eq,
	Hash,
	scale_info::TypeInfo,
	serde::Deserialize,
	serde::Serialize,
)]
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
	/// We identify standalone state machines by their consensus state
	#[codec(index = 3)]
	Grandpa(ConsensusStateId),
	/// State machines chains running on beefy consensus state
	#[codec(index = 4)]
	Beefy(ConsensusStateId),
	#[codec(index = 5)]
	/// Polygon Pos
	Polygon,
	/// Bsc Pos
	#[codec(index = 6)]
	Bsc,
}

impl Display for StateMachine {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let str = match self {
			StateMachine::Ethereum(ethereum) => match ethereum {
				Ethereum::ExecutionLayer => "ETHE".to_string(),
				Ethereum::Arbitrum => "ARBI".to_string(),
				Ethereum::Optimism => "OPTI".to_string(),
				Ethereum::Base => "BASE".to_string(),
				Ethereum::Blast => "BLST".to_string(),
				Ethereum::Mantle => "MNTL".to_string(),
				Ethereum::Manta => "MNTA".to_string(),
				Ethereum::Bob => "BOB".to_string(),
			},
			StateMachine::Polkadot(id) => format!("POLKADOT-{id}"),
			StateMachine::Kusama(id) => format!("KUSAMA-{id}"),
			StateMachine::Grandpa(id) => format!("GRANDPA-{}", u32::from_be_bytes(*id)),
			StateMachine::Beefy(id) => format!("BEEFY-{}", u32::from_be_bytes(*id)),
			StateMachine::Polygon => "POLY".to_string(),
			StateMachine::Bsc => "BSC".to_string(),
		};
		write!(f, "{}", str)
	}
}

impl FromStr for StateMachine {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = match s {
			"ETHE" => StateMachine::Ethereum(Ethereum::ExecutionLayer),
			"ARBI" => StateMachine::Ethereum(Ethereum::Arbitrum),
			"OPTI" => StateMachine::Ethereum(Ethereum::Optimism),
			"BASE" => StateMachine::Ethereum(Ethereum::Base),
			"BLST" => StateMachine::Ethereum(Ethereum::Blast),
			"MNTL" => StateMachine::Ethereum(Ethereum::Mantle),
			"MNTA" => StateMachine::Ethereum(Ethereum::Manta),
			"BOB" => StateMachine::Ethereum(Ethereum::Bob),
			"POLY" => StateMachine::Polygon,
			"BSC" => StateMachine::Bsc,
			name if name.starts_with("POLKADOT-") => {
				let id = name
					.split('-')
					.last()
					.and_then(|id| u32::from_str(id).ok())
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				StateMachine::Polkadot(id)
			},
			name if name.starts_with("KUSAMA-") => {
				let id = name
					.split('-')
					.last()
					.and_then(|id| u32::from_str(id).ok())
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				StateMachine::Kusama(id)
			},
			name if name.starts_with("GRANDPA-") => {
				let id = name
					.split('-')
					.last()
					.and_then(|id| u32::from_str(id).ok().map(u32::to_be_bytes))
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				StateMachine::Grandpa(id)
			},
			name if name.starts_with("BEEFY-") => {
				let id = name
					.split('-')
					.last()
					.and_then(|id| u32::from_str(id).ok().map(u32::to_be_bytes))
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				StateMachine::Beefy(id)
			},
			name => Err(format!("Unknown state machine: {name}"))?,
		};

		Ok(s)
	}
}

#[cfg(test)]
mod tests {
	use crate::host::{Ethereum, StateMachine};
	use alloc::string::ToString;
	use core::str::FromStr;

	#[test]
	fn state_machine_conversions() {
		let grandpa = StateMachine::Grandpa(*b"hybr");
		let beefy = StateMachine::Beefy(*b"hybr");
		let eth = StateMachine::Ethereum(Ethereum::ExecutionLayer);
		let arb = StateMachine::Ethereum(Ethereum::Arbitrum);
		let op = StateMachine::Ethereum(Ethereum::Optimism);
		let base = StateMachine::Ethereum(Ethereum::Base);

		let grandpa_string = grandpa.to_string();
		let beefy_string = beefy.to_string();
		let eth_str = eth.to_string();
		let arb_str = arb.to_string();
		let op_str = op.to_string();
		let base_str = base.to_string();

		dbg!(&grandpa_string);
		dbg!(&beefy_string);

		assert_eq!(grandpa, StateMachine::from_str(&grandpa_string).unwrap());
		assert_eq!(beefy, StateMachine::from_str(&beefy_string).unwrap());
		assert_eq!(eth, StateMachine::from_str(&eth_str).unwrap());
		assert_eq!(arb, StateMachine::from_str(&arb_str).unwrap());
		assert_eq!(op, StateMachine::from_str(&op_str).unwrap());
		assert_eq!(base, StateMachine::from_str(&base_str).unwrap());
	}
}
