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

use alloc::{collections::BTreeMap, string::ToString, vec::Vec};
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
use pharos_primitives::{spv, NonExistenceProof, PharosProofNode};
use primitive_types::{H160, H256};

/// Account proof data for a 20-byte address query.
#[derive(Encode, Decode, Clone)]
pub struct AccountProofData {
	/// Proof nodes from MSU root to the account leaf
	pub proof_nodes: Vec<PharosProofNode>,
	/// RLP-encoded account value (nonce, balance, storage_root, code_hash)
	pub raw_value: Vec<u8>,
}

/// Pharos-specific state proof (replaces EvmStateProof).
///
/// Contains Pharos hexary hash tree proof data with SHA-256 hashing.
/// Pharos uses a flat trie where storage proofs verify directly against
/// the state_root, so no separate account proof is needed for storage queries.
#[derive(Encode, Decode, Clone)]
pub struct PharosStateProof {
	/// Map of storage key (slot hash) to storage proof nodes
	pub storage_proof: BTreeMap<Vec<u8>, Vec<PharosProofNode>>,
	/// Map of storage key (slot hash) to the 32-byte padded storage value
	pub storage_values: BTreeMap<Vec<u8>, Vec<u8>>,
	/// Map of storage key (slot hash) to non-existence proof for absent keys
	pub non_existence_proofs: BTreeMap<Vec<u8>, NonExistenceProof>,
	/// Map of account address (20 bytes) to account proof data
	pub account_proofs: BTreeMap<Vec<u8>, AccountProofData>,
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
	PharosStateProof::decode(&mut &proof.proof[..]).map_err(|e| {
		Error::AnyHow(anyhow::anyhow!("Cannot decode pharos state proof: {:?}", e).into())
	})
}

/// Verify membership of ISMP commitments in the Pharos state.
pub fn verify_membership<H: Keccak256 + Send + Sync>(
	item: RequestResponse,
	root: StateCommitment,
	proof: &Proof,
	contract_address: H160,
) -> Result<(), Error> {
	let pharos_proof = decode_pharos_state_proof(proof)?;

	let state_root = H256::from_slice(&root.state_root[..]);
	let address: [u8; 20] = contract_address.0;

	let commitment_keys = req_res_commitment_key::<H, _>(item, |k| k.to_vec());

	// Pharos uses a flat trie — storage proofs verify directly against state_root.
	for slot_hash in commitment_keys {
		let storage_proof_nodes = pharos_proof
			.storage_proof
			.get(&slot_hash)
			.ok_or_else(|| Error::Custom("Missing storage proof for commitment key".to_string()))?;

		let slot_key: [u8; 32] = slot_hash.try_into().map_err(|e: Vec<u8>| {
			Error::Custom(alloc::format!("Invalid slot hash length: expected 32, got {}", e.len()))
		})?;

		spv::verify_membership_proof(
			storage_proof_nodes,
			&spv::build_storage_key(&address, &slot_key),
			&state_root.0,
		)
		.map_err(|e| Error::AnyHow(anyhow::Error::from(e).into()))?;
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

	let state_root = H256::from_slice(&root.state_root[..]);

	// Pharos uses a flat trie — storage proofs verify directly against state_root.
	let mut map = BTreeMap::new();

	for key in keys {
		let (contract_addr, slot_hash) = if key.len() == 52 {
			// First 20 bytes = contract address, last 32 = slot hash
			let addr = H160::from_slice(&key[..20]);
			(addr, key[20..].to_vec())
		} else if key.len() == 32 {
			// Direct slot hash for the ISMP host contract
			(ismp_address, key.clone())
		} else if key.len() == 20 {
			// Account query which verifies account proof and return raw account value
			let address: [u8; 20] = key.clone().try_into().map_err(|e: Vec<u8>| {
				Error::Custom(alloc::format!("Invalid address: expected 20 bytes, got {}", e.len()))
			})?;
			let account_data = pharos_proof
				.account_proofs
				.get(&key)
				.ok_or_else(|| Error::Custom("Missing account proof".to_string()))?;

			spv::verify_proof(
				&account_data.proof_nodes,
				&address,
				&account_data.raw_value,
				&state_root.0,
			)
			.map_err(|e| Error::AnyHow(anyhow::Error::from(e).into()))?;

			map.insert(key, Some(account_data.raw_value.clone()));
			continue;
		} else {
			return Err(Error::Custom(
				"Unsupported key type: expected length 20, 32, or 52".to_string(),
			));
		};

		let contract_address: [u8; 20] = contract_addr.0;

		let slot_key: [u8; 32] = slot_hash.clone().try_into().map_err(|e: Vec<u8>| {
			Error::AnyHow(
				anyhow::anyhow!("Invalid slot hash length: expected 32, got {}", e.len()).into(),
			)
		})?;

		// Check if this is a non-existence proof
		if let Some(non_existence) = pharos_proof.non_existence_proofs.get(slot_key.as_slice()) {
			spv::verify_non_existence_proof(
				&non_existence.proof_nodes,
				&spv::build_storage_key(&contract_address, &slot_key),
				&state_root.0,
				&non_existence.sibling_proofs,
			)
			.map_err(|e| Error::AnyHow(anyhow::Error::from(e).into()))?;
			map.insert(key, None);
			continue;
		}

		// Otherwise verify existence proof
		let storage_proof_nodes = pharos_proof
			.storage_proof
			.get(slot_key.as_slice())
			.ok_or_else(|| Error::Custom("Missing storage proof for key".to_string()))?;

		let storage_value = pharos_proof
			.storage_values
			.get(&slot_hash)
			.ok_or_else(|| Error::Custom("Missing storage value for key".to_string()))?;

		// Pad value to 32 bytes for proof verification
		let mut padded_value = [0u8; 32];
		if storage_value.len() <= 32 {
			padded_value[32 - storage_value.len()..].copy_from_slice(storage_value);
		} else {
			return Err(Error::Custom("Storage value exceeds 32 bytes".to_string()));
		}

		spv::verify_proof(
			storage_proof_nodes,
			&spv::build_storage_key(&contract_address, &slot_key),
			&padded_value,
			&state_root.0,
		)
		.map_err(|e| Error::AnyHow(anyhow::Error::from(e).into()))?;

		map.insert(key, Some(storage_value.clone()));
	}

	Ok(map)
}
