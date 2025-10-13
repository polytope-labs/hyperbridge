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

//! Tendermint EVM State Machine client implementation

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
	messaging::Proof,
	router::RequestResponse,
};
use pallet_ismp_host_executive::EvmHosts;
use primitive_types::H160;
use tendermint_ics23_primitives::ICS23HostFunctions;
use tendermint_primitives::keys::{DefaultEvmKeys, EvmStoreKeys, SeiEvmKeys};

use crate::{alloc::string::ToString, req_res_commitment_key, req_res_receipt_keys};
use alloc::{collections::BTreeMap, string::String, vec, vec::Vec};

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

		let proofs: Vec<crate::types::EvmKVProof> = codec::Decode::decode(&mut &proof.proof[..])
			.map_err(|e| Error::Custom(e.to_string()))?;

		let app_hash: [u8; 32] = root.state_root.0;
		let store_key_str = store_key_for(proof.height.id.state_id);
		let store_key = store_key_str.as_bytes();

		if proofs.len() != slot_keys.len() {
			return Err(Error::Custom("mismatched proofs/keys".to_string()));
		}

		for (slot, ev) in slot_keys.into_iter().zip(proofs.into_iter()) {
			// slot is expected to be 32 bytes keccak slot
			let key = storage_key_for(
				proof.height.id.state_id,
				&contract_address.0,
				slot.clone().try_into().expect("32 bytes"),
			);

			let commitment_proof = CommitmentProofBytes::try_from(ev.proof.clone())
				.map_err(|e| Error::Custom(e.to_string()))?;
			let merkle_proof = MerkleProof::try_from(&commitment_proof)
				.map_err(|e| Error::Custom(e.to_string()))?;
			let specs = ProofSpecs::cosmos();
			let root_hash = MerkleRoot { hash: app_hash.to_vec() };
			let merkle_path = MerklePath::new(vec![
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

	fn receipts_state_trie_key(&self, items: RequestResponse) -> Vec<Vec<u8>> {
		req_res_receipt_keys::<ICS23HostFunctions>(items)
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

		verify_evm_kv_proofs(keys, contract_address, root, proof)
	}
}

/// Helper function to verify Tendermint ICS23 KV proofs for EVM storage keys.
pub fn verify_evm_kv_proofs(
	mut keys: Vec<Vec<u8>>,
	default_contract_address: H160,
	root: StateCommitment,
	proof: &Proof,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
	let store_key_str = store_key_for(proof.height.id.state_id);
	let store_key = store_key_str.as_bytes();
	let app_hash: [u8; 32] = root.state_root.0;
	let proofs: Vec<crate::types::EvmKVProof> =
		codec::Decode::decode(&mut &proof.proof[..]).map_err(|e| Error::Custom(e.to_string()))?;
	// Only support 32-byte or 52-byte keys
	if keys.iter().any(|k| !(k.len() == 32 || k.len() == 52)) {
		return Err(Error::Custom("Only 32-byte or 52-byte keys are supported".to_string()));
	}

	if proofs.len() != keys.len() {
		return Err(Error::Custom("mismatched proofs/keys".to_string()));
	}

	let mut out = BTreeMap::new();

	for (key_bytes, ev) in keys.drain(..).zip(proofs.into_iter()) {
		// Determine contract address and 32-byte slot based on key length
		let (addr, slot): (H160, [u8; 32]) = if key_bytes.len() == 32 {
			(default_contract_address, key_bytes.clone().try_into().expect("32 bytes"))
		} else {
			// 52 bytes: first 20 bytes are contract address, last 32 bytes are the slot
			let addr = H160::from_slice(&key_bytes[..20]);
			let mut slot_arr = [0u8; 32];
			slot_arr.copy_from_slice(&key_bytes[20..]);
			(addr, slot_arr)
		};

		let key = storage_key_for(proof.height.id.state_id, &addr.0, slot);

		let commitment_proof = CommitmentProofBytes::try_from(ev.proof.clone())
			.map_err(|e| Error::Custom(e.to_string()))?;
		let merkle_proof =
			MerkleProof::try_from(&commitment_proof).map_err(|e| Error::Custom(e.to_string()))?;
		let specs = ProofSpecs::cosmos();
		let root_hash = MerkleRoot { hash: app_hash.to_vec() };
		let merkle_path =
			MerklePath::new(vec![PathBytes::from_bytes(store_key), PathBytes::from_bytes(&key)]);
		merkle_proof
			.verify_membership::<ICS23HostFunctions>(
				&specs,
				root_hash,
				merkle_path,
				ev.value.clone(),
				0,
			)
			.map_err(|e| Error::Custom(e.to_string()))?;

		out.insert(key_bytes, Some(ev.value));
	}

	Ok(out)
}

fn store_key_for(state_id: ismp::host::StateMachine) -> String {
	match state_id {
		ismp::host::StateMachine::Evm(_) => "evm".to_string(),
		_ => "evm".to_string(),
	}
}

fn storage_key_for(state_id: ismp::host::StateMachine, addr: &[u8; 20], slot: [u8; 32]) -> Vec<u8> {
	match state_id {
		ismp::host::StateMachine::Evm(id) => match id {
			1329 | 1328 => SeiEvmKeys::storage_key(addr, slot),
			_ => DefaultEvmKeys::storage_key(addr, slot),
		},
		_ => DefaultEvmKeys::storage_key(addr, slot),
	}
}
