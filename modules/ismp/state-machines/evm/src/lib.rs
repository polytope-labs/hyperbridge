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

//! Constants and methods used for evm verification

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

use alloc::collections::BTreeMap;
use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	error::Error,
	host::IsmpHost,
	messaging::{Keccak256, Proof},
	router::RequestResponse,
};
use primitive_types::{H160, H256};

pub mod prelude {
	pub use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec, vec::Vec};
}

use pallet_ismp_host_executive::EvmHosts;
use prelude::*;

pub mod presets;
pub mod tendermint;
pub mod types;
pub mod utils;
pub use tendermint::TendermintEvmStateMachine;
pub use utils::*;

pub fn verify_membership<H: Keccak256 + Send + Sync>(
	item: RequestResponse,
	root: StateCommitment,
	proof: &Proof,
	contract_address: H160,
) -> Result<(), Error> {
	let mut evm_state_proof = decode_evm_state_proof(proof)?;
	let storage_proof = evm_state_proof
		.storage_proof
		.remove(&contract_address.0.to_vec())
		.ok_or_else(|| Error::Custom("Ismp contract account trie proof is missing".to_string()))?;
	let keys = req_res_commitment_key::<H>(item);
	let root = H256::from_slice(&root.state_root[..]);
	let contract_root = get_contract_account::<H>(
		evm_state_proof.contract_proof,
		&contract_address.0,
		root.clone(),
	)?
	.storage_root
	.0
	.into();
	let values = get_values_from_proof::<H>(keys, contract_root, storage_proof)?;

	if values.into_iter().any(|val| val.is_none()) {
		Err(Error::Custom("Missing values for some keys in the proof".to_string()))?
	}

	Ok(())
}

pub fn verify_state_proof<H: Keccak256 + Send + Sync>(
	keys: Vec<Vec<u8>>,
	root: StateCommitment,
	proof: &Proof,
	ismp_address: H160,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
	let evm_state_proof = decode_evm_state_proof(proof)?;
	let mut map = BTreeMap::new();
	let mut contract_to_keys = BTreeMap::new();
	let mut contract_account_queries = Vec::new();
	// Group keys by the contract address they belong to
	for key in keys {
		// For keys that are 52 bytes we expect the first 20 bytes to be the contract address and
		// the last 32 bytes the slot hash.
		// For keys that are 20 bytes we expect that to the
		// contract or account address.
		// For keys that are 32 bytes we expect that to be a slothash in
		// the Ismp EVM host
		let contract_address = if key.len() == 52 {
			H160::from_slice(&key[..20])
		} else if key.len() == 32 {
			ismp_address
		} else if key.len() == 20 {
			contract_account_queries.push(H160::from_slice(&key));
			continue;
		} else {
			Err(Error::Custom(
				"Unsupported Key type, found a key whose length is not one of 20, 32 or 52"
					.to_string(),
			))?
		};
		let entry = contract_to_keys.entry(contract_address.0.to_vec()).or_insert(vec![]);

		let slot_hash = if key.len() == 52 {
			H::keccak256(&key[20..]).0.to_vec()
		} else {
			H::keccak256(&key).0.to_vec()
		};

		entry.push((key, slot_hash));
	}

	// Ensure there is a proof for all contract addresses
	let result = contract_to_keys
		.clone()
		.into_keys()
		.all(|contract| evm_state_proof.storage_proof.contains_key(&contract));
	if !result {
		Err(Error::Custom(
			"The storage proof is incomplete, missing some contract proofs".to_string(),
		))?
	}

	for (contract_address, storage_proof) in evm_state_proof.storage_proof {
		let contract_root = get_contract_account::<H>(
			evm_state_proof.contract_proof.clone(),
			&contract_address,
			root.state_root,
		)?
		.storage_root
		.0
		.into();

		if let Some(keys) = contract_to_keys.remove(&contract_address) {
			let slot_hashes = keys.iter().map(|(_, slot_hash)| slot_hash.clone()).collect();
			let values = get_values_from_proof::<H>(slot_hashes, contract_root, storage_proof)?;
			keys.into_iter().zip(values).for_each(|((key, _), value)| {
				map.insert(key, value);
			});
		}
	}

	for contract_address in contract_account_queries {
		let account = get_contract_account::<H>(
			evm_state_proof.contract_proof.clone(),
			&contract_address[..],
			root.state_root,
		)?;

		// Using rlp encoding for uniformity, storage values from state proofs are rlp encoded
		let encoded = alloy_rlp::encode(account);

		map.insert(contract_address.0.to_vec(), Some(encoded));
	}

	Ok(map)
}

pub struct EvmStateMachine<H: IsmpHost, T: pallet_ismp_host_executive::Config>(
	core::marker::PhantomData<(H, T)>,
);

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Default for EvmStateMachine<H, T> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Clone for EvmStateMachine<H, T> {
	fn clone(&self) -> Self {
		EvmStateMachine::<H, T>::default()
	}
}

impl<H: IsmpHost + Send + Sync, T: pallet_ismp_host_executive::Config> StateMachineClient
	for EvmStateMachine<H, T>
{
	fn verify_membership(
		&self,
		host: &dyn IsmpHost,
		item: RequestResponse,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or_else(|| Error::Custom("Ismp contract address not found".to_string()))?;
		verify_membership::<H>(item, root, proof, contract_address)
	}

	fn receipts_state_trie_key(&self, items: RequestResponse) -> Vec<Vec<u8>> {
		// State trie keys are used to process timeouts from EVM chains
		// We return the trie keys for request or response receipts
		req_res_receipt_keys::<H>(items)
	}

	fn verify_state_proof(
		&self,
		host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		let ismp_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or_else(|| Error::Custom("Ismp contract address not found".to_string()))?;
		verify_state_proof::<H>(keys, root, proof, ismp_address)
	}
}
