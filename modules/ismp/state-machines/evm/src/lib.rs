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

use alloc::{
	collections::{BTreeMap, BTreeSet},
	string::{String, ToString},
};
use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	error::Error,
	host::IsmpHost,
	messaging::{Keccak256, Proof},
};
use primitive_types::{H160, H256};
use thiserror::Error as ThisError;

/// Errors produced by the EVM state machine client and its proof helpers.
#[derive(Debug, ThisError)]
pub enum EvmStateMachineError {
	/// No ISMP host contract is registered for the requested state machine id.
	#[error("Ismp contract address not found")]
	IsmpContractNotFound,
	/// The encoded state proof has no entry for the requested contract.
	#[error("Ismp contract account trie proof is missing")]
	ContractAccountProofMissing,
	/// At least one membership key has no value in the supplied proof.
	#[error("Missing values for some keys in the proof")]
	MissingMembershipValues,
	/// A non-membership proof contained at least one delivered request.
	#[error("Some Requests in the batch have been delivered")]
	DeliveredRequestsInBatch,
	/// The number of verified values doesn't match the number of supplied keys.
	#[error("Mismatched values/keys: the proof did not account for every key")]
	MismatchedValuesAndKeys,
	/// A query key length didn't match any supported layout (20, 32, or 52 bytes).
	#[error("Unsupported Key type, found a key whose length is not one of 20, 32 or 52")]
	UnsupportedKeyLength,
	/// The same key was supplied more than once.
	///
	/// Repeats resolve to the same value and collapse in the returned map, so the caller's
	/// key-count check already rejects them — but only after each repeat has been verified.
	/// Rejecting here keeps that cost off an input that was never going to be accepted.
	#[error("Duplicate key in the query")]
	DuplicateKey,
	/// The proof is missing the storage trie for at least one referenced contract.
	#[error("The storage proof is incomplete, missing some contract proofs")]
	IncompleteStorageProof,
	/// The proof carries a storage trie for a contract no key was requested from.
	///
	/// Paired with [`Self::IncompleteStorageProof`] this makes the correspondence exact: the
	/// storage proof must cover the requested contracts and nothing else. Resolving a contract
	/// means cloning the whole contract proof and rebuilding its trie, so accepting unrequested
	/// entries would let the cost of a proof grow with padding rather than with the work asked
	/// for.
	#[error("The storage proof contains a contract that was not requested")]
	UnrequestedContractProof,
	/// Failed to SCALE-decode the EVM state proof from the proof bytes.
	#[error("Cannot decode evm state proof")]
	StateProofDecodeError,
	/// A 32-byte input had the wrong length.
	#[error("Input vector must have exactly 32 elements, got {0}")]
	BadByteLength(usize),
	/// The contract account proof was malformed or invalid.
	#[error("Invalid contract account proof")]
	InvalidContractAccountProof,
	/// The proof is missing the contract account leaf.
	#[error("Contract account is not present in proof")]
	ContractAccountNotPresent,
	/// The contract account leaf failed to RLP-decode.
	#[error("Error decoding contract account from value")]
	ContractAccountDecodeError,
	/// The trie backend returned an error while reading a key.
	#[error("Error reading proof db: {0}")]
	TrieReadError(String),
}

impl From<EvmStateMachineError> for Error {
	fn from(e: EvmStateMachineError) -> Error {
		Error::AnyHow(anyhow::Error::new(e).into())
	}
}

pub mod prelude {
	pub use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec, vec::Vec};
}

use pallet_ismp_host_executive::EvmHosts;
use prelude::*;

pub mod presets;
pub mod substrate_evm;
pub mod tendermint;
pub mod types;
pub mod utils;

pub use substrate_evm::SubstrateEvmStateMachine;
pub use tendermint::TendermintEvmStateMachine;
pub use utils::*;

