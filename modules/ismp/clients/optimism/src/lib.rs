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

use alloc::format;
use alloy_rlp::Decodable;
use evm_state_machine::{
	derive_array_item_key, derive_map_key, get_contract_account, get_value_from_proof, prelude::*,
};
use geth_primitives::{CodecHeader, Header};
use ismp::{
	consensus::{
		ConsensusStateId, IntermediateState, StateCommitment, StateMachineHeight, StateMachineId,
	},
	error::Error,
	host::StateMachine,
	messaging::Keccak256,
};
use primitive_types::{H160, H256, U128, U256};

// Constants

/// Slot for the disputeGames map in DisputeFactory contract
pub const DISPUTE_GAMES_SLOT: u64 = 103;
/// Slot for the l2Outputs array in the L2Oracle contract
pub const L2_OUTPUTS_SLOT: u64 = 3;

#[derive(codec::Encode, codec::Decode, Debug)]
pub struct OptimismPayloadProof {
	/// Actual state root of the optimism execution layer
	pub state_root: H256,
	/// Storage root hash of the optimism withdrawal contracts
	pub withdrawal_storage_root: H256,
	/// Optimism Block hash at which the values aboved were fetched
	pub l2_block_hash: H256,
	/// L2Oracle contract version
	pub version: H256,
	/// Membership Proof for the L2Oracle contract account in the ethereum world trie
	pub l2_oracle_proof: Vec<Vec<u8>>,
	/// Membership proof for output root in l2Outputs array
	pub output_root_proof: Vec<Vec<u8>>,
	/// Membership proof Timestamp and block number in the l2Outputs array
	pub multi_proof: Vec<Vec<u8>>,
	/// Index of the output root that needs to be proved in the l2Outputs array
	pub output_root_index: u64,
	/// Block number
	pub block_number: u64,
	/// Timestamp
	pub timestamp: u64,
}

pub fn verify_optimism_payload<H: Keccak256 + Send + Sync>(
	payload: OptimismPayloadProof,
	root: H256,
	l2_oracle_address: H160,
	consensus_state_id: ConsensusStateId,
) -> Result<IntermediateState, Error> {
	let storage_root =
		get_contract_account::<H>(payload.l2_oracle_proof, &l2_oracle_address.0, root)?
			.storage_root
			.0
			.into();

	let output_root = calculate_output_root::<H>(
		payload.version,
		payload.state_root,
		payload.withdrawal_storage_root,
		payload.l2_block_hash,
	);
	let output_root_key = derive_array_item_key::<H>(L2_OUTPUTS_SLOT, payload.output_root_index, 0);

	let proof_value = match get_value_from_proof::<H>(
		output_root_key,
		storage_root,
		payload.output_root_proof,
	)? {
		Some(value) => value.clone(),
		_ => Err(Error::MembershipProofVerificationFailed("Value not found in proof".to_string()))?,
	};

	let proof_value = <alloy_primitives::U256 as Decodable>::decode(&mut &*proof_value)
		.map_err(|_| Error::Custom(format!("Error decoding output root from {:?}", &proof_value)))?
		.to_be_bytes::<32>();

	if proof_value != output_root.0 {
		return Err(Error::MembershipProofVerificationFailed(
			"Invalid optimism output root proof".to_string(),
		));
	}

	// verify timestamp and block number
	let timestamp_block_number_key =
		derive_array_item_key::<H>(L2_OUTPUTS_SLOT, payload.output_root_index, 1);
	let block_and_timestamp = match get_value_from_proof::<H>(
		timestamp_block_number_key,
		storage_root,
		payload.multi_proof,
	)? {
		Some(value) => value.clone(),
		_ => Err(Error::MembershipProofVerificationFailed("Value not found in proof".to_string()))?,
	};

	let block_and_timestamp =
		<alloy_primitives::U256 as Decodable>::decode(&mut &*block_and_timestamp)
			.map_err(|_| {
				Error::Custom(format!(
					"Error decoding block and timestamp from{:?}",
					&block_and_timestamp
				))
			})?
			.to_be_bytes::<32>();

	let block_and_timestamp = U256::from_big_endian(&block_and_timestamp);
	// Timestamp is contained in the first two u64 values
	let timestamp = block_and_timestamp.low_u128() as u64;

	// Block number occupies the last two u64 values
	let mut block_number = [0u64; 2];
	block_number.copy_from_slice(&block_and_timestamp.0[2..]);
	let block_number = U128(block_number).as_u128() as u64;

	if payload.timestamp != timestamp && payload.block_number != block_number {
		return Err(Error::MembershipProofVerificationFailed(
			"Invalid optimism block and timestamp proof".to_string(),
		));
	}

	Ok(IntermediateState {
		height: StateMachineHeight {
			id: StateMachineId {
				// note: This will state machine id should not be used to store the state commitment
				state_id: StateMachine::Evm(Default::default()),
				consensus_state_id,
			},
			height: payload.block_number,
		},
		commitment: StateCommitment {
			timestamp: payload.timestamp,
			overlay_root: None,
			state_root: payload.state_root,
		},
	})
}

