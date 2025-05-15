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

use crate::{
	prelude::*,
	presets::{
		REQUEST_COMMITMENTS_SLOT, REQUEST_RECEIPTS_SLOT, RESPONSE_COMMITMENTS_SLOT,
		RESPONSE_RECEIPTS_SLOT,
	},
	types::{Account, EvmStateProof, KeccakHasher},
};
use alloc::{format, string::ToString};
use alloy_rlp::Decodable;
use codec::Decode;
use ethereum_triedb::{EIP1186Layout, StorageProof};
use ismp::{
	consensus::{
		ConsensusStateId, IntermediateState, StateCommitment, StateMachineHeight, StateMachineId,
	},
	error::Error,
	host::StateMachine,
	messaging::{hash_request, hash_response, Keccak256, Proof},
	router::RequestResponse,
};
use primitive_types::{H256, U256};
use trie_db::{DBValue, Trie, TrieDBBuilder};

pub fn construct_intermediate_state(
	state_id: StateMachine,
	consensus_state_id: ConsensusStateId,
	height: u64,
	timestamp: u64,
	state_root: &[u8],
) -> Result<IntermediateState, Error> {
	let state_machine_id = StateMachineId { state_id, consensus_state_id };

	let state_machine_height = StateMachineHeight { id: state_machine_id, height };

	let state_commitment = StateCommitment {
		timestamp,
		overlay_root: None,
		state_root: to_bytes_32(&state_root[..])?.into(),
	};

	let intermediate_state =
		IntermediateState { height: state_machine_height, commitment: state_commitment };

	Ok(intermediate_state)
}

pub fn decode_evm_state_proof(proof: &Proof) -> Result<EvmStateProof, Error> {
	let evm_state_proof = EvmStateProof::decode(&mut &proof.proof[..])
		.map_err(|_| Error::Custom(format!("Cannot decode evm state proof")))?;

	Ok(evm_state_proof)
}

pub fn req_res_commitment_key<H: Keccak256>(item: RequestResponse) -> Vec<Vec<u8>> {
	let mut keys = vec![];
	match item {
		RequestResponse::Request(requests) =>
			for req in requests {
				let commitment = hash_request::<H>(&req);
				let key = derive_map_key_with_offset::<H>(
					commitment.0.to_vec(),
					REQUEST_COMMITMENTS_SLOT,
					1,
				);
				keys.push(key.0.to_vec())
			},
		RequestResponse::Response(responses) =>
			for res in responses {
				let commitment = hash_response::<H>(&res);
				let key = derive_map_key_with_offset::<H>(
					commitment.0.to_vec(),
					RESPONSE_COMMITMENTS_SLOT,
					1,
				);
				keys.push(key.0.to_vec())
			},
	}

	keys
}

pub fn req_res_receipt_keys<H: Keccak256>(item: RequestResponse) -> Vec<Vec<u8>> {
	let mut keys = vec![];
	match item {
		RequestResponse::Request(requests) =>
			for req in requests {
				let commitment = hash_request::<H>(&req);
				let key =
					derive_unhashed_map_key::<H>(commitment.0.to_vec(), REQUEST_RECEIPTS_SLOT);
				keys.push(key.0.to_vec())
			},
		RequestResponse::Response(responses) =>
			for res in responses {
				let commitment = hash_request::<H>(&res.request());
				let key =
					derive_unhashed_map_key::<H>(commitment.0.to_vec(), RESPONSE_RECEIPTS_SLOT);
				keys.push(key.0.to_vec())
			},
	}

	keys
}

pub(super) fn to_bytes_32(bytes: &[u8]) -> Result<[u8; 32], Error> {
	if bytes.len() != 32 {
		return Err(Error::Custom(format!(
			"Input vector must have exactly 32 elements {:?}",
			bytes
		)));
	}

	let mut array = [0u8; 32];

	array.copy_from_slice(&bytes);

	Ok(array)
}

pub fn get_contract_account<H: Keccak256 + Send + Sync>(
	contract_account_proof: Vec<Vec<u8>>,
	contract_address: &[u8],
	root: H256,
) -> Result<Account, Error> {
	let db = StorageProof::new(contract_account_proof).into_memory_db::<KeccakHasher<H>>();
	let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher<H>>>::new(&db, &root).build();
	let key = H::keccak256(contract_address).0;
	let result = trie
		.get(&key)
		.map_err(|_| Error::Custom("Invalid contract account proof".to_string()))?
		.ok_or_else(|| Error::Custom("Contract account is not present in proof".to_string()))?;

	let contract_account = <Account as Decodable>::decode(&mut &*result).map_err(|_| {
		Error::Custom(format!("Error decoding contract account from value {:?}", &result))
	})?;

	Ok(contract_account)
}