pub fn verify_membership<H: Keccak256 + Send + Sync>(
	commitments: Vec<H256>,
	root: StateCommitment,
	proof: &Proof,
	contract_address: H160,
) -> Result<(), Error> {
	let mut evm_state_proof = decode_evm_state_proof(proof)?;
	let storage_proof = evm_state_proof
		.storage_proof
		.remove(&contract_address.0.to_vec())
		.ok_or(EvmStateMachineError::ContractAccountProofMissing)?;
	let keys = req_commitment_key::<H, _>(commitments, |k| H::keccak256(k).0.to_vec());
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
		return Err(EvmStateMachineError::MissingMembershipValues.into());
	}

	Ok(())
}

pub fn verify_state_proof<H: Keccak256 + Send + Sync>(
	keys: Vec<Vec<u8>>,
	root: H256,
	proof: &Proof,
	ismp_address: H160,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
	// Reject repeats before doing any work at all. They are already an error — repeats collapse
	// in the map returned below, so the caller's key-count check fails — but reaching that
	// check means every repeat has been verified first, and for account queries that is a
	// clone of the whole contract proof each time.
	let mut seen_keys = BTreeSet::new();
	for key in &keys {
		if !seen_keys.insert(key.as_slice()) {
			return Err(EvmStateMachineError::DuplicateKey.into());
		}
	}

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
			return Err(EvmStateMachineError::UnsupportedKeyLength.into());
		};
		let entry = contract_to_keys.entry(contract_address.0.to_vec()).or_insert(vec![]);

		let slot_hash = if key.len() == 52 {
			H::keccak256(&key[20..]).0.to_vec()
		} else {
			H::keccak256(&key).0.to_vec()
		};

		entry.push((key, slot_hash));
	}

	// The storage proof must correspond exactly to the contracts a key was requested from:
	// every one covered, and nothing else. Both directions are settled here, before a single
	// entry is resolved — resolving one clones the whole contract proof and rebuilds its trie,
	// so checking as we go would let a proof padded with unrequested accounts do that work
	// before being rejected, and would make which error surfaces depend on map ordering.
	// The honest prover already emits exactly this set, deriving its map from the requested
	// keys, so nothing legitimate is turned away.
	let result = contract_to_keys
		.clone()
		.into_keys()
		.all(|contract| evm_state_proof.storage_proof.contains_key(&contract));
	if !result {
		return Err(EvmStateMachineError::IncompleteStorageProof.into());
	}

	if evm_state_proof
		.storage_proof
		.keys()
		.any(|contract| !contract_to_keys.contains_key(contract))
	{
		return Err(EvmStateMachineError::UnrequestedContractProof.into());
	}

	for (contract_address, storage_proof) in evm_state_proof.storage_proof {
		// Unreachable: the correspondence check above already rejected any entry without a
		// requested key. Kept as a hard error rather than a skip so the invariant survives if
		// that check is ever moved or relaxed.
		let Some(keys) = contract_to_keys.remove(&contract_address) else {
			return Err(EvmStateMachineError::UnrequestedContractProof.into());
		};

		let contract_root = get_contract_account::<H>(
			evm_state_proof.contract_proof.clone(),
			&contract_address,
			root,
		)?
		.storage_root
		.0
		.into();

		let slot_hashes = keys.iter().map(|(_, slot_hash)| slot_hash.clone()).collect();
		let values = get_values_from_proof::<H>(slot_hashes, contract_root, storage_proof)?;
		keys.into_iter().zip(values).for_each(|((key, _), value)| {
			map.insert(key, value);
		});
	}

	for contract_address in contract_account_queries {
		let account = get_contract_account::<H>(
			evm_state_proof.contract_proof.clone(),
			&contract_address[..],
			root,
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
		commitments: Vec<H256>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let contract_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or(EvmStateMachineError::IsmpContractNotFound)?;
		verify_membership::<H>(commitments, root, proof, contract_address)
	}

	fn commitment_state_trie_key(&self, commitments: Vec<H256>) -> Vec<Vec<u8>> {
		req_commitment_key::<H, _>(commitments, |k| H::keccak256(k).0.to_vec())
	}

	fn receipts_state_trie_key(&self, commitments: Vec<H256>) -> Vec<Vec<u8>> {
		// State trie keys are used to process timeouts from EVM chains.
		// We return the trie keys for request receipts.
		req_receipt_keys::<H>(commitments)
	}

	fn verify_non_membership(
		&self,
		host: &dyn IsmpHost,
		commitments: Vec<H256>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let keys = self.receipts_state_trie_key(commitments);
		let keys_len = keys.len();
		let values = self.verify_state_proof(host, keys, root.state_root, proof)?;
		if values.len() != keys_len {
			return Err(EvmStateMachineError::MismatchedValuesAndKeys.into());
		}
		if values.into_iter().any(|(_key, val)| val.is_some()) {
			return Err(EvmStateMachineError::DeliveredRequestsInBatch.into());
		}
		Ok(())
	}

	fn verify_state_proof(
		&self,
		host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		root: H256,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		let ismp_address = EvmHosts::<T>::get(&proof.height.id.state_id)
			.ok_or(EvmStateMachineError::IsmpContractNotFound)?;
		let keys_len = keys.len();
		let values = verify_state_proof::<H>(keys, root, proof, ismp_address)?;
		if values.len() != keys_len {
			return Err(EvmStateMachineError::MismatchedValuesAndKeys.into());
		}
		Ok(values)
	}
}

