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

//! Child trie storage layout for pallet-hyperbridge
//!
//! pallet-hyperbridge leverages a child trie to store the payment receipts for outgoing requests
//! and responses. This ensures that hyperbridge can verify that requests have been paid for
//! before it routes them to their destination chain.

use alloc::vec::Vec;
use polkadot_sdk::*;

use frame_support::storage::{child, child::ChildInfo};
use primitive_types::H256;

// we share the same child trie prefix as pallet-ismp
use pallet_ismp::child_trie::CHILD_TRIE_PREFIX;

/// Stores the payment receipts for outgoing requests. The key is the request commitment
pub struct RequestPayments;

/// Stores the payment receipts for outgoing responses. The key is the response commitment
pub struct ResponsePayments;

/// Returns the storage key for a request commitment in the child trie
pub fn request_payment_storage_key(key: H256) -> Vec<u8> {
	let mut full_key = "RequestPayment".as_bytes().to_vec();
	full_key.extend_from_slice(&key.0);
	full_key
}

/// Returns the storage key for a response commitment in the child trie
pub fn response_payment_storage_key(key: H256) -> Vec<u8> {
	let mut full_key = "ResponsePayment".as_bytes().to_vec();
	full_key.extend_from_slice(&key.0);
	full_key
}

impl RequestPayments {
	/// Returns the hashed storage key
	pub fn storage_key(key: H256) -> Vec<u8> {
		request_payment_storage_key(key)
	}

	/// Get the provided key from the child trie
	pub fn get(key: H256) -> Option<u128> {
		child::get(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Insert the key and value into the child trie
	pub fn insert(key: H256, meta: u128) {
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

impl ResponsePayments {
	/// Returns the hashed storage key
	pub fn storage_key(key: H256) -> Vec<u8> {
		response_payment_storage_key(key)
	}

	/// Get the provided key from the child trie
	pub fn get(key: H256) -> Option<u128> {
		child::get(&ChildInfo::new_default(CHILD_TRIE_PREFIX), &Self::storage_key(key))
	}

	/// Insert the key and value into the child trie
	pub fn insert(key: H256, meta: u128) {
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