#[derive(codec::Encode, codec::Decode, Debug, Clone)]
pub struct OptimismDisputeGameProof {
	/// Op stack header
	pub header: CodecHeader,
	/// Storage root hash of the optimism withdrawal contracts
	pub withdrawal_storage_root: H256,
	/// L2Oracle contract version
	pub version: H256,
	/// Membership Proof for the DisputeFactory contract account in the ethereum world trie
	pub dispute_factory_proof: Vec<Vec<u8>>,
	/// Membership proof for dispute game in disputeGames map
	pub dispute_game_proof: Vec<Vec<u8>>,
	/// Dispute game proxy address
	pub proxy: H160,
	/// Extra data that was used in initializing the dispute game
	pub extra_data: Vec<u8>,
	/// Game type
	pub game_type: u32,
	/// L1 Timestamp at game creation
	pub timestamp: u64,
}

// https://github.com/ethereum-optimism/optimism/blob/f707883038d527cbf1e9f8ea513fe33255deadbc/packages/contracts-bedrock/src/dispute/DisputeGameFactory.sol#L127
pub fn get_game_uuid<H: Keccak256>(game_type: u32, root_claim: H256, extra_data: Vec<u8>) -> H256 {
	let tokens = [
		ethabi::Token::Uint(game_type.into()),
		ethabi::Token::FixedBytes(root_claim.0.to_vec()),
		ethabi::Token::Bytes(extra_data),
	];
	let encoded = ethabi::encode(&tokens);
	H::keccak256(&encoded)
}

pub fn calculate_output_root<H: Keccak256>(
	version: H256,
	state_root: H256,
	withdrawal_storage_root: H256,
	l2_block_hash: H256,
) -> H256 {
	let mut buf = Vec::with_capacity(128);
	buf.extend_from_slice(&version[..]);
	buf.extend_from_slice(&state_root[..]);
	buf.extend_from_slice(&withdrawal_storage_root[..]);
	buf.extend_from_slice(&l2_block_hash[..]);

	H::keccak256(&buf)
}

// https://github.com/ethereum-optimism/optimism/blob/f707883038d527cbf1e9f8ea513fe33255deadbc/packages/contracts-bedrock/src/libraries/DisputeTypes.sol#L94
/// Game types
pub const CANNON: u32 = 0;
pub const _PERMISSIONED: u32 = 1;

pub fn verify_optimism_dispute_game_proof<H: Keccak256 + Send + Sync>(
	payload: OptimismDisputeGameProof,
	root: H256,
	dispute_factory_address: H160,
	respected_game_types: Vec<u32>,
	consensus_state_id: ConsensusStateId,
) -> Result<IntermediateState, Error> {
	// Is the game type the respected game types?
	if !respected_game_types.contains(&payload.game_type) {
		Err(Error::MembershipProofVerificationFailed(
			"Game type must be the respected game type".to_string(),
		))?;
	}
	let storage_root =
		get_contract_account::<H>(payload.dispute_factory_proof, &dispute_factory_address.0, root)?
			.storage_root
			.0
			.into();
	let l2_block_hash = Header::from(&payload.header).hash::<H>();

	let root_claim = calculate_output_root::<H>(
		payload.version,
		payload.header.state_root,
		payload.withdrawal_storage_root,
		l2_block_hash,
	);

	let game_uuid = get_game_uuid::<H>(payload.game_type, root_claim, payload.extra_data);

	let dispute_game_key = derive_map_key::<H>(game_uuid.0.to_vec(), DISPUTE_GAMES_SLOT);

	// Does the dispute game's unique identifier exist in the _disputeGames map?
	let proof_value = match get_value_from_proof::<H>(
		dispute_game_key.0.to_vec(),
		storage_root,
		payload.dispute_game_proof,
	)? {
		Some(value) => value.clone(),
		_ => Err(Error::MembershipProofVerificationFailed(
			"Dispute Game's Id not found in proof".to_string(),
		))?,
	};

	let mut encoded_game_id = <alloy_primitives::Bytes as Decodable>::decode(&mut &*proof_value)
		.map_err(|_| {
			Error::Custom(format!("Error decoding dispute game id from {:?}", &proof_value))
		})?
		.0
		.to_vec();

	let game_id = get_game_id(payload.game_type, payload.timestamp, payload.proxy);
	let game_id_bytes = game_id.to_big_endian();

	// Pad the encoded game id gotten from proof with zeros so it becomes 32 bytes long
	(0..game_id_bytes.len().saturating_sub(encoded_game_id.len()))
		.for_each(|_| encoded_game_id.insert(0, 0));

	// Derived game id must be equal to encoded game id
	if encoded_game_id != game_id_bytes {
		Err(Error::MembershipProofVerificationFailed(
			"Dispute Game Id from proof does not match derived game id".to_string(),
		))?
	}

	Ok(IntermediateState {
		height: StateMachineHeight {
			id: StateMachineId {
				// note: This will state machine id should not be used to store the state commitment
				state_id: StateMachine::Evm(Default::default()),
				consensus_state_id,
			},
			height: payload.header.number.low_u64(),
		},
		commitment: StateCommitment {
			timestamp: payload.header.timestamp,
			overlay_root: None,
			state_root: payload.header.state_root,
		},
	})
}

// https://github.com/ethereum-optimism/optimism/blob/f707883038d527cbf1e9f8ea513fe33255deadbc/packages/contracts-bedrock/src/dispute/lib/LibGameId.sol#L15
fn get_game_id(game_type: u32, timestamp: u64, game_proxy: H160) -> U256 {
	let mut bytes = U256::zero();
	// Use bitwise shifts and bitwise OR for packing
	bytes |= U256::from(game_type) << 224;
	bytes |= U256::from(timestamp) << 160;

	let mut addr = vec![0u8; 12];
	addr.extend_from_slice(&game_proxy.0);
	let proxy = U256::from_big_endian(&addr);

	bytes |= proxy;
	bytes
}