#[cfg(test)]
mod bounded_verification_tests {
	use super::*;
	use crate::types::EvmStateProof;
	use codec::Encode;
	use ismp::consensus::{StateMachineHeight, StateMachineId};
	use ismp::host::StateMachine;

	/// A proof whose bytes are irrelevant: the duplicate-key check runs before decoding, so
	/// these tests never reach it. That ordering is the point — a repeated key costs nothing.
	fn dummy_proof() -> Proof {
		Proof {
			height: StateMachineHeight {
				id: StateMachineId {
					state_id: StateMachine::Evm(1),
					consensus_state_id: *b"ETH0",
				},
				height: 1,
			},
			proof: vec![0xde, 0xad, 0xbe, 0xef],
		}
	}

	fn slot_key(byte: u8) -> Vec<u8> {
		vec![byte; 52]
	}

	fn account_key(byte: u8) -> Vec<u8> {
		vec![byte; 20]
	}

	/// Never invoked: the duplicate check runs before any key is hashed, and the control cases
	/// stop at the undecodable proof. Panicking makes that ordering an assertion rather than an
	/// assumption — if hashing is ever reached, these tests fail loudly.
	struct UnusedHasher;
	impl Keccak256 for UnusedHasher {
		fn keccak256(_bytes: &[u8]) -> H256 {
			panic!("hashing must not be reached before the duplicate-key check")
		}
	}

	/// Used where a test must get past key grouping, which hashes every key. The digest itself is
	/// irrelevant here: grouping takes the contract address from `key[..20]`, and the slot hash is
	/// only consulted after the rejection under test, so any total function will do.
	struct StubHasher;
	impl Keccak256 for StubHasher {
		fn keccak256(_bytes: &[u8]) -> H256 {
			H256::zero()
		}
	}

	/// Wraps an encoded state proof in the `Proof` envelope `verify_state_proof` expects.
	fn proof_with(state_proof: EvmStateProof) -> Proof {
		let mut proof = dummy_proof();
		proof.proof = state_proof.encode();
		proof
	}

