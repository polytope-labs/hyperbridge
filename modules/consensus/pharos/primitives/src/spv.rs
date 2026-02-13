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

//! Pharos hexary hash tree SPV (Simple Payment Verification) proof verification.
//!
//! Pharos uses a hexary hash tree with SHA-256 hashing instead of Ethereum's
//! Merkle-Patricia Trie with Keccak-256. This module implements bottom-up
//! proof verification matching the Pharos proof format.
//!
//! ## Proof Structure
//!
//! Each proof is an ordered list of nodes from root to leaf:
//! - **Leaf node** (last): 1 byte metadata + 32 bytes `sha256(key)` + 32 bytes `sha256(value)`
//! - **Internal node**: 3 bytes metadata + N × 32 byte child hashes (variable branching)
//!
//! ## Verification Algorithm (bottom-up)
//!
//! Verify leaf: `proof_node[1:33] == sha256(key)` and `proof_node[33:65] == sha256(value)`
//! Walk bottom-up: for each parent, find current hash at `[begin_offset..end_offset]`
//! Hash current node: `current_hash = sha256(proof_node)`
//! Root check: final hash == expected root

use crate::types::PharosProofNode;
use sha2::{Digest, Sha256};

/// Compute SHA-256 hash of the given data.
pub fn sha256(data: &[u8]) -> [u8; 32] {
	let mut hasher = Sha256::new();
	hasher.update(data);
	hasher.finalize().into()
}

/// Verify a Pharos hexary hash tree proof (bottom-up).
///
/// `proof_nodes` are ordered root-to-leaf (index 0 = root, last = leaf).
/// `key` is the raw key bytes (address for accounts, address||slot_key for storage).
/// `value` is the raw value bytes (rawValue for accounts, 32-byte padded value for storage).
/// `root` is the expected root hash (stateRoot or storageHash).
///
/// Returns `true` if the proof is valid.
pub fn verify_pharos_proof(
	proof_nodes: &[PharosProofNode],
	key: &[u8],
	value: &[u8],
	root: &[u8; 32],
) -> bool {
	if proof_nodes.is_empty() {
		return false;
	}

	// Verify the leaf node (last in the array)
	let leaf = &proof_nodes[proof_nodes.len() - 1];
	let leaf_data = &leaf.proof_node;

	// Leaf: 1 byte metadata + 32 bytes sha256(key) + 32 bytes sha256(value) = 65 bytes
	if leaf_data.len() != 65 {
		return false;
	}

	let key_hash = sha256(key);
	let value_hash = sha256(value);

	// Verify key hash at bytes [1..33]
	if leaf_data[1..33] != key_hash {
		return false;
	}

	// Verify value hash at bytes [33..65]
	if leaf_data[33..65] != value_hash {
		return false;
	}

	// Walk bottom-up, hashing each node and checking the parent contains it
	let mut current_hash = sha256(leaf_data);

	// Iterate from second-to-last to first (bottom-up, skipping the leaf)
	for i in (0..proof_nodes.len() - 1).rev() {
		let parent = &proof_nodes[i];
		let begin = parent.next_begin_offset as usize;
		let end = parent.next_end_offset as usize;

		// Validate offsets
		if end > parent.proof_node.len() || begin >= end || (end - begin) != 32 {
			return false;
		}

		// Check that the current hash appears at the expected position in the parent
		if parent.proof_node[begin..end] != current_hash {
			return false;
		}

		// Hash this parent node to get the hash to check in the next parent
		current_hash = sha256(&parent.proof_node);
	}

	// Final hash should equal the expected root
	current_hash == *root
}

/// Verify an account proof against the state root.
///
/// `address` is the 20-byte account address.
/// `raw_value` is the RLP-encoded account value (rawValue from eth_getProof).
/// `state_root` is the state root from the block header.
pub fn verify_account_proof(
	proof_nodes: &[PharosProofNode],
	address: &[u8; 20],
	raw_value: &[u8],
	state_root: &[u8; 32],
) -> bool {
	// For account proofs, the key is just the address bytes
	verify_pharos_proof(proof_nodes, address, raw_value, state_root)
}

/// Verify a storage proof for a single key.
///
/// `address` is the 20-byte contract address.
/// `storage_key` is the 32-byte storage slot hash.
/// `storage_value` is the 32-byte padded storage value.
/// `storage_hash` is the storage trie root from the account proof.
pub fn verify_storage_proof(
	proof_nodes: &[PharosProofNode],
	address: &[u8; 20],
	storage_key: &[u8; 32],
	storage_value: &[u8; 32],
	storage_hash: &[u8; 32],
) -> bool {
	// For storage proofs, the key is address || storage_key (52 bytes)
	let mut key = [0u8; 52];
	key[..20].copy_from_slice(address);
	key[20..].copy_from_slice(storage_key);

	verify_pharos_proof(proof_nodes, &key, storage_value, storage_hash)
}