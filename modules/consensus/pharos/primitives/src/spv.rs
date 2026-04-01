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

//! Pharos hexary hash tree SPV proof verification.
//!
//! Node types: MSU Root (8192 bytes), Internal (515 bytes), Leaf (65 bytes).
//! Internal nodes use SkipEmpty hashing: `sha256(header || non-zero child slots)`.

use crate::types::{PharosProofNode, SiblingLeftmostLeafProof};

const INTERNAL_NODE_HEADER: usize = 3;
const INTERNAL_NODE_SLOTS: usize = 16;
const INTERNAL_NODE_SLOT_SIZE: usize = 32;
const INTERNAL_NODE_LEN: usize =
	INTERNAL_NODE_HEADER + INTERNAL_NODE_SLOTS * INTERNAL_NODE_SLOT_SIZE;
const MSU_ROOT_NODE_LEN: usize = 256 * INTERNAL_NODE_SLOT_SIZE;
const LEAF_NODE_LEN: usize = 65;
const LEAF_NODE_TYPE: u8 = 1;
const ZERO_HASH: [u8; 32] = [0u8; 32];

pub fn sha256(data: &[u8]) -> [u8; 32] {
	sp_io::hashing::sha2_256(data)
}

/// Pharos nibble extraction: low nibble first at even depths, high nibble at odd depths.
pub fn nibble_at_depth(key_hash: &[u8], depth: usize) -> u8 {
	let byte_index = depth / 2;
	if depth % 2 == 0 {
		key_hash[byte_index] & 0x0F
	} else {
		(key_hash[byte_index] >> 4) & 0x0F
	}
}

fn is_zero_slot(slot: &[u8]) -> bool {
	slot.iter().all(|&b| b == 0)
}

/// SkipEmpty: `sha256(3-byte header || non-zero slots)`. All-zero node hashes to `[0; 32]`.
fn hash_internal_node(proof_node: &[u8]) -> [u8; 32] {
	let all_empty = (0..INTERNAL_NODE_SLOTS).all(|i| {
		let start = INTERNAL_NODE_HEADER + i * INTERNAL_NODE_SLOT_SIZE;
		is_zero_slot(&proof_node[start..start + INTERNAL_NODE_SLOT_SIZE])
	});

	if all_empty {
		return ZERO_HASH;
	}

	let mut data = Vec::with_capacity(INTERNAL_NODE_LEN);
	data.extend_from_slice(&proof_node[..INTERNAL_NODE_HEADER]);
	for i in 0..INTERNAL_NODE_SLOTS {
		let start = INTERNAL_NODE_HEADER + i * INTERNAL_NODE_SLOT_SIZE;
		let slot = &proof_node[start..start + INTERNAL_NODE_SLOT_SIZE];
		if !is_zero_slot(slot) {
			data.extend_from_slice(slot);
		}
	}
	sha256(&data)
}

fn compute_node_hash(proof_node: &[u8]) -> Option<[u8; 32]> {
	match proof_node.len() {
		LEAF_NODE_LEN => Some(sha256(proof_node)),
		INTERNAL_NODE_LEN => Some(hash_internal_node(proof_node)),
		MSU_ROOT_NODE_LEN => Some(sha256(proof_node)),
		_ => None,
	}
}

fn is_leaf(node: &[u8]) -> bool {
	node.len() == LEAF_NODE_LEN && node[0] == LEAF_NODE_TYPE
}

/// Bottom-up hash chain walk from last node to root.
fn verify_proof_walk(proof_nodes: &[PharosProofNode], root: &[u8; 32]) -> bool {
	let Some(last) = proof_nodes.last() else { return false };
	let Some(mut current_hash) = compute_node_hash(&last.proof_node) else { return false };

	for i in (0..proof_nodes.len() - 1).rev() {
		let parent = &proof_nodes[i];
		let begin = parent.next_begin_offset as usize;
		let end = parent.next_end_offset as usize;

		if end > parent.proof_node.len() || begin >= end || (end - begin) != 32 {
			return false;
		}

		if parent.proof_node[begin..end] != current_hash {
			return false;
		}

		current_hash = match compute_node_hash(&parent.proof_node) {
			Some(h) => h,
			None => return false,
		};
	}

	current_hash == *root
}

fn build_storage_key(address: &[u8; 20], slot_hash: &[u8; 32]) -> [u8; 52] {
	let mut key = [0u8; 52];
	key[..20].copy_from_slice(address);
	key[20..].copy_from_slice(slot_hash);
	key
}

