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
#![allow(dead_code)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use codec::{Decode, Encode};
use hash256_std_hasher::Hash256StdHasher;
use hash_db::Hasher;
use ismp::messaging::Keccak256;
use primitive_types::H256;

pub struct KeccakHasher<H: Keccak256>(core::marker::PhantomData<H>);

impl<H: Keccak256 + Send + Sync> Hasher for KeccakHasher<H> {
	type Out = H256;
	type StdHasher = Hash256StdHasher;
	const LENGTH: usize = 32;

	fn hash(x: &[u8]) -> Self::Out {
		H::keccak256(x)
	}
}

#[derive(Encode, Decode, Clone)]
pub struct EvmStateProof {
	/// Contract account proof
	pub contract_proof: Vec<Vec<u8>>,
	/// A map of contract address to the associated account trie proof for all keys requested from
	/// the contract
	pub storage_proof: BTreeMap<Vec<u8>, Vec<Vec<u8>>>,
}

#[derive(Encode, Decode, Clone)]
pub struct EvmKVProof {
	/// The raw value bytes stored under the queried key
	pub value: Vec<u8>,
	/// ICS23 proof ops to verify the value against the app hash
	pub proof: Vec<u8>,
}

/// The ethereum account stored in the global state trie.
#[derive(RlpDecodable, RlpEncodable)]
pub struct Account {
	pub nonce: u64,
	pub balance: alloy_primitives::U256,
	pub storage_root: alloy_primitives::B256,
	pub code_hash: alloy_primitives::B256,
}
