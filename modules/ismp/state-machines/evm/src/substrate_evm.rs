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

//! Substrate EVM State Machine client implementation

use crate::{prelude::*, req_res_commitment_key, req_res_receipt_keys};
use alloc::collections::BTreeMap;
use codec::{Decode, Encode};
use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	error::Error,
	host::IsmpHost,
	messaging::Proof,
	router::RequestResponse,
};
use pallet_ismp_host_executive::EvmHosts;
use polkadot_sdk::*;
use primitive_types::H160;
use sp_core::{hashing, storage::ChildInfo, H256};
use sp_trie::{LayoutV0, StorageProof, Trie, TrieDBBuilder};

/// Proof structure for Substrate EVM verification
#[derive(Decode, Encode)]
pub struct SubstrateEvmProof {
	/// Proof of the Contract AccountInfo and the Child Trie Root in the Main State Trie
	pub main_proof: Vec<Vec<u8>>,
	/// Proof of the Storage Slots in the Contract's Child Trie
	pub child_proof: Vec<Vec<u8>>,
}

/// Substrate EVM State machine client
pub struct SubstrateEvmStateMachine<H: IsmpHost, T: pallet_ismp_host_executive::Config>(
	core::marker::PhantomData<(H, T)>,
);

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Default
	for SubstrateEvmStateMachine<H, T>
{
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Clone for SubstrateEvmStateMachine<H, T> {
	fn clone(&self) -> Self {
		Self::default()
	}
}

impl<H: IsmpHost + Send + Sync, T: pallet_ismp_host_executive::Config> StateMachineClient
	for SubstrateEvmStateMachine<H, T>
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

		let proof: SubstrateEvmProof = Decode::decode(&mut &proof.proof[..])
			.map_err(|e| Error::Custom(format!("Failed to decode proof: {:?}", e)))?;

		let state_root = H256::from_slice(&root.state_root[..]);

		// verify contact info in main trie first to get Trie Id
		let contract_info_key = contract_info_key(contract_address);
		let trie_id =
			fetch_trie_id_from_main_proof::<H>(&proof.main_proof, state_root, &contract_info_key)?;

		let child_root =
			fetch_child_root_from_main_proof::<H>(&proof.main_proof, state_root, &trie_id)?;

		// verify storage slots in child trie
		let keys = req_res_commitment_key::<H>(item);

		// convert evm keys to child trie keys
		let storage_keys: Vec<Vec<u8>> =
			keys.into_iter().map(|k| hashing::blake2_256(&k).to_vec()).collect();

		verify_child_trie_membership::<H>(child_root, &proof.child_proof, storage_keys)
	}

	fn receipts_state_trie_key(&self, request: RequestResponse) -> Vec<Vec<u8>> {
		req_res_receipt_keys::<H>(request)
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or_else(|| Error::Custom("Ismp contract address not found".to_string()))?;

		let proof: SubstrateEvmProof = Decode::decode(&mut &proof.proof[..])
			.map_err(|e| Error::Custom(format!("Failed to decode proof: {:?}", e)))?;

		let state_root = H256::from_slice(&root.state_root[..]);

		let contract_info_key = contract_info_key(contract_address);
		let trie_id =
			fetch_trie_id_from_main_proof::<H>(&proof.main_proof, state_root, &contract_info_key)?;

		let child_root =
			fetch_child_root_from_main_proof::<H>(&proof.main_proof, state_root, &trie_id)?;

		let storage_keys: Vec<Vec<u8>> =
			keys.iter().map(|k| hashing::blake2_256(k).to_vec()).collect();

		let values = verify_child_trie_values::<H>(child_root, &proof.child_proof, storage_keys)?;

		let mut map = BTreeMap::new();
		for (key, value) in keys.into_iter().zip(values.into_iter()) {
			map.insert(key, value);
		}

		Ok(map)
	}
}

