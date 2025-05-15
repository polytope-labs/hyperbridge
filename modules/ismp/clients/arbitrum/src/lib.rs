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
use alloy_sol_types::SolValue;
use anyhow::anyhow;
use evm_state_machine::{derive_map_key, get_contract_account, get_value_from_proof, prelude::*};
use geth_primitives::{CodecHeader, Header};
use ismp::{
	consensus::{
		ConsensusStateId, IntermediateState, StateCommitment, StateMachineHeight, StateMachineId,
	},
	error::Error,
	host::StateMachine,
	messaging::Keccak256,
};
use polkadot_sdk::*;
use primitive_types::{H160, H256, U256};

/// Storage layout slot for the nodes map in the Rollup Contract
pub const NODES_SLOT: u64 = 118;
/// Storage layout slot for the _assertions map in the Rollup Contract
pub const ASSERTIONS_SLOT: u64 = 117;

#[derive(codec::Encode, codec::Decode, Debug, Clone)]
pub struct GlobalState {
	pub block_hash: H256,
	pub send_root: H256,
	pub inbox_position: u64,
	pub position_in_message: u64,
}

impl GlobalState {
	/// https://github.com/OffchainLabs/nitro/blob/5e9f4228e6418b114a5aea0aa7f2f0cc161b67c0/contracts/src/state/GlobalState.sol#L16
	pub fn hash<H: Keccak256>(&self) -> H256 {
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

#[derive(codec::Encode, codec::Decode, Debug, Clone)]
pub enum MachineStatus {
	Running = 0,
	Finished = 1,
	Errored = 2,
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
fn get_state_hash<H: Keccak256>(
	global_state: GlobalState,
	machine_status: MachineStatus,
	inbox_max_count: U256,
) -> H256 {
	// abi encode packed
	let mut buf = Vec::new();
	buf.extend_from_slice(&global_state.hash::<H>()[..]);
	let inbox = inbox_max_count.to_big_endian();
	buf.extend_from_slice(&inbox);
	buf.extend_from_slice((machine_status as u8).to_be_bytes().as_slice());
	H::keccak256(&buf)
}

pub fn verify_arbitrum_payload<H: Keccak256 + Send + Sync>(
	payload: ArbitrumPayloadProof,
	root: H256,
	rollup_core_address: H160,
	consensus_state_id: ConsensusStateId,
) -> Result<IntermediateState, Error> {
	let storage_root =
		get_contract_account::<H>(payload.contract_proof, &rollup_core_address.0, root)?
			.storage_root
			.0
			.into();

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

	let key = U256::from(payload.node_number).to_big_endian();
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

#[derive(codec::Encode, codec::Decode, Debug)]
pub struct ArbitrumBoldProof {
	/// Arbitrum header that corresponds to the node being created
	pub arbitrum_header: CodecHeader,
	/// After State as recorded in the AssertionCreated event that was emitted for this node
	pub after_state: AssertionState,
	/// Previous assertion hash
	pub previous_assertion_hash: H256,
	/// Sequencer batch acc as recorded in the AssertionCreated event that was emitted for this
	/// node
	pub sequencer_batch_acc: H256,
	/// Proof for the assertion hash field in the _assertions map in the
	/// RollupCore
	pub storage_proof: Vec<Vec<u8>>,
	/// RollupCore contract proof in the ethereum world trie
	pub contract_proof: Vec<Vec<u8>>,
}

// https://github.com/OffchainLabs/nitro-contracts/blob/780366a0c40caf694ed544a6a1d52c0de56573ba/src/rollup/AssertionState.sol#L11
#[derive(codec::Encode, codec::Decode, Debug, Clone)]
pub struct AssertionState {
	pub global_state: GlobalState,
	pub machine_status: MachineStatus,
	pub end_history_root: H256,
}

alloy_sol_macro::sol! {
	#![sol(all_derives)]

	enum MachineStatusSol {
		RUNNING,
		FINISHED,
		ERRORED
	}

	struct GlobalStateSol {
		bytes32[2] bytes32Vals;
		uint64[2] u64Vals;
	}

	struct AssertionStateSol {
		GlobalStateSol globalState;
		MachineStatusSol machineStatus;
		bytes32 endHistoryRoot;
	}

}

impl From<MachineStatus> for MachineStatusSol {
	fn from(value: MachineStatus) -> Self {
		match value {
			MachineStatus::Running => MachineStatusSol::RUNNING,
			MachineStatus::Finished => MachineStatusSol::FINISHED,
			MachineStatus::Errored => MachineStatusSol::ERRORED,
		}
	}
}

impl From<GlobalState> for GlobalStateSol {
	fn from(value: GlobalState) -> Self {
		GlobalStateSol {
			bytes32Vals: [value.block_hash.0.into(), value.send_root.0.into()],
			u64Vals: [value.inbox_position, value.position_in_message],
		}
	}
}

impl From<AssertionState> for AssertionStateSol {
	fn from(value: AssertionState) -> Self {
		AssertionStateSol {
			globalState: value.global_state.into(),
			machineStatus: value.machine_status.into(),
			endHistoryRoot: value.end_history_root.0.into(),
		}
	}
}

impl AssertionState {
	fn hash(&self) -> H256 {
		sp_io::hashing::keccak_256(&self.abi_encode()).into()
	}

	fn abi_encode(&self) -> Vec<u8> {
		let assertion_state_sol: AssertionStateSol = self.clone().into();
		assertion_state_sol.abi_encode()
	}
}

// https://github.com/OffchainLabs/nitro-contracts/blob/109a8a36cd4c6a2a0d2b5003b01adee60d83e2a1/src/rollup/RollupLib.sol#L33
fn compute_assertion_hash(
	previous_assertion_hash: H256,
	after_state_hash: H256,
	sequencer_batch_acc: H256,
) -> H256 {
	let mut buf = Vec::new();
	buf.extend_from_slice(&previous_assertion_hash[..]);
	buf.extend_from_slice(&after_state_hash[..]);
	buf.extend_from_slice(&sequencer_batch_acc[..]);
	sp_io::hashing::keccak_256(&buf).into()
}

pub fn verify_arbitrum_bold<H: Keccak256 + Send + Sync>(
	payload: ArbitrumBoldProof,
	root: H256,
	rollup_core_address: H160,
	consensus_state_id: ConsensusStateId,
) -> Result<IntermediateState, anyhow::Error> {
	let storage_root =
		get_contract_account::<H>(payload.contract_proof, &rollup_core_address.0, root)?
			.storage_root
			.0
			.into();

	let header: Header = payload.arbitrum_header.as_ref().into();
	if &payload.after_state.global_state.send_root[..] != &payload.arbitrum_header.extra_data {
		Err(anyhow!("Arbitrum header extra data does not match send root in global state",))?
	}

	let block_number = payload.arbitrum_header.number.low_u64();
	let timestamp = payload.arbitrum_header.timestamp;
	let state_root = payload.arbitrum_header.state_root.0.into();

	let header_hash = header.hash::<H>();
	if payload.after_state.global_state.block_hash != header_hash {
		Err(anyhow!("Arbitrum header hash does not match block hash in global state",))?
	}

	let assertion_hash = compute_assertion_hash(
		payload.previous_assertion_hash,
		payload.after_state.hash(),
		payload.sequencer_batch_acc,
	);

	let assertion_hash_key = derive_map_key::<H>(assertion_hash.0.to_vec(), ASSERTIONS_SLOT);

	// Only valid assertions nodes are inserted in the rollup storage
	// A Some() value from the proof asserts that this assertion is valid and exists in storage
	// https://github.com/OffchainLabs/nitro-contracts/blob/94999b3e2d3b4b7f8e771cc458b9eb229620dd8f/src/rollup/RollupCore.sol#L542

	get_value_from_proof::<H>(assertion_hash_key.0.to_vec(), storage_root, payload.storage_proof)?
		.ok_or_else(|| anyhow!("Assertion provided is invalid"))?;

	Ok(IntermediateState {
		height: StateMachineHeight {
			id: StateMachineId {
				// note: This default state machine id should not be used to store the state
				// commitment
				state_id: StateMachine::Evm(Default::default()),
				consensus_state_id,
			},
			height: block_number,
		},
		commitment: StateCommitment { timestamp, overlay_root: None, state_root },
	})
}
