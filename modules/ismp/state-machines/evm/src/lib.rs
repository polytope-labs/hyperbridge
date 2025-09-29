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
use alloc::string::String;
use ibc::core::{
	commitment_types::{
		commitment::CommitmentProofBytes,
		merkle::{MerklePath, MerkleProof},
		proto::v1::MerkleRoot,
		specs::ProofSpecs,
	},
	host::types::path::PathBytes,
};
use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	error::Error,
	host::IsmpHost,
	messaging::{Keccak256, Proof},
	router::RequestResponse,
};
use primitive_types::{H160, H256};
use tendermint_ics23_primitives::ICS23HostFunctions;
use tendermint_primitives::keys::{EvmStoreKeys, SeiEvmKeys};

pub mod prelude {
	pub use alloc::collections::BTreeMap;
	pub use alloc::{boxed::Box, string::ToString, vec, vec::Vec};
}

use pallet_ismp_host_executive::EvmHosts;
use prelude::*;

pub mod presets;
pub mod types;
pub mod utils;
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

/// Tendermint EVM State Machine client verifying ICS23 KV proofs against app hash
pub struct TendermintEvmStateMachine<H: IsmpHost, T: pallet_ismp_host_executive::Config>(
	core::marker::PhantomData<(H, T)>,
);

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Default
	for TendermintEvmStateMachine<H, T>
{
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Clone for TendermintEvmStateMachine<H, T> {
	fn clone(&self) -> Self {
		Self::default()
	}
}

impl<H: IsmpHost + Send + Sync, T: pallet_ismp_host_executive::Config> StateMachineClient
	for TendermintEvmStateMachine<H, T>
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

		let slot_keys = req_res_commitment_key::<ICS23HostFunctions>(item);

		let proofs: prelude::Vec<crate::types::EvmKVProof> =
			codec::Decode::decode(&mut &proof.proof[..])
				.map_err(|e| Error::Custom(e.to_string()))?;

		// Compose ABCI keys using chain-specific layout and verify against Tendermint app hash
		let app_hash: [u8; 32] = root.state_root.0;
		let (store_key_str, key_provider) = select_keys_by_chain(proof.height.id.state_id);
		let store_key = store_key_str.as_bytes();

		if proofs.len() != slot_keys.len() {
			return Err(Error::Custom("mismatched proofs/keys".to_string()));
		}

		for (slot, ev) in slot_keys.into_iter().zip(proofs.into_iter()) {
			// slot is expected to be 32 bytes keccak slot
			let key = key_provider
				.storage_key(&contract_address.0, slot.clone().try_into().expect("32 bytes"));

			// Verify ICS23 membership using provided value and commitment proof bytes
			let commitment_proof = CommitmentProofBytes::try_from(ev.proof.clone())
				.map_err(|e| Error::Custom(e.to_string()))?;
			let merkle_proof = MerkleProof::try_from(&commitment_proof)
				.map_err(|e| Error::Custom(e.to_string()))?;
			let specs = ProofSpecs::cosmos();
			let root_hash = MerkleRoot { hash: app_hash.to_vec() };
			let merkle_path = MerklePath::new(prelude::vec![
				PathBytes::from_bytes(store_key),
				PathBytes::from_bytes(&key),
			]);
			merkle_proof
				.verify_membership::<ICS23HostFunctions>(
					&specs,
					root_hash,
					merkle_path,
					ev.value,
					0,
				)
				.map_err(|e| Error::Custom(e.to_string()))?;
		}
		Ok(())
	}

	fn receipts_state_trie_key(&self, items: RequestResponse) -> prelude::Vec<prelude::Vec<u8>> {
		req_res_receipt_keys::<ICS23HostFunctions>(items)
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		keys: prelude::Vec<prelude::Vec<u8>>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<prelude::BTreeMap<prelude::Vec<u8>, Option<prelude::Vec<u8>>>, Error> {
		let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or_else(|| Error::Custom("Ismp contract address not found".to_string()))?;

		// Only support 32-byte slot keys
		if keys.iter().any(|k| k.len() != 32) {
			return Err(Error::Custom("Only 32-byte keys supported".to_string()));
		}

		let proofs: prelude::Vec<crate::types::EvmKVProof> =
			codec::Decode::decode(&mut &proof.proof[..])
				.map_err(|e| Error::Custom(e.to_string()))?;
		if proofs.len() != keys.len() {
			return Err(Error::Custom("mismatched proofs/keys".to_string()));
		}

		let app_hash: [u8; 32] = root.state_root.0;
		let (store_key_str, key_provider) = select_keys_by_chain(proof.height.id.state_id);
		let store_key = store_key_str.as_bytes();
		let mut out = prelude::BTreeMap::new();

		for (slot, ev) in keys.into_iter().zip(proofs.into_iter()) {
			let key = key_provider
				.storage_key(&contract_address.0, slot.clone().try_into().expect("32 bytes"));

			let commitment_proof = CommitmentProofBytes::try_from(ev.proof.clone())
				.map_err(|e| Error::Custom(e.to_string()))?;
			let merkle_proof = MerkleProof::try_from(&commitment_proof)
				.map_err(|e| Error::Custom(e.to_string()))?;
			let specs = ProofSpecs::cosmos();
			let root_hash = MerkleRoot { hash: app_hash.to_vec() };
			let merkle_path = MerklePath::new(prelude::vec![
				PathBytes::from_bytes(store_key),
				PathBytes::from_bytes(&key),
			]);
			merkle_proof
				.verify_membership::<ICS23HostFunctions>(
					&specs,
					root_hash,
					merkle_path,
					ev.value.clone(),
					0,
				)
				.map_err(|e| Error::Custom(e.to_string()))?;

			out.insert(slot, Some(ev.value));
		}

		Ok(out)
	}
}

fn select_keys_by_chain(
	state_id: ismp::host::StateMachine,
) -> (String, Box<dyn EvmStoreKeys + Send + Sync>) {
	match state_id {
		// Map per-chain once we have all the chain keys; default to Sei-style EVM layout and store key "evm"
		ismp::host::StateMachine::Tendermint(_id) => ("evm".to_string(), Box::new(SeiEvmKeys)),
		_ => ("evm".to_string(), Box::new(SeiEvmKeys)),
	}
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