fn contract_info_key(address: H160) -> Vec<u8> {
	let mut key = Vec::new();
	key.extend_from_slice(&hashing::twox_128(b"Revive"));
	key.extend_from_slice(&hashing::twox_128(b"AccountInfoOf"));
	key.extend_from_slice(address.as_bytes());
	key
}

fn fetch_trie_id_from_main_proof<H: IsmpHost>(
	proof: &[Vec<u8>],
	root: H256,
	key: &[u8],
) -> Result<Vec<u8>, Error> {
	let db = StorageProof::new(proof.to_vec()).into_memory_db::<sp_core::Blake2Hasher>();
	let trie = TrieDBBuilder::<LayoutV0<sp_core::Blake2Hasher>>::new(&db, &root).build();

	let val = trie
		.get(key)
		.map_err(|e| Error::Custom(format!("Trie error: {:?}", e)))?
		.ok_or_else(|| Error::Custom("Contract Info not found in main trie".to_string()))?;

	// Decodes AccountInfo to get trie_id
	// AccountInfo { account_type: enum { Contract(ContractInfo { trie_id, ... }) = 0, ... }, ... }
	let mut input = &val[..];
	let variant_index = u8::decode(&mut input)
		.map_err(|_| Error::Custom("Failed to decode AccountInfo variant".to_string()))?;

	if variant_index != 0 {
		return Err(Error::Custom("Account is not a contract".to_string()));
	}

	let trie_id = Vec::<u8>::decode(&mut input)
		.map_err(|_| Error::Custom("Failed to decode trie_id".to_string()))?;

	Ok(trie_id)
}

fn fetch_child_root_from_main_proof<H: IsmpHost>(
	proof: &[Vec<u8>],
	root: H256,
	trie_id: &[u8],
) -> Result<H256, Error> {
	let child_info = ChildInfo::new_default(trie_id);
	let key = child_info.prefixed_storage_key();

	let db = StorageProof::new(proof.to_vec()).into_memory_db::<sp_core::Blake2Hasher>();
	let trie = TrieDBBuilder::<LayoutV0<sp_core::Blake2Hasher>>::new(&db, &root).build();

	let val = trie
		.get(&key)
		.map_err(|e| Error::Custom(format!("Trie error: {:?}", e)))?
		.ok_or_else(|| Error::Custom("Child Trie Root not found in main trie".to_string()))?;

	let child_root = H256::decode(&mut &val[..])
		.map_err(|_| Error::Custom("Failed to decode child root".to_string()))?;

	Ok(child_root)
}

fn verify_child_trie_membership<H: IsmpHost>(
	root: H256,
	proof: &[Vec<u8>],
	keys: Vec<Vec<u8>>,
) -> Result<(), Error> {
	let db = StorageProof::new(proof.to_vec()).into_memory_db::<sp_core::Blake2Hasher>();
	let trie = TrieDBBuilder::<LayoutV0<sp_core::Blake2Hasher>>::new(&db, &root).build();

	for key in keys {
		let val = trie
			.get(&key)
			.map_err(|e| Error::Custom(format!("Child Trie error: {:?}", e)))?;
		if val.is_none() {
			return Err(Error::Custom(format!("Key {:?} not found in child trie", key)));
		}
	}
	Ok(())
}

fn verify_child_trie_values<H: IsmpHost>(
	root: H256,
	proof: &[Vec<u8>],
	keys: Vec<Vec<u8>>,
) -> Result<Vec<Option<Vec<u8>>>, Error> {
	let db = StorageProof::new(proof.to_vec()).into_memory_db::<sp_core::Blake2Hasher>();
	let trie = TrieDBBuilder::<LayoutV0<sp_core::Blake2Hasher>>::new(&db, &root).build();

	let mut values = Vec::new();
	for key in keys {
		let val = trie
			.get(&key)
			.map_err(|e| Error::Custom(format!("Child Trie error: {:?}", e)))?;
		values.push(val);
	}
	Ok(values)
}
