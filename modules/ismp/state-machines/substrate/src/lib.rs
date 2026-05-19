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

//! The [`StateMachineClient`] implementation for substrate state machines

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::{
	collections::BTreeMap,
	format,
	string::{String, ToString},
	vec::Vec,
};
use codec::{Decode, Encode};
use core::{fmt::Debug, marker::PhantomData, time::Duration};
use frame_support::traits::Get;
use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::Proof,
};
use pallet_ismp::{
	child_trie::{RequestCommitments, RequestReceipts},
	ConsensusDigest, TimestampDigest, ISMP_ID, ISMP_TIMESTAMP_ID,
};
use polkadot_sdk::*;
use primitive_types::H256;
use sp_consensus_aura::{Slot, AURA_ENGINE_ID};
use sp_consensus_babe::{digests::PreDigest, BABE_ENGINE_ID};
use sp_runtime::{
	traits::{BlakeTwo256, Keccak256},
	Digest, DigestItem,
};
use sp_trie::{HashDBT, LayoutV0, StorageProof, Trie, TrieDBBuilder, EMPTY_PREFIX};
use thiserror::Error as ThisError;
use trie_db::TrieError;

/// Errors produced by the substrate state machine client.
#[derive(Debug, ThisError)]
pub enum SubstrateStateMachineError {
	/// Failed to SCALE-decode the supplied proof.
	#[error("Failed to decode proof: {0:?}")]
	ProofDecodeError(codec::Error),
	/// Caller supplied a `StateProof` variant for a membership/non-membership check
	/// that must be served from the ISMP child trie via an `OverlayProof`.
	#[error("Expected Overlay Proof")]
	ExpectedOverlayProof,
	/// The state commitment doesn't include the child trie root, and the request
	/// state machine is not the coprocessor.
	#[error("Child trie root is not available for provided state commitment")]
	MissingChildTrieRoot,
	/// The trie backend returned an error while reading a key.
	#[error("Trie error: {0}")]
	TrieError(String),
	/// A membership proof omitted the value for one of the requested keys.
	#[error("Every key in a membership proof should have a value, found a key {0:?} with None")]
	MissingMembershipValue(Vec<u8>),
	/// A non-membership proof contained at least one delivered request.
	#[error("Some Requests in the batch have been delivered")]
	DeliveredRequestsInBatch,
}

impl From<SubstrateStateMachineError> for Error {
	fn from(e: SubstrateStateMachineError) -> Error {
		Error::AnyHow(anyhow::Error::new(e).into())
	}
}

/// Hashing algorithm for the state proof
#[derive(
	Debug, Encode, Decode, Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq, Eq,
)]
pub enum HashAlgorithm {
	/// For chains that use keccak as their hashing algo
	Keccak,
	/// For chains that use blake2 as their hashing algo
	Blake2,
}

/// The substrate state machine proof. This will be a base-16 merkle patricia proof.
/// It's [`TrieLayout`](sp_trie::TrieLayout) will be the [`LayoutV0`]
#[derive(Debug, Encode, Decode, Clone)]
pub struct StateMachineProof {
	/// Algorithm to use for state proof verification
	pub hasher: HashAlgorithm,
	/// Intermediate trie nodes in the key path from the root to their relevant values.
	pub storage_proof: Vec<Vec<u8>>,
}

/// Holds the relevant data needed for state proof verification
#[derive(Debug, Encode, Decode, Clone)]
pub enum SubstrateStateProof {
	/// Uses overlay root for verification
	OverlayProof(StateMachineProof),
	/// Uses state root for verification
	StateProof(StateMachineProof),
}

impl SubstrateStateProof {
	/// Returns hash algo
	pub fn hasher(&self) -> HashAlgorithm {
		match self {
			Self::OverlayProof(proof) => proof.hasher,
			Self::StateProof(proof) => proof.hasher,
		}
	}

	/// Returns storage proof
	pub fn storage_proof(self) -> Vec<Vec<u8>> {
		match self {
			Self::OverlayProof(proof) => proof.storage_proof,
			Self::StateProof(proof) => proof.storage_proof,
		}
	}
}

/// The [`StateMachineClient`] implementation for substrate state machines. Assumes requests are
/// stored in a child trie.
pub struct SubstrateStateMachine<T>(PhantomData<T>);

impl<T> Default for SubstrateStateMachine<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T> From<StateMachine> for SubstrateStateMachine<T> {
	fn from(_: StateMachine) -> Self {
		Self::default()
	}
}