	fn verify(keys: Vec<Vec<u8>>) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		verify_state_proof::<UnusedHasher>(keys, H256::zero(), &dummy_proof(), H160::zero())
	}

	fn is_duplicate_key_error(err: &Error) -> bool {
		alloc::format!("{err:?}").contains("Duplicate key")
	}

	/// The reported shape: many repeats of one account key. Each repeat used to cost a clone of
	/// the whole contract proof and a trie rebuild before the caller's key-count check rejected
	/// the batch anyway.
	#[test]
	fn rejects_repeated_account_keys_before_touching_the_proof() {
		let keys = alloc::vec![account_key(0xaa); 64];
		let err = verify(keys).expect_err("repeats must be rejected");
		assert!(is_duplicate_key_error(&err), "expected a duplicate-key error, got {err:?}");
	}

	#[test]
	fn rejects_repeated_slot_keys() {
		let err = verify(vec![slot_key(0x11), slot_key(0x11)]).expect_err("repeats rejected");
		assert!(is_duplicate_key_error(&err), "expected a duplicate-key error, got {err:?}");
	}

	/// A repeat anywhere in the batch is caught, not only an adjacent one.
	#[test]
	fn rejects_a_repeat_that_is_not_adjacent() {
		let keys = vec![slot_key(0x01), account_key(0x02), slot_key(0x03), slot_key(0x01)];
		let err = verify(keys).expect_err("repeats rejected");
		assert!(is_duplicate_key_error(&err), "expected a duplicate-key error, got {err:?}");
	}

	/// Control: distinct keys must not be mistaken for repeats. These proceed past the check and
	/// fail later on the deliberately undecodable proof, which is what proves the check let them
	/// through rather than rejecting them up front.
	#[test]
	fn distinct_keys_pass_the_duplicate_check() {
		let keys = vec![slot_key(0x01), slot_key(0x02), account_key(0x03), account_key(0x04)];
		let err = verify(keys).expect_err("the dummy proof cannot decode");
		assert!(
			!is_duplicate_key_error(&err),
			"distinct keys must not be rejected as duplicates, got {err:?}"
		);
	}

	/// A proof carrying a contract nobody asked for is rejected, and rejected before the entry is
	/// resolved — which is why this test needs no real trie data. `keys` is empty, so nothing is
	/// requested and the completeness check passes trivially; the single storage-proof entry is
	/// therefore unrequested by construction.
	#[test]
	fn rejects_a_storage_proof_for_an_unrequested_contract() {
		let mut storage_proof = BTreeMap::new();
		storage_proof.insert(vec![0xab; 20], alloc::vec![alloc::vec![0u8; 8]]);
		let proof = proof_with(EvmStateProof { contract_proof: vec![], storage_proof });

		let err = verify_state_proof::<UnusedHasher>(vec![], H256::zero(), &proof, H160::zero())
			.expect_err("an unrequested contract proof must be rejected");
		assert!(
			alloc::format!("{err:?}").contains("not requested"),
			"expected an unrequested-contract error, got {err:?}"
		);
	}

	/// Padding is rejected even alongside a legitimately requested contract, so an attacker
	/// cannot hide extra accounts behind a real query.
	#[test]
	fn rejects_padding_added_alongside_a_requested_contract() {
		let requested = vec![0x11u8; 20];
		let mut key = requested.clone();
		key.extend_from_slice(&[0x22u8; 32]);

		let mut storage_proof = BTreeMap::new();
		storage_proof.insert(requested, alloc::vec![alloc::vec![0u8; 8]]);
		storage_proof.insert(vec![0xcd; 20], alloc::vec![alloc::vec![0u8; 8]]);
		let proof = proof_with(EvmStateProof { contract_proof: vec![], storage_proof });

		let err = verify_state_proof::<StubHasher>(vec![key], H256::zero(), &proof, H160::zero())
			.expect_err("padding must be rejected");
		assert!(
			alloc::format!("{err:?}").contains("not requested"),
			"expected an unrequested-contract error, got {err:?}"
		);
	}

	/// Control: an exactly-corresponding proof is not rejected by this check. With nothing
	/// requested and nothing supplied the loop has no entries, so verification completes.
	#[test]
	fn an_exactly_corresponding_proof_is_accepted() {
		let proof =
			proof_with(EvmStateProof { contract_proof: vec![], storage_proof: BTreeMap::new() });

		let values = verify_state_proof::<UnusedHasher>(vec![], H256::zero(), &proof, H160::zero())
			.expect("an empty query with an empty proof must verify");
		assert!(values.is_empty());
	}

	#[test]
	fn empty_and_single_key_batches_pass_the_duplicate_check() {
		for keys in [vec![], vec![slot_key(0x01)]] {
			if let Err(err) = verify(keys) {
				assert!(
					!is_duplicate_key_error(&err),
					"must not be rejected as a duplicate, got {err:?}"
				);
			}
		}
	}
}
