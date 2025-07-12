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

use alloc::{collections::BTreeMap, format, string::ToString, vec, vec::Vec};
use codec::{Decode, Encode};
use core::{fmt::Debug, marker::PhantomData, time::Duration};
use frame_support::{ensure, traits::Get};
use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::{hash_post_response, hash_request, hash_response, Proof},
	router::{Request, RequestResponse, Response},
};
use pallet_ismp::{
	child_trie::{RequestCommitments, RequestReceipts, ResponseCommitments, ResponseReceipts},
	ConsensusDigest, TimestampDigest, ISMP_ID, ISMP_TIMESTAMP_ID,
};
use polkadot_sdk::*;
use sp_consensus_aura::{Slot, AURA_ENGINE_ID};
use sp_consensus_babe::{digests::PreDigest, BABE_ENGINE_ID};
use sp_runtime::{
	traits::{BlakeTwo256, Keccak256},
	Digest, DigestItem,
};
use sp_trie::{HashDBT, LayoutV0, StorageProof, Trie, TrieDBBuilder, EMPTY_PREFIX};
use trie_db::TrieError;

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
		item: RequestResponse,
		state: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let state_proof: SubstrateStateProof = codec::Decode::decode(&mut &*proof.proof)
			.map_err(|e| Error::Custom(format!("failed to decode proof: {e:?}")))?;
		ensure!(
			matches!(state_proof, SubstrateStateProof::OverlayProof { .. }),
			Error::Custom("Expected Overlay Proof".to_string())
		);

		let root = match T::Coprocessor::get() {
			Some(id) if id == proof.height.id.state_id => state.state_root,
			_ => state.overlay_root.ok_or_else(|| {
				Error::Custom(
					"Child trie root is not available for provided state commitment".into(),
				)
			})?,
		};

		let keys = match item {
			RequestResponse::Request(requests) => requests
				.into_iter()
				.map(|request| {
					let commitment = hash_request::<pallet_ismp::Pallet<T>>(&request);
					RequestCommitments::<T>::storage_key(commitment)
				})
				.collect::<Vec<Vec<u8>>>(),
			RequestResponse::Response(responses) => responses
				.into_iter()
				.map(|response| {
					let commitment = hash_response::<pallet_ismp::Pallet<T>>(&response);
					ResponseCommitments::<T>::storage_key(commitment)
				})
				.collect::<Vec<Vec<u8>>>(),
		};
		let _ = match state_proof.hasher() {
			HashAlgorithm::Keccak => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<Keccak256>();
				let trie = TrieDBBuilder::<LayoutV0<Keccak256>>::new(&db, &root).build();
				keys.into_iter()
                    .map(|key| {
                        let value = trie.get(&key).map_err(|e| {
                            Error::Custom(format!(
                                "SubstrateStateMachine: Error reading Keccak state proof: {e:?}"
                            ))
                        })?.ok_or_else(|| Error::Custom(format!(
                            "Every key in a membership proof should have a value, found a key {:?} with None", key
                        )))?;
                        Ok((key, value))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?
			},
			HashAlgorithm::Blake2 => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<BlakeTwo256>();

				let trie = TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root).build();
				keys.into_iter()
                    .map(|key| {
                        let value = trie.get(&key).map_err(|e| {
                            Error::Custom(format!(
                                "SubstrateStateMachine: Error reading Blake2 state proof: {e:?}"
                            ))
                        })?.ok_or_else(|| Error::Custom(format!(
                            "Every key in a membership proof should have a value, found a key {:?} with None", key
                        )))?;
                        Ok((key, value))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?
			},
		};

		Ok(())
	}

	fn receipts_state_trie_key(&self, items: RequestResponse) -> Vec<Vec<u8>> {
		let mut keys = vec![];
		match items {
			RequestResponse::Request(requests) =>
				for req in requests {
					match req {
						Request::Post(post) => {
							let request = Request::Post(post);
							let commitment = hash_request::<pallet_ismp::Pallet<T>>(&request);
							keys.push(RequestReceipts::<T>::storage_key(commitment));
						},
						Request::Get(_) => continue,
					}
				},
			RequestResponse::Response(responses) =>
				for res in responses {
					match res {
						Response::Post(post_response) => {
							let commitment =
								hash_post_response::<pallet_ismp::Pallet<T>>(&post_response);
							keys.push(ResponseReceipts::<T>::storage_key(commitment));
						},
						Response::Get(_) => continue,
					}
				},
		};

		keys
	}

	fn verify_state_proof(
		&self,
		_host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		let state_proof: SubstrateStateProof = codec::Decode::decode(&mut &*proof.proof)
			.map_err(|e| Error::Custom(format!("failed to decode proof: {e:?}")))?;
		let root = match &state_proof {
			SubstrateStateProof::OverlayProof { .. } => {
				match T::Coprocessor::get() {
					Some(id) if id == proof.height.id.state_id => root.state_root,
					// child root on hyperbridge
					_ => root.overlay_root.ok_or_else(|| {
						Error::Custom(
							"Child trie root is not available for provided state commitment".into(),
						)
					})?,
				}
			},
			SubstrateStateProof::StateProof { .. } => root.state_root,
		};
		let data = match state_proof.hasher() {
			HashAlgorithm::Keccak => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<Keccak256>();
				let trie = TrieDBBuilder::<LayoutV0<Keccak256>>::new(&db, &root).build();
				keys.into_iter()
					.map(|key| {
						let value = trie.get(&key).map_err(|e| {
							Error::Custom(format!("Error reading state proof: {e:?}"))
						})?;
						Ok((key, value))
					})
					.collect::<Result<BTreeMap<_, _>, _>>()?
			},
			HashAlgorithm::Blake2 => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<BlakeTwo256>();

				let trie = TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root).build();
				keys.into_iter()
					.map(|key| {
						let value = trie.get(&key).map_err(|e| {
							Error::Custom(format!("Error reading state proof: {e:?}"))
						})?;
						Ok((key, value))
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