impl<T> StateMachineClient for SubstrateStateMachine<T>
where
	T: pallet_ismp::Config,
{
	fn verify_membership(
		&self,
		_host: &dyn IsmpHost,
		commitments: Vec<H256>,
		state: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let state_proof: SubstrateStateProof = codec::Decode::decode(&mut &*proof.proof)
			.map_err(SubstrateStateMachineError::ProofDecodeError)?;
		if !matches!(state_proof, SubstrateStateProof::OverlayProof { .. }) {
			return Err(SubstrateStateMachineError::ExpectedOverlayProof.into());
		}

		let root = match T::Coprocessor::get() {
			Some(id) if id == proof.height.id.state_id => state.state_root,
			_ => state.overlay_root.ok_or(SubstrateStateMachineError::MissingChildTrieRoot)?,
		};

		let keys = self.commitment_state_trie_key(commitments);
		let read_value = |key: Vec<u8>, value: Option<Vec<u8>>| {
			value
				.ok_or_else(|| SubstrateStateMachineError::MissingMembershipValue(key.clone()))
				.map(|v| (key, v))
		};
		match state_proof.hasher() {
			HashAlgorithm::Keccak => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<Keccak256>();
				let trie = TrieDBBuilder::<LayoutV0<Keccak256>>::new(&db, &root).build();
				for key in keys {
					let value = trie
						.get(&key)
						.map_err(|e| SubstrateStateMachineError::TrieError(format!("{e:?}")))?;
					read_value(key, value)?;
				}
			},
			HashAlgorithm::Blake2 => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<BlakeTwo256>();
				let trie = TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root).build();
				for key in keys {
					let value = trie
						.get(&key)
						.map_err(|e| SubstrateStateMachineError::TrieError(format!("{e:?}")))?;
					read_value(key, value)?;
				}
			},
		}

		Ok(())
	}

	fn commitment_state_trie_key(&self, commitments: Vec<H256>) -> Vec<Vec<u8>> {
		commitments.into_iter().map(RequestCommitments::<T>::storage_key).collect()
	}

	fn receipts_state_trie_key(&self, commitments: Vec<H256>) -> Vec<Vec<u8>> {
		commitments.into_iter().map(RequestReceipts::<T>::storage_key).collect()
	}

	fn verify_non_membership(
		&self,
		_host: &dyn IsmpHost,
		commitments: Vec<H256>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let state_proof: SubstrateStateProof = codec::Decode::decode(&mut &*proof.proof)
			.map_err(SubstrateStateMachineError::ProofDecodeError)?;
		if !matches!(state_proof, SubstrateStateProof::OverlayProof { .. }) {
			return Err(SubstrateStateMachineError::ExpectedOverlayProof.into());
		}

		let root = match T::Coprocessor::get() {
			Some(id) if id == proof.height.id.state_id => root.state_root,
			_ => root.overlay_root.ok_or(SubstrateStateMachineError::MissingChildTrieRoot)?,
		};

		let keys = self.receipts_state_trie_key(commitments);

		let check_absent = |value: Option<Vec<u8>>| -> Result<(), SubstrateStateMachineError> {
			if value.is_some() {
				Err(SubstrateStateMachineError::DeliveredRequestsInBatch)
			} else {
				Ok(())
			}
		};

		match state_proof.hasher() {
			HashAlgorithm::Keccak => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<Keccak256>();
				let trie = TrieDBBuilder::<LayoutV0<Keccak256>>::new(&db, &root).build();
				for key in keys {
					let value = trie
						.get(&key)
						.map_err(|e| SubstrateStateMachineError::TrieError(format!("{e:?}")))?;
					check_absent(value)?;
				}
			},
			HashAlgorithm::Blake2 => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<BlakeTwo256>();
				let trie = TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root).build();
				for key in keys {
					let value = trie
						.get(&key)
						.map_err(|e| SubstrateStateMachineError::TrieError(format!("{e:?}")))?;
					check_absent(value)?;
				}
			},
		}

		Ok(())
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		root: H256,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		// The trie root is supplied by the caller, bound to the calling context. The
		// `SubstrateStateProof` variant tag is no longer used to select the root, so a relayer
		// can no longer steer verification at the wrong trie.
		let state_proof: SubstrateStateProof = codec::Decode::decode(&mut &*proof.proof)
			.map_err(SubstrateStateMachineError::ProofDecodeError)?;
		let data = match state_proof.hasher() {
			HashAlgorithm::Keccak => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<Keccak256>();
				let trie = TrieDBBuilder::<LayoutV0<Keccak256>>::new(&db, &root).build();
				keys.into_iter()
					.map(|key| {
						let value = trie
							.get(&key)
							.map_err(|e| SubstrateStateMachineError::TrieError(format!("{e:?}")))?;
						Ok::<_, SubstrateStateMachineError>((key, value))
					})
					.collect::<Result<BTreeMap<_, _>, _>>()?
			},
			HashAlgorithm::Blake2 => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<BlakeTwo256>();
				let trie = TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root).build();
				keys.into_iter()
					.map(|key| {
						let value = trie
							.get(&key)
							.map_err(|e| SubstrateStateMachineError::TrieError(format!("{e:?}")))?;
						Ok::<_, SubstrateStateMachineError>((key, value))
					})
					.collect::<Result<BTreeMap<_, _>, _>>()?
			},
		};

		Ok(data)
	}
}

