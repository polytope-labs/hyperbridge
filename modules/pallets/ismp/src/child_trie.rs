// Copyright (c) 2025 Polytope Labs.
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

//! Child trie storage layout for pallet-ismp
//!
//! pallet-ismp leverages a child trie to store outgoing/incoming requests and responses. This is
//! because child tries provide cheaper state proofs compared the global state trie. This module
//! describes the storage layout in the child trie.

use crate::{dispatcher::RequestMetadata, utils::ResponseReceipt, Config};
use alloc::vec::Vec;
use codec::Encode;
use core::marker::PhantomData;
use frame_support::storage::child;
use ismp::consensus::{StateCommitment, StateMachineHeight};
use polkadot_sdk::*;
use sp_core::{storage::ChildInfo, H256};

/// Commitments for outgoing requests
/// The key is the request commitment
pub struct RequestCommitments<T: Config>(PhantomData<T>);

/// Receipts for incoming requests
/// The key is the request commitment
pub struct RequestReceipts<T: Config>(PhantomData<T>);

/// Commitments for outgoing responses
/// The key is the response commitment
pub struct ResponseCommitments<T: Config>(PhantomData<T>);

/// Receipts for incoming responses
/// The key is the request commitment
pub struct ResponseReceipts<T: Config>(PhantomData<T>);

/// State commitments are inserted into the child trie
/// for more efficient state reads of these aggregated states
/// by 3rd party applications
pub struct StateCommitments<T: Config>(PhantomData<T>);

/// Child trie prefix for all substrate chains
pub const CHILD_TRIE_PREFIX: &'static [u8] = b"ISMP";

/// Key for the state commitments in the child trie
pub const STATE_COMMITMENTS_KEY: &'static [u8] = b"state";

/// Returns the storage key for a request commitment in the child trie
pub fn request_commitment_storage_key(key: H256) -> Vec<u8> {
	let mut full_key = "RequestCommitments".as_bytes().to_vec();
	full_key.extend_from_slice(&key.0);
	full_key
}

/// Returns the storage key for a response commitment in the child trie
pub fn response_commitment_storage_key(key: H256) -> Vec<u8> {
	let mut full_key = "ResponseCommitments".as_bytes().to_vec();
	full_key.extend_from_slice(&key.0);
	full_key
}

/// Returns the storage key for a request receipt in the child trie
pub fn request_receipt_storage_key(key: H256) -> Vec<u8> {
	let mut full_key = "RequestReceipts".as_bytes().to_vec();
	full_key.extend_from_slice(&key.0);
	full_key
}

/// Returns the storage key for a response receipt in the child trie
/// The request commitment is the key
pub fn response_receipt_storage_key(key: H256) -> Vec<u8> {
	let mut full_key = "ResponseReceipts".as_bytes().to_vec();
	full_key.extend_from_slice(&key.0);
	full_key
}

/// Returns the storage key for a state commitment in the child trie
pub fn state_commitment_storage_key(height: StateMachineHeight) -> Vec<u8> {
	[STATE_COMMITMENTS_KEY.to_vec(), sp_io::hashing::keccak_256(&height.encode()).to_vec()].concat()
}

impl<T: Config> StateCommitments<T> {
	/// Returns the hashed storage key
	pub fn storage_key(key: StateMachineHeight) -> Vec<u8> {
		state_commitment_storage_key(key)
	}

	/// Get the provided key from the child trie
	/// child tree reads are more pov-efficient
	pub fn get(key: StateMachineHeight) -> Option<StateCommitment> {
		child::get(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
			.or(crate::StateCommitments::<T>::get(&key))
	}

	/// Insert the key and value into the child trie
	pub fn insert(key: StateMachineHeight, meta: StateCommitment) {
		crate::StateCommitments::<T>::insert(key.clone(), meta.clone());
		child::put(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key), &meta);
	}

	/// Remove the key from the child trie
	pub fn remove(key: StateMachineHeight) {
		crate::StateCommitments::<T>::remove(&key);
		child::kill(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Return true if key is contained in child trie
	pub fn contains_key(key: StateMachineHeight) -> bool {
		child::exists(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}
}

impl<T: Config> RequestCommitments<T> {
	/// Returns the hashed storage key
	pub fn storage_key(key: H256) -> Vec<u8> {
		request_commitment_storage_key(key)
	}

	/// Get the provided key from the child trie
	pub fn get(key: H256) -> Option<RequestMetadata<T>> {
		child::get(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Insert the key and value into the child trie
	pub fn insert(key: H256, meta: RequestMetadata<T>) {
		child::put(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key), &meta);
	}

	/// Remove the key from the child trie
	pub fn remove(key: H256) {
		child::kill(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Return true if key is contained in child trie
	pub fn contains_key(key: H256) -> bool {
		child::exists(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}
}

impl<T: Config> ResponseCommitments<T> {
	/// Returns the hashed storage key
	pub fn storage_key(key: H256) -> Vec<u8> {
		response_commitment_storage_key(key)
	}

	/// Get the provided key from the child trie
	pub fn get(key: H256) -> Option<RequestMetadata<T>> {
		child::get(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Insert the key and value into the child trie
	pub fn insert(key: H256, meta: RequestMetadata<T>) {
		child::put(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key), &meta);
	}

	/// Remove the key from the child trie
	pub fn remove(key: H256) {
		child::kill(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Return true if key is contained in child trie
	pub fn contains_key(key: H256) -> bool {
		child::exists(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}
}

impl<T: Config> RequestReceipts<T> {
	/// Returns the hashed storage key
	pub fn storage_key(key: H256) -> Vec<u8> {
		request_receipt_storage_key(key)
	}

	/// Get the provided key from the child trie
	pub fn get(key: H256) -> Option<Vec<u8>> {
		child::get(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Insert the key and value into the child trie
	pub fn insert(key: H256, relayer: &[u8]) {
		child::put(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key), &relayer);
	}

	/// Remove the key from the child trie
	pub fn remove(key: H256) {
		child::kill(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Return true if key is contained in child trie
	pub fn contains_key(key: H256) -> bool {
		child::exists(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}
}

impl<T: Config> ResponseReceipts<T> {
	/// Returns the hashed storage key
	pub fn storage_key(key: H256) -> Vec<u8> {
		response_receipt_storage_key(key)
	}

	/// Get the provided key from the child trie
	pub fn get(key: H256) -> Option<ResponseReceipt> {
		child::get(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Insert the key and value into the child trie
	pub fn insert(key: H256, receipt: ResponseReceipt) {
		child::put(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key), &receipt);
	}

	/// Remove the key from the child trie
	pub fn remove(key: H256) {
		child::kill(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Return true if key is contained in child trie
	pub fn contains_key(key: H256) -> bool {
		child::exists(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}
}
