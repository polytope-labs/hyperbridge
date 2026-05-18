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

use ibc_core_commitment_types::{
	commitment::CommitmentProofBytes,
	merkle::{MerklePath, MerkleProof},
	proto::v1::MerkleRoot,
	specs::ProofSpecs,
};
use ibc_core_host::types::path::PathBytes;
use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	error::Error,
	host::IsmpHost,
	messaging::{Keccak256, Proof},
};
use pallet_ismp_host_executive::EvmHosts;
use primitive_types::{H160, H256};
use tendermint_ics23_primitives::ICS23HostFunctions;
use tendermint_primitives::keys::{DefaultEvmKeys, EvmStoreKeys, SeiEvmKeys};

use crate::{req_commitment_key, req_receipt_keys};
use alloc::{
	collections::BTreeMap,
	string::{String, ToString},
	vec,
	vec::Vec,
};
use thiserror::Error as ThisError;

/// Errors produced by the Tendermint EVM state machine client.
#[derive(Debug, ThisError)]
pub enum TendermintEvmError {
	/// No ISMP host contract is registered for the requested state machine id.
	#[error("Ismp contract address not found")]
	IsmpContractNotFound,
	/// The number of supplied ICS23 proofs doesn't match the number of queried keys.
	#[error("mismatched proofs/keys")]
	MismatchedProofsAndKeys,
	/// A query key length didn't match any supported layout (32 or 52 bytes).
	#[error("Only 32-byte or 52-byte keys are supported")]
	UnsupportedKeyLength,
	/// A non-membership proof contained at least one delivered request.
	#[error("Some Requests in the batch have been delivered")]
	DeliveredRequestsInBatch,
	/// SCALE decoding the EVM KV proof bundle failed.
	#[error("Failed to decode proof bundle: {0}")]
	ProofDecodeError(String),
	/// The ICS23 commitment bytes were malformed.
	#[error("Invalid commitment proof bytes: {0}")]
	InvalidCommitmentProof(String),
	/// The ICS23 merkle proof failed to verify.
	#[error("Merkle proof verification failed: {0}")]
	MerkleProofVerificationFailed(String),
}

impl From<TendermintEvmError> for Error {
	fn from(e: TendermintEvmError) -> Error {
		Error::AnyHow(anyhow::Error::new(e).into())
	}
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
		commitments: Vec<H256>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or(TendermintEvmError::IsmpContractNotFound)?;

		let slot_keys = self.commitment_state_trie_key(commitments);

		let proofs: Vec<crate::types::EvmKVProof> = codec::Decode::decode(&mut &proof.proof[..])
			.map_err(|e| TendermintEvmError::ProofDecodeError(e.to_string()))?;

		let app_hash: [u8; 32] = root.state_root.0;
		let store_key_str = store_key_for(proof.height.id.state_id);
		let store_key = store_key_str.as_bytes();

		if proofs.len() != slot_keys.len() {
			return Err(TendermintEvmError::MismatchedProofsAndKeys.into());
		}

		for (slot, ev) in slot_keys.into_iter().zip(proofs.into_iter()) {
			// slot is expected to be 32 bytes keccak slot
			let key = storage_key_for(
				proof.height.id.state_id,
				&contract_address.0,
				slot.clone().try_into().expect("32 bytes"),
			);

			let commitment_proof = CommitmentProofBytes::try_from(ev.proof.clone())
				.map_err(|e| TendermintEvmError::InvalidCommitmentProof(e.to_string()))?;
			let merkle_proof = MerkleProof::try_from(&commitment_proof)
				.map_err(|e| TendermintEvmError::InvalidCommitmentProof(e.to_string()))?;
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
				.map_err(|e| TendermintEvmError::MerkleProofVerificationFailed(e.to_string()))?;
		}
		Ok(())
	}

	fn commitment_state_trie_key(&self, commitments: Vec<H256>) -> Vec<Vec<u8>> {
		req_commitment_key::<ICS23HostFunctions, _>(commitments, |k| {
			ICS23HostFunctions::keccak256(k).0.to_vec()
		})
	}

	fn receipts_state_trie_key(&self, commitments: Vec<H256>) -> Vec<Vec<u8>> {
		req_receipt_keys::<ICS23HostFunctions>(commitments)
	}

	fn verify_non_membership(
		&self,
		host: &dyn IsmpHost,
		commitments: Vec<H256>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let keys = self.receipts_state_trie_key(commitments);
		let values = self.verify_state_proof(host, keys, root, proof)?;
		if values.into_iter().any(|(_key, val)| val.is_some()) {
			return Err(TendermintEvmError::DeliveredRequestsInBatch.into());
		}
		Ok(())
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or(TendermintEvmError::IsmpContractNotFound)?;

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
	let proofs: Vec<crate::types::EvmKVProof> = codec::Decode::decode(&mut &proof.proof[..])
		.map_err(|e| TendermintEvmError::ProofDecodeError(e.to_string()))?;
	// Only support 32-byte or 52-byte keys
	if keys.iter().any(|k| !(k.len() == 32 || k.len() == 52)) {
		return Err(TendermintEvmError::UnsupportedKeyLength.into());
	}

	if proofs.len() != keys.len() {
		return Err(TendermintEvmError::MismatchedProofsAndKeys.into());
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
			.map_err(|e| TendermintEvmError::InvalidCommitmentProof(e.to_string()))?;
		let merkle_proof = MerkleProof::try_from(&commitment_proof)
			.map_err(|e| TendermintEvmError::InvalidCommitmentProof(e.to_string()))?;
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
			.map_err(|e| TendermintEvmError::MerkleProofVerificationFailed(e.to_string()))?;

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