/// Verify existence of a key-value pair in the trie.
pub fn verify_pharos_proof(
	proof_nodes: &[PharosProofNode],
	key: &[u8],
	value: &[u8],
	root: &[u8; 32],
) -> bool {
	let Some(last) = proof_nodes.last() else { return false };

	if !is_leaf(&last.proof_node) {
		return false;
	}

	if last.proof_node[1..33] != sha256(key) || last.proof_node[33..65] != sha256(value) {
		return false;
	}

	verify_proof_walk(proof_nodes, root)
}

pub fn verify_account_proof(
	proof_nodes: &[PharosProofNode],
	address: &[u8; 20],
	raw_value: &[u8],
	state_root: &[u8; 32],
) -> bool {
	verify_pharos_proof(proof_nodes, address, raw_value, state_root)
}

pub fn verify_storage_proof(
	proof_nodes: &[PharosProofNode],
	address: &[u8; 20],
	slot_hash: &[u8; 32],
	storage_value: &[u8; 32],
	storage_root: &[u8; 32],
) -> bool {
	let key = build_storage_key(address, slot_hash);
	verify_pharos_proof(proof_nodes, &key, storage_value, storage_root)
}

/// Verify key membership without the raw value. Returns `Some(value_hash)` from the leaf.
pub fn verify_pharos_proof_membership(
	proof_nodes: &[PharosProofNode],
	key: &[u8],
	root: &[u8; 32],
) -> Option<[u8; 32]> {
	let Some(last) = proof_nodes.last() else { return None };

	if !is_leaf(&last.proof_node) || last.proof_node[1..33] != sha256(key) {
		return None;
	}

	let mut value_hash = [0u8; 32];
	value_hash.copy_from_slice(&last.proof_node[33..65]);

	verify_proof_walk(proof_nodes, root).then_some(value_hash)
}

pub fn verify_storage_membership_proof(
	proof_nodes: &[PharosProofNode],
	address: &[u8; 20],
	slot_hash: &[u8; 32],
	storage_root: &[u8; 32],
) -> Option<[u8; 32]> {
	let key = build_storage_key(address, slot_hash);
	verify_pharos_proof_membership(proof_nodes, &key, storage_root)
}

fn is_existence_proof(proof_nodes: &[PharosProofNode], key: &[u8]) -> bool {
	match proof_nodes.last() {
		Some(last) => is_leaf(&last.proof_node) && last.proof_node[1..33] == sha256(key),
		None => false,
	}
}

/// Verify that `key` does NOT exist in the trie.
///
/// Case 1: Proof ends at a leaf with a different key_hash (path collision).
/// Case 2: Proof ends at an internal node where the target nibble slot is empty.
///         Sibling proofs pin the non-empty slots to the same root, preventing forgery.
pub fn verify_non_existence_proof(
	proof_nodes: &[PharosProofNode],
	key: &[u8],
	root: &[u8; 32],
	sibling_proofs: &[SiblingLeftmostLeafProof],
) -> bool {
	let Some(last_node) = proof_nodes.last() else { return false };
	let last = &last_node.proof_node;
	let key_hash = sha256(key);

	// Case 1: leaf with different key
	if is_leaf(last) {
		if last[1..33] == key_hash {
			return false; // key matches, this is an existence proof
		}
		return verify_proof_walk(proof_nodes, root);
	}

	// Case 2: internal node with empty target slot
	if last.len() != INTERNAL_NODE_LEN {
		return false;
	}

	let depth = proof_nodes.len().saturating_sub(2);
	let nibble = nibble_at_depth(&key_hash, depth) as usize;
	let slot_start = INTERNAL_NODE_HEADER + nibble * INTERNAL_NODE_SLOT_SIZE;

	if slot_start + INTERNAL_NODE_SLOT_SIZE > last.len() ||
		!is_zero_slot(&last[slot_start..slot_start + INTERNAL_NODE_SLOT_SIZE])
	{
		return false;
	}

	if !verify_proof_walk(proof_nodes, root) {
		return false;
	}

	// Each sibling proves a non-empty slot is genuine by walking to a real leaf
	let parent_nodes = &proof_nodes[..proof_nodes.len() - 1];
	for sib in sibling_proofs {
		if sib.proof_path.is_empty() {
			continue;
		}

		let mut combined: Vec<PharosProofNode> = parent_nodes.to_vec();
		combined.extend_from_slice(&sib.proof_path);

		if !verify_proof_walk(&combined, root) ||
			!is_existence_proof(&combined, &sib.leftmost_leaf_key)
		{
			return false;
		}
	}

	true
}

pub fn verify_storage_non_existence_proof(
	proof_nodes: &[PharosProofNode],
	address: &[u8; 20],
	slot_hash: &[u8; 32],
	storage_root: &[u8; 32],
	sibling_proofs: &[SiblingLeftmostLeafProof],
) -> bool {
	let key = build_storage_key(address, slot_hash);
	verify_non_existence_proof(proof_nodes, &key, storage_root, sibling_proofs)
}
