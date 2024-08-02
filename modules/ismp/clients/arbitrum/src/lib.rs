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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]
extern crate alloc;

#[cfg(test)]
mod tests;

use alloc::format;
use alloy_rlp::Decodable;
use ethabi::ethereum_types::{H160, H256, U256};
use evm_common::{derive_map_key, get_contract_storage_root, get_value_from_proof, prelude::*};
use geth_primitives::{CodecHeader, Header};
use ismp::{
	consensus::{
		ConsensusStateId, IntermediateState, StateCommitment, StateMachineHeight, StateMachineId,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
};

/// Storage layout slot for the nodes map in the Rollup Contract
pub const NODES_SLOT: u64 = 118;

#[derive(codec::Encode, codec::Decode, Debug)]
pub struct GlobalState {
	pub block_hash: H256,
	pub send_root: H256,
	pub inbox_position: u64,
	pub position_in_message: u64,
}

impl GlobalState {
	/// https://github.com/OffchainLabs/nitro/blob/5e9f4228e6418b114a5aea0aa7f2f0cc161b67c0/contracts/src/state/GlobalState.sol#L16
	pub fn hash<H: IsmpHost>(&self) -> H256 {
		// abi encode packed
		let mut buf = Vec::new();
		buf.extend_from_slice("Global state:".as_bytes());
		buf.extend_from_slice(&self.block_hash[..]);
		buf.extend_from_slice(&self.send_root[..]);
		buf.extend_from_slice(&self.inbox_position.to_be_bytes()[..]);
		buf.extend_from_slice(&self.position_in_message.to_be_bytes()[..]);
		H::keccak256(&buf)
	}
}

#[derive(codec::Encode, codec::Decode, Debug)]
pub enum MachineStatus {
	Running = 0,
	Finished = 1,
	Errored = 2,
	TooFar = 3,
}

impl TryFrom<u8> for MachineStatus {
	type Error = &'static str;

	fn try_from(status: u8) -> Result<Self, Self::Error> {
		if status == 0 {
			Ok(MachineStatus::Running)
		} else if status == 1 {
			Ok(MachineStatus::Finished)
		} else if status == 2 {
			Ok(MachineStatus::Errored)
		} else if status == 3 {
			Ok(MachineStatus::TooFar)
		} else {
			Err("Invalid machine status received")
		}
	}
}

#[derive(codec::Encode, codec::Decode, Debug)]
pub struct ArbitrumPayloadProof {
	/// Arbitrum header that corresponds to the node being created
	pub arbitrum_header: CodecHeader,
	/// Global State as recorded in the NodeCreated event that was emitted for this node
	pub global_state: GlobalState,
	/// Machine status as recorded in the NodeCreated event that was emitted for this node
	pub machine_status: MachineStatus,
	/// Inbox max count as recorded in the NodeCreated event that was emitted for this node
	pub inbox_max_count: U256,
	/// Key used to store the node  in the _nodes mapping in the RollupCore as recorded in the
	/// latestNodeCreated field of the NodeCreated event
	pub node_number: u64,
	/// Proof for the state_hash field in the Node struct inside the _nodes mapping in the
	/// RollupCore
	pub storage_proof: Vec<Vec<u8>>,
	/// RollupCore contract proof in the ethereum world trie
	pub contract_proof: Vec<Vec<u8>>,
}

/// https://github.com/OffchainLabs/nitro/blob/5e9f4228e6418b114a5aea0aa7f2f0cc161b67c0/contracts/src/rollup/RollupLib.sol#L59
fn get_state_hash<H: IsmpHost>(
	global_state: GlobalState,
	machine_status: MachineStatus,
	inbox_max_count: U256,
) -> H256 {
	// abi encode packed
	let mut buf = Vec::new();
	buf.extend_from_slice(&global_state.hash::<H>()[..]);
	let mut inbox = [0u8; 32];
	inbox_max_count.to_big_endian(&mut inbox);
	buf.extend_from_slice(&inbox);
	buf.extend_from_slice((machine_status as u8).to_be_bytes().as_slice());
	H::keccak256(&buf)
}

pub fn verify_arbitrum_payload<H: IsmpHost + Send + Sync>(
	payload: ArbitrumPayloadProof,
	root: H256,
	rollup_core_address: H160,
	consensus_state_id: ConsensusStateId,
) -> Result<IntermediateState, Error> {
	let storage_root =
		get_contract_storage_root::<H>(payload.contract_proof, &rollup_core_address.0, root)?;

	let header: Header = payload.arbitrum_header.as_ref().into();
	if &payload.global_state.send_root[..] != &payload.arbitrum_header.extra_data {
		Err(Error::Custom(
			"Arbitrum header extra data does not match send root in global state".to_string(),
		))?
	}

	let block_number = payload.arbitrum_header.number.low_u64();
	let timestamp = payload.arbitrum_header.timestamp;
	let state_root = payload.arbitrum_header.state_root.0.into();

	let header_hash = header.hash::<H>();
	if payload.global_state.block_hash != header_hash {
		Err(Error::Custom(
			"Arbitrum header hash does not match block hash in global state".to_string(),
		))?
	}

	let state_hash =
		get_state_hash::<H>(payload.global_state, payload.machine_status, payload.inbox_max_count);

	let mut key = [0u8; 32];
	U256::from(payload.node_number).to_big_endian(&mut key);
	let state_hash_key = derive_map_key::<H>(key.to_vec(), NODES_SLOT);
	let proof_value = match get_value_from_proof::<H>(
		state_hash_key.0.to_vec(),
		storage_root,
		payload.storage_proof,
	)? {
		Some(value) => value.clone(),
		_ => Err(Error::MembershipProofVerificationFailed("Value not found in proof".to_string()))?,
	};

	let proof_value = <alloy_primitives::U256 as Decodable>::decode(&mut &*proof_value)
		.map_err(|_| Error::Custom(format!("Error decoding state hash {:?}", &proof_value)))?
		.to_be_bytes::<32>();

	if proof_value != state_hash.0 {
		Err(Error::MembershipProofVerificationFailed(
			"State hash from proof does not match calculated state hash".to_string(),
		))?
	}

	Ok(IntermediateState {
		height: StateMachineHeight {
			id: StateMachineId {
				// note: This will state machine id should not be used to store the state commitment
				state_id: StateMachine::Evm(Default::default()),
				consensus_state_id,
			},
			height: block_number,
		},
		commitment: StateCommitment { timestamp, overlay_root: None, state_root },
	})
}
