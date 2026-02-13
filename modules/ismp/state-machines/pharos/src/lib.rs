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

//! Pharos state machine verification.
//!
//! Uses Pharos hexary hash tree proofs with SHA-256 hashing instead of
//! Ethereum's Merkle-Patricia Trie with Keccak-256.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{collections::BTreeMap, format, string::ToString, vec, vec::Vec};
use codec::{Decode, Encode};
use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	error::Error,
	host::IsmpHost,
	messaging::{hash_request, hash_response, Keccak256, Proof},
	router::RequestResponse,
};
use pallet_ismp_host_executive::EvmHosts;
use pharos_primitives::{spv, PharosProofNode};
use primitive_types::{H160, H256, U256};

/// Slot index for requests commitments map in the ISMP contract
pub const REQUEST_COMMITMENTS_SLOT: u64 = 0;
/// Slot index for response commitments map
pub const RESPONSE_COMMITMENTS_SLOT: u64 = 1;
/// Slot index for requests receipts map
pub const REQUEST_RECEIPTS_SLOT: u64 = 2;
/// Slot index for response receipts map
pub const RESPONSE_RECEIPTS_SLOT: u64 = 3;

/// Pharos-specific state proof (replaces EvmStateProof).
///
/// Contains Pharos hexary hash tree proof data with SHA-256 hashing.
#[derive(Encode, Decode, Clone)]
pub struct PharosStateProof {
	/// Account proof nodes for the   contract
	pub contract_proof: Vec<PharosProofNode>,
	/// Map of storage key (slot hash) to storage proof nodes
	pub storage_proof: BTreeMap<Vec<u8>, Vec<PharosProofNode>>,
	/// Storage trie root from eth_getProof
	pub storage_hash: H256,
	/// RLP-encoded account value (rawValue from eth_getProof)
	pub raw_account_value: Vec<u8>,
}

/// Pharos state machine client for ISMP state proof verification.
pub struct PharosStateMachine<H: IsmpHost, T: pallet_ismp_host_executive::Config>(
	core::marker::PhantomData<(H, T)>,
);

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Default for PharosStateMachine<H, T> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Clone for PharosStateMachine<H, T> {
	fn clone(&self) -> Self {
		PharosStateMachine::<H, T>::default()
	}
}

impl<H: IsmpHost + Send + Sync, T: pallet_ismp_host_executive::Config> StateMachineClient
	for PharosStateMachine<H, T>
{
	fn verify_membership(
		&self,
		_host: &dyn IsmpHost,
		item: RequestResponse,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or_else(|| Error::Custom("Ismp contract address not found".to_string()))?;
		verify_membership::<H>(item, root, proof, contract_address)
	}

	fn receipts_state_trie_key(&self, items: RequestResponse) -> Vec<Vec<u8>> {
		req_res_receipt_keys::<H>(items)
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		let ismp_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or_else(|| Error::Custom("Ismp contract address not found".to_string()))?;
		verify_state_proof::<H>(keys, root, proof, ismp_address)
	}
}

/// Decode a PharosStateProof from the proof bytes.
fn decode_pharos_state_proof(proof: &Proof) -> Result<PharosStateProof, Error> {
	PharosStateProof::decode(&mut &proof.proof[..])
		.map_err(|_| Error::Custom(format!("Cannot decode pharos state proof")))
}

/// Verify membership of ISMP commitments in the Pharos state.
pub fn verify_membership<H: Keccak256 + Send + Sync>(
	item: RequestResponse,
	root: StateCommitment,
	proof: &Proof,
	contract_address: H160,
) -> Result<(), Error> {
	let pharos_proof = decode_pharos_state_proof(proof)?;

	// Verify the account proof against state_root
	let state_root = H256::from_slice(&root.state_root[..]);
	let address: [u8; 20] = contract_address.0;
	if !spv::verify_account_proof(
		&pharos_proof.contract_proof,
		&address,
		&pharos_proof.raw_account_value,
		&state_root.0,
	) {
		return Err(Error::Custom("Invalid contract account proof".to_string()));
	}

	let storage_hash = pharos_proof.storage_hash;

	let commitment_keys = req_res_commitment_key::<H>(item);

	// verify each commitment exists in the storage proof
	for slot_hash in commitment_keys {
		// In Pharos, the trie key is contract_address || slot_hash (52 bytes)
		let mut trie_key = Vec::with_capacity(52);
		trie_key.extend_from_slice(&address);
		trie_key.extend_from_slice(&slot_hash);

		let storage_proof_nodes = pharos_proof
			.storage_proof
			.get(&slot_hash)
			.ok_or_else(|| Error::Custom("Missing storage proof for commitment key".to_string()))?;

		let leaf = storage_proof_nodes
			.last()
			.ok_or_else(|| Error::Custom("Empty storage proof".to_string()))?;

		if leaf.proof_node.len() != 65 {
			return Err(Error::Custom("Invalid storage proof leaf node".to_string()));
		}

		// Verify the storage proof: we need to reconstruct the value from the leaf
		// The leaf contains sha256(value), so we verify the proof structure itself
		let leaf_value_hash = &leaf.proof_node[33..65];
		let leaf_key_hash = &leaf.proof_node[1..33];

		let expected_key_hash = spv::sha256(&trie_key);
		if leaf_key_hash != expected_key_hash {
			return Err(Error::Custom("Storage proof key mismatch".to_string()));
		}

		// Verify the proof chain from leaf to root
		let mut current_hash = spv::sha256(&leaf.proof_node);
		for i in (0..storage_proof_nodes.len() - 1).rev() {
			let parent = &storage_proof_nodes[i];
			let begin = parent.next_begin_offset as usize;
			let end = parent.next_end_offset as usize;

			if end > parent.proof_node.len() || begin >= end || (end - begin) != 32 {
				return Err(Error::Custom("Invalid proof node offsets".to_string()));
			}

			if parent.proof_node[begin..end] != current_hash {
				return Err(Error::Custom("Proof hash chain broken".to_string()));
			}

			current_hash = spv::sha256(&parent.proof_node);
		}

		if current_hash != storage_hash.0 {
			return Err(Error::Custom("Storage proof root mismatch".to_string()));
		}

		// If we got here, the key exists in the trie with some value (non-empty leaf)
		// For membership verification, we just need to confirm existence
		let _ = leaf_value_hash;
	}

	Ok(())
}

/// Verify state proof and return key-value map.
pub fn verify_state_proof<H: Keccak256 + Send + Sync>(
	keys: Vec<Vec<u8>>,
	root: StateCommitment,
	proof: &Proof,
	ismp_address: H160,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
	let pharos_proof = decode_pharos_state_proof(proof)?;

	// Verify the account proof against state_root
	let state_root = H256::from_slice(&root.state_root[..]);
	let address: [u8; 20] = ismp_address.0;
	if !spv::verify_account_proof(
		&pharos_proof.contract_proof,
		&address,
		&pharos_proof.raw_account_value,
		&state_root.0,
	) {
		return Err(Error::Custom("Invalid contract account proof".to_string()));
	}

	let storage_hash = pharos_proof.storage_hash;
	let mut map = BTreeMap::new();

	for key in keys {
		let (contract_addr, slot_hash) = if key.len() == 52 {
			// First 20 bytes = contract address, last 32 = slot hash
			let addr = H160::from_slice(&key[..20]);
			let slot = H::keccak256(&key[20..]).0.to_vec();
			(addr, slot)
		} else if key.len() == 32 {
			// Direct slot hash for the ISMP host contract
			let slot = H::keccak256(&key).0.to_vec();
			(ismp_address, slot)
		} else if key.len() == 20 {
			map.insert(key, None);
			continue;
		} else {
			return Err(Error::Custom(
				"Unsupported key type: expected length 20, 32, or 52".to_string(),
			));
		};

		let contract_address: [u8; 20] = contract_addr.0;

		// Look up the storage proof for this slot
		if let Some(storage_proof_nodes) = pharos_proof.storage_proof.get(&slot_hash) {
			// Build the trie key: contract_address || slot_hash
			let mut trie_key = Vec::with_capacity(52);
			trie_key.extend_from_slice(&contract_address);
			trie_key.extend_from_slice(&slot_hash);


			let leaf = storage_proof_nodes.last();
			if let Some(leaf) = leaf {
				if leaf.proof_node.len() == 65 {
					// Verify the proof chain
					let expected_key_hash = spv::sha256(&trie_key);
					if leaf.proof_node[1..33] == expected_key_hash {
						let mut current_hash = spv::sha256(&leaf.proof_node);
						let mut valid = true;

						for i in (0..storage_proof_nodes.len() - 1).rev() {
							let parent = &storage_proof_nodes[i];
							let begin = parent.next_begin_offset as usize;
							let end = parent.next_end_offset as usize;

							if end > parent.proof_node.len() ||
								begin >= end || (end - begin) != 32
							{
								valid = false;
								break;
							}

							if parent.proof_node[begin..end] != current_hash {
								valid = false;
								break;
							}

							current_hash = spv::sha256(&parent.proof_node);
						}

						if valid && current_hash == storage_hash.0 {
							// Extract the raw value from the leaf's value_hash
							// The actual raw value needs to be included in the proof
							// For now, return the value hash as indication of existence
							let value_hash = leaf.proof_node[33..65].to_vec();
							map.insert(key, Some(value_hash));
							continue;
						}
					}
				}
			}
			// Proof verification failed - key not found or invalid
			map.insert(key, None);
		} else {
			// No proof provided for this key
			map.insert(key, None);
		}
	}

	Ok(map)
}

/// Compute Solidity storage slot keys for ISMP request/response commitments.
///
/// Solidity layout: `mapping(bytes32 => FungibleAssetRegistration)` at a given slot.
/// Key derivation: `keccak256(keccak256(commitment || uint256(slot)) + offset)`
fn req_res_commitment_key<H: Keccak256>(item: RequestResponse) -> Vec<Vec<u8>> {
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

/// Compute Solidity storage slot keys for ISMP request/response receipts.
fn req_res_receipt_keys<H: Keccak256>(item: RequestResponse) -> Vec<Vec<u8>> {
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

/// Derive a Solidity mapping key with offset:
/// `keccak256(uint256(keccak256(key || uint256(slot))) + offset)`
fn derive_map_key_with_offset<H: Keccak256>(mut key: Vec<u8>, slot: u64, offset: u64) -> H256 {
	key.extend_from_slice(&U256::from(slot).to_big_endian());
	let root_key = H::keccak256(&key).0;
	let number = U256::from_big_endian(root_key.as_slice()) + U256::from(offset);
	H::keccak256(&number.to_big_endian())
}

/// Derive an unhashed Solidity mapping key: `keccak256(key || uint256(slot))`
fn derive_unhashed_map_key<H: Keccak256>(mut key: Vec<u8>, slot: u64) -> H256 {
	key.extend_from_slice(&U256::from(slot).to_big_endian());
	H::keccak256(&key)
}