pub fn derive_map_key<H: Keccak256>(mut key: Vec<u8>, slot: u64) -> H256 {
	key.extend_from_slice(&U256::from(slot).to_big_endian());
	H::keccak256(H::keccak256(&key).0.as_slice())
}

pub fn derive_map_key_with_offset<H: Keccak256>(mut key: Vec<u8>, slot: u64, offset: u64) -> H256 {
	key.extend_from_slice(&U256::from(slot).to_big_endian());
	let root_key = H::keccak256(&key).0;
	let number = U256::from_big_endian(root_key.as_slice()) + U256::from(offset);
	H::keccak256(&number.to_big_endian())
}

pub fn derive_unhashed_map_key<H: Keccak256>(mut key: Vec<u8>, slot: u64) -> H256 {
	key.extend_from_slice(&U256::from(slot).to_big_endian());
	H::keccak256(&key)
}

pub fn add_off_set_to_map_key(key: &[u8], offset: u64) -> H256 {
	let number = U256::from_big_endian(key) + U256::from(offset);
	H256(number.to_big_endian())
}

pub fn derive_array_item_key<H: Keccak256>(slot: u64, index: u64, offset: u64) -> Vec<u8> {
	let hash_result = H::keccak256(&U256::from(slot).to_big_endian());

	let array_pos = U256::from_big_endian(&hash_result.0);
	let item_pos = array_pos + U256::from(index * 2) + U256::from(offset);

	H::keccak256(&item_pos.to_big_endian()).0.to_vec()
}

pub fn get_values_from_proof<H: Keccak256 + Send + Sync>(
	keys: Vec<Vec<u8>>,
	root: H256,
	proof: Vec<Vec<u8>>,
) -> Result<Vec<Option<DBValue>>, Error> {
	let mut values = vec![];
	let proof_db = StorageProof::new(proof).into_memory_db::<KeccakHasher<H>>();
	let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher<H>>>::new(&proof_db, &root).build();
	for key in keys {
		let val = trie.get(&key).map_err(|_| Error::Custom(format!("Error reading proof db")))?;
		values.push(val);
	}

	Ok(values)
}

pub fn get_value_from_proof<H: Keccak256 + Send + Sync>(
	key: Vec<u8>,
	root: H256,
	proof: Vec<Vec<u8>>,
) -> Result<Option<DBValue>, Error> {
	let proof_db = StorageProof::new(proof).into_memory_db::<KeccakHasher<H>>();
	let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher<H>>>::new(&proof_db, &root).build();
	let val = trie
		.get(&key)
		.map_err(|e| Error::Custom(format!("Error reading proof db {:?}", e)))?;

	Ok(val)
}

// keccak256(uint256(4009) . keccak256(uint256(200_000_000) . uint256(STATE_COMMITMENT_SLOT)))
pub fn state_comitment_key(state_machine_id: U256, block_height: U256) -> (H256, H256, H256) {
	use polkadot_sdk::sp_crypto_hashing::keccak_256;

	const STATE_COMMITMENT_SLOT: u64 = 5;

	// Parent map key
	let slot = U256::from(STATE_COMMITMENT_SLOT).to_big_endian();

	let state_id = state_machine_id.to_big_endian();
	let mut key = state_id.to_vec();
	key.extend_from_slice(&slot);
	let parent_map_key = keccak_256(&key);

	// Commitment key
	let mut commitment_key = block_height.to_big_endian().to_vec();
	commitment_key.extend_from_slice(&parent_map_key);

	let slot_hash = keccak_256(&commitment_key);

	// Timestamp is at offset 0

	// overlay root is at offset 1

	let overlay_root_slot = {
		let slot = U256::from_big_endian(&slot_hash) + U256::one();
		H256::from_slice(&slot.to_big_endian())
	};

	// state root is at offset 2

	let state_root_key = {
		let slot = U256::from_big_endian(&slot_hash) + U256::one() + U256::one();
		H256::from_slice(&slot.to_big_endian())
	};

	(slot_hash.into(), overlay_root_slot, state_root_key)
}
