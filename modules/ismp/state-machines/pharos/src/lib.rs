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

use alloc::{collections::BTreeMap, format, string::ToString, vec::Vec};
use codec::{Decode, Encode};
use evm_state_machine::{req_res_commitment_key, req_res_receipt_keys};
use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	error::Error,
	host::IsmpHost,
	messaging::{Keccak256, Proof},
	router::RequestResponse,
};
use pallet_ismp_host_executive::EvmHosts;
use pharos_primitives::{spv, PharosProofNode};
use primitive_types::{H160, H256};

/// Pharos-specific state proof (replaces EvmStateProof).
///
/// Contains Pharos hexary hash tree proof data with SHA-256 hashing.
#[derive(Encode, Decode, Clone)]
pub struct PharosStateProof {
	/// Account proof nodes for the contract
	pub contract_proof: Vec<PharosProofNode>,
	/// Map of storage key (slot hash) to storage proof nodes
	pub storage_proof: BTreeMap<Vec<u8>, Vec<PharosProofNode>>,
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

	// Extract the storage root from the verified account value
	// The account value is RLP([nonce, balance, storage_root,
	// code_hash]). In Pharos's flat trie, the per-account storage_root is empty,
	// so we fall back to state_root since storage proofs verify against the state trie.
	let decoded_root =
		spv::decode_storage_root(&pharos_proof.raw_account_value).ok_or_else(|| {
			Error::Custom("Failed to decode storage root from account value".to_string())
		})?;
	let storage_hash =
		if decoded_root == [0u8; 32] { state_root } else { H256::from(decoded_root) };

	let commitment_keys = req_res_commitment_key::<H, _>(item, |k| H::keccak256(k).0.to_vec());

	// Verify each commitment exists in the storage proof
	for slot_hash in commitment_keys {
		let storage_proof_nodes = pharos_proof
			.storage_proof
			.get(&slot_hash)
			.ok_or_else(|| Error::Custom("Missing storage proof for commitment key".to_string()))?;

		let slot_key: [u8; 32] = slot_hash
			.try_into()
			.map_err(|_| Error::Custom("Invalid slot hash length".to_string()))?;

		spv::verify_storage_membership_proof(
			storage_proof_nodes,
			&address,
			&slot_key,
			&storage_hash.0,
		)
		.ok_or_else(|| Error::Custom("Storage membership proof verification failed".to_string()))?;
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

	// Extract the storage root from the verified account value.
	// In Pharos's flat trie, the per-account storage_root is
	// empty, so we fall back to state_root since storage proofs verify against the
	// state trie.
	let decoded_root =
		spv::decode_storage_root(&pharos_proof.raw_account_value).ok_or_else(|| {
			Error::Custom("Failed to decode storage root from account value".to_string())
		})?;
	let storage_hash =
		if decoded_root == [0u8; 32] { state_root } else { H256::from(decoded_root) };
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

		// Look up the storage proof for this slot and verify it
		if let Some(storage_proof_nodes) = pharos_proof.storage_proof.get(&slot_hash) {
			if let Ok(slot_key) = <[u8; 32]>::try_from(slot_hash.as_slice()) {
				if let Some(value_hash) = spv::verify_storage_membership_proof(
					storage_proof_nodes,
					&contract_address,
					&slot_key,
					&storage_hash.0,
				) {
					map.insert(key, Some(value_hash.to_vec()));
					continue;
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
