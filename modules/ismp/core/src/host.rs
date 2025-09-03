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
use alloc::{boxed::Box, format, string::String};
use codec::{Decode, DecodeWithMemTracking, Encode};
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
	fn store_request_receipt(&self, req: &Request, signer: &Vec<u8>) -> Result<Vec<u8>, Error>;

	/// Stores a receipt that shows that the given request has received a response. Includes the
	/// relayer account
	/// Implementors should map the request commitment to the response object commitment.
	fn store_response_receipt(&self, req: &Response, signer: &Vec<u8>) -> Result<Vec<u8>, Error>;

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
	fn challenge_period(&self, state_machine: StateMachineId) -> Option<Duration>;

	/// Set the challenge period in seconds for a consensus state.
	fn store_challenge_period(
		&self,
		state_machine: StateMachineId,
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

	/// Should return the previous height of the state machine
	fn previous_commitment_height(&self, id: StateMachineId) -> Option<u64>;
}

/// Currently supported state machines.
#[derive(
	Clone,
	Debug,
	Copy,
	Encode,
	Decode,
	DecodeWithMemTracking,
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
	/// Evm state machines
	#[codec(index = 0)]
	Evm(u32),
	/// Polkadot parachains
	#[codec(index = 1)]
	Polkadot(u32),
	/// Kusama parachains
	#[codec(index = 2)]
	Kusama(u32),
	/// Substrate-based standalone chain
	#[codec(index = 3)]
	Substrate(ConsensusStateId),
	/// Tendermint chains
	#[codec(index = 4)]
	Tendermint(ConsensusStateId),
	/// Alternative relaychain parachains
	/// The state machine id also includes the consensus state id to prevent name clashes
	#[codec(index = 5)]
	Relay {
		/// Consensus state id
		relay: ConsensusStateId,
		/// Parachain Id
		para_id: u32,
	},
}

impl StateMachine {
	/// Check if the state machine is evm based.
	pub fn is_evm(&self) -> bool {
		match self {
			StateMachine::Evm(_) => true,
			_ => false,
		}
	}

	/// Check if the state machine is substrate-based
	pub fn is_substrate(&self) -> bool {
		match self {
			StateMachine::Polkadot(_) |
			StateMachine::Kusama(_) |
			StateMachine::Substrate(_) |
			StateMachine::Relay { .. } => true,
			_ => false,
		}
	}
}

impl Display for StateMachine {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let str = match self {
			StateMachine::Evm(id) => {
				format!("EVM-{id}")
			},
			StateMachine::Polkadot(id) => format!("POLKADOT-{id}"),
			StateMachine::Kusama(id) => format!("KUSAMA-{id}"),
			StateMachine::Substrate(id) => {
				format!(
					"SUBSTRATE-{}",
					String::from_utf8(id.to_vec()).map_err(|_| core::fmt::Error)?
				)
			},
			StateMachine::Tendermint(id) => format!(
				"TNDRMINT-{}",
				String::from_utf8(id.to_vec()).map_err(|_| core::fmt::Error)?
			),
			StateMachine::Relay { relay, para_id } => format!(
				"RELAY-{}-{para_id}",
				String::from_utf8(relay.to_vec()).map_err(|_| core::fmt::Error)?
			),
		};
		write!(f, "{}", str)
	}
}

impl FromStr for StateMachine {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = match s {
			name if name.starts_with("EVM-") => {
				let id = name
					.split('-')
					.last()
					.and_then(|id| u32::from_str(id).ok())
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				StateMachine::Evm(id)
			},
			name if name.starts_with("POLKADOT-") => {
				let id = name
					.split('-')
					.last()
					.and_then(|id| u32::from_str(id).ok())
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				StateMachine::Polkadot(id)
			},

			name if name.starts_with("RELAY-") => {
				let values = name.split('-').collect::<Vec<_>>();
				let id = values
					.last()
					.and_then(|id| u32::from_str(id).ok())
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				let relay = values
					.get(1)
					.and_then(|id| {
						let bytes = id.as_bytes();
						if bytes.len() == 4 {
							let mut dest = [0u8; 4];
							dest.copy_from_slice(bytes);
							Some(dest)
						} else {
							None
						}
					})
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				StateMachine::Relay { relay, para_id: id }
			},
			name if name.starts_with("KUSAMA-") => {
				let id = name
					.split('-')
					.last()
					.and_then(|id| u32::from_str(id).ok())
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				StateMachine::Kusama(id)
			},
			name if name.starts_with("SUBSTRATE-") => {
				let name = name
					.split('-')
					.last()
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				let mut id = [0u8; 4];
				id.copy_from_slice(name.as_bytes());
				StateMachine::Substrate(id)
			},
			name if name.starts_with("TNDRMINT-") => {
				let name = name
					.split('-')
					.last()
					.ok_or_else(|| format!("invalid state machine: {name}"))?;
				let mut id = [0u8; 4];
				id.copy_from_slice(name.as_bytes());
				StateMachine::Tendermint(id)
			},
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
		let grandpa = StateMachine::Substrate(*b"hybr");
		let beefy = StateMachine::Tendermint(*b"hybr");
		let solo_relay = StateMachine::Relay { relay: *b"CENJ", para_id: 1000 };

		let grandpa_string = grandpa.to_string();
		let beefy_string = beefy.to_string();
		let solo_string = solo_relay.to_string();
		dbg!(&grandpa_string);
		dbg!(&beefy_string);
		dbg!(&solo_string);

		assert_eq!(grandpa, StateMachine::from_str(&grandpa_string).unwrap());
		assert_eq!(beefy, StateMachine::from_str(&beefy_string).unwrap());
		assert_eq!(solo_relay, StateMachine::from_str(&solo_string).unwrap());
	}
}