/// Lifted directly from [`sp_state_machine::read_proof_check`](https://github.com/paritytech/substrate/blob/b27c470eaff379f512d1dec052aff5d551ed3b03/primitives/state-machine/src/lib.rs#L1075-L1094)
pub fn read_proof_check<H, I>(
	root: &H::Out,
	proof: StorageProof,
	keys: I,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error>
where
	H: hash_db::Hasher,
	H::Out: Debug,
	I: IntoIterator,
	I::Item: AsRef<[u8]>,
{
	let db = proof.into_memory_db();

	if !db.contains(root, EMPTY_PREFIX) {
		Err(Error::Custom("Invalid Proof".into()))?
	}

	let trie = TrieDBBuilder::<LayoutV0<H>>::new(&db, root).build();
	let mut result = BTreeMap::new();

	for key in keys.into_iter() {
		let value = trie
			.get(key.as_ref())
			.map_err(|e| Error::Custom(format!("Error reading from trie: {e:?}")))?
			.and_then(|val| Decode::decode(&mut &val[..]).ok());
		result.insert(key.as_ref().to_vec(), value);
	}

	Ok(result)
}

/// Lifted directly from [`sp_state_machine::read_proof_check`](https://github.com/paritytech/substrate/blob/b27c470eaff379f512d1dec052aff5d551ed3b03/primitives/state-machine/src/lib.rs#L1075-L1094)
pub fn read_proof_check_for_parachain<H, I>(
	root: &H::Out,
	proof: StorageProof,
	keys: I,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error>
where
	H: hash_db::Hasher,
	H::Out: Debug,
	I: IntoIterator,
	I::Item: AsRef<[u8]>,
{
	let db = proof.into_memory_db();

	if !db.contains(root, EMPTY_PREFIX) {
		Err(Error::Custom("Invalid Proof".into()))?
	}

	let trie = TrieDBBuilder::<LayoutV0<H>>::new(&db, root).build();
	let mut result = BTreeMap::new();

	for key in keys {
		let raw_key = key.as_ref();

		match trie.get(raw_key) {
			Ok(Some(val)) => {
				let decoded = Decode::decode(&mut &val[..])
					.map_err(|e| Error::Custom(format!("Decode error: {e:?}")))?;
				result.insert(raw_key.to_vec(), Some(decoded));
			},
			Ok(None) => {
				result.insert(raw_key.to_vec(), None);
			},
			Err(e) =>
				if let TrieError::IncompleteDatabase(_) = *e {
					continue;
				} else {
					return Err(Error::Custom(format!("Trie fetch error: {e:?}",)));
				},
		}
	}

	Ok(result)
}

/// Result for processing consensus digest logs
#[derive(Default)]
pub struct DigestResult {
	/// Timestamp
	pub timestamp: u64,
	/// Ismp digest
	pub ismp_digest: ConsensusDigest,
}

/// Fetches the overlay (ismp) root and timestamp from the header digest
pub fn fetch_overlay_root_and_timestamp(
	digest: &Digest,
	slot_duration: u64,
) -> Result<DigestResult, Error> {
	let mut digest_result = DigestResult::default();

	for digest in digest.logs.iter() {
		match digest {
			DigestItem::Consensus(consensus_engine_id, value)
				if *consensus_engine_id == ISMP_TIMESTAMP_ID =>
			{
				let timestamp_digest = TimestampDigest::decode(&mut &value[..]).map_err(|e| {
					Error::Custom(format!("Failed to decode timestamp digest: {e:?}"))
				})?;
				digest_result.timestamp = timestamp_digest.timestamp;
			},
			DigestItem::PreRuntime(consensus_engine_id, value)
				if *consensus_engine_id == AURA_ENGINE_ID =>
			{
				let slot = Slot::decode(&mut &value[..])
					.map_err(|e| Error::Custom(format!("Cannot slot: {e:?}")))?;
				digest_result.timestamp = Duration::from_millis(*slot * slot_duration).as_secs();
			},
			DigestItem::PreRuntime(consensus_engine_id, value)
				if *consensus_engine_id == BABE_ENGINE_ID =>
			{
				let slot = PreDigest::decode(&mut &value[..])
					.map_err(|e| Error::Custom(format!("Cannot slot: {e:?}")))?
					.slot();
				digest_result.timestamp = Duration::from_millis(*slot * slot_duration).as_secs();
			},
			DigestItem::Consensus(consensus_engine_id, value)
				if *consensus_engine_id == ISMP_ID =>
			{
				let digest = ConsensusDigest::decode(&mut &value[..])
					.map_err(|e| Error::Custom(format!("Failed to decode digest: {e:?}")))?;

				digest_result.ismp_digest = digest
			},
			// don't really care about the rest
			_ => {},
		};
	}

	Ok(digest_result)
}
