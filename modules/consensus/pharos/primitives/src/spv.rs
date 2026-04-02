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

use alloc::vec::Vec;

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
/// Uses `nextBeginOffset` from each node to locate the child hash slot.
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

/// Key-aware bottom-up hash chain walk. Uses nibble_at_depth to locate child slots
/// in internal nodes instead of relying on nextBeginOffset. Required for sibling
/// proof verification where the offsets in the main chain don't apply to the
/// sibling's key path.
fn verify_proof_walk_with_key(
	proof_nodes: &[PharosProofNode],
	key: &[u8],
	root: &[u8; 32],
) -> bool {
	let Some(last) = proof_nodes.last() else { return false };
	let Some(mut current_hash) = compute_node_hash(&last.proof_node) else { return false };
	let key_hash = sha256(key);

	for i in (0..proof_nodes.len() - 1).rev() {
		let parent = &proof_nodes[i];

		// MSU root (index 0): use begin_offset; internal nodes: use nibble-based lookup
		let start = if i == 0 {
			parent.next_begin_offset as usize
		} else {
			let trie_depth = i - 1;
			let nibble = nibble_at_depth(&key_hash, trie_depth) as usize;
			INTERNAL_NODE_HEADER + nibble * INTERNAL_NODE_SLOT_SIZE
		};

		if start + INTERNAL_NODE_SLOT_SIZE > parent.proof_node.len() {
			return false;
		}

		if parent.proof_node[start..start + INTERNAL_NODE_SLOT_SIZE] != current_hash {
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

	// Count non-empty slots excluding the target.
	// When non-empty slots exist, every one must have a sibling proof to prevent the
	// SkipEmpty attack (moving a hash to a different slot without changing the node hash).
	// When the terminal node is entirely empty, the hash chain to root is sufficient,
	// the parent already commits to this empty subtree via the zero hash.
	let non_empty_count = (0..INTERNAL_NODE_SLOTS)
		.filter(|&i| {
			i != nibble && {
				let s = INTERNAL_NODE_HEADER + i * INTERNAL_NODE_SLOT_SIZE;
				!is_zero_slot(&last[s..s + INTERNAL_NODE_SLOT_SIZE])
			}
		})
		.count();

	if non_empty_count > 0 {
		if sibling_proofs.len() != non_empty_count {
			return false;
		}

		let parent_nodes = &proof_nodes[..proof_nodes.len() - 1];
		for sib in sibling_proofs {
			let idx = sib.slot_index as usize;
			if idx >= INTERNAL_NODE_SLOTS || idx == nibble {
				return false;
			}
			let s = INTERNAL_NODE_HEADER + idx * INTERNAL_NODE_SLOT_SIZE;
			if is_zero_slot(&last[s..s + INTERNAL_NODE_SLOT_SIZE]) {
				return false;
			}

			if sib.proof_path.is_empty() {
				return false;
			}

			let mut combined: Vec<PharosProofNode> = parent_nodes.to_vec();
			combined.extend_from_slice(&sib.proof_path);

			if !verify_proof_walk_with_key(&combined, &sib.leftmost_leaf_key, root) ||
				!is_existence_proof(&combined, &sib.leftmost_leaf_key)
			{
				return false;
			}
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

#[cfg(test)]
mod tests {
	use super::*;

	fn make_leaf(key: &[u8], value: &[u8]) -> Vec<u8> {
		let mut leaf = vec![LEAF_NODE_TYPE];
		leaf.extend_from_slice(&sha256(key));
		leaf.extend_from_slice(&sha256(value));
		leaf
	}

	fn make_internal_with_child(slot: usize, child_hash: &[u8; 32]) -> Vec<u8> {
		let mut node = vec![0u8; INTERNAL_NODE_LEN];
		let start = INTERNAL_NODE_HEADER + slot * INTERNAL_NODE_SLOT_SIZE;
		node[start..start + 32].copy_from_slice(child_hash);
		node
	}

	fn make_msu_root_with_child(slot: usize, child_hash: &[u8; 32]) -> Vec<u8> {
		let mut node = vec![0u8; MSU_ROOT_NODE_LEN];
		let start = slot * INTERNAL_NODE_SLOT_SIZE;
		node[start..start + 32].copy_from_slice(child_hash);
		node
	}

	fn node(data: impl Into<Vec<u8>>, begin: u32, end: u32) -> PharosProofNode {
		let data = data.into();
		PharosProofNode { proof_node: data, next_begin_offset: begin, next_end_offset: end }
	}

	#[test]
	fn test_hash_internal_node_skip_empty() {
		// All-zero node hashes to zero
		let empty = vec![0u8; INTERNAL_NODE_LEN];
		assert_eq!(hash_internal_node(&empty), ZERO_HASH);

		// Node with one child: hash = sha256(header || child_hash)
		let child_hash = sha256(b"test");
		let node = make_internal_with_child(5, &child_hash);
		let mut expected_input = vec![0u8; INTERNAL_NODE_HEADER];
		expected_input.extend_from_slice(&child_hash);
		assert_eq!(hash_internal_node(&node), sha256(&expected_input));

		// SkipEmpty: moving a hash to a different slot produces a DIFFERENT result
		// only if we track position (we don't — but the sibling proofs catch this)
		let node_moved = make_internal_with_child(10, &child_hash);
		// Both nodes have the same SkipEmpty hash since it's just sha256(header || child_hash)
		assert_eq!(hash_internal_node(&node), hash_internal_node(&node_moved));
		// This proves why sibling proofs are necessary for non-existence!
	}

	#[test]
	fn test_existence_proof_valid() {
		let key = b"test_key";
		let value = b"test_value";

		let leaf_data = make_leaf(key, value);
		let leaf_hash = sha256(&leaf_data);

		// Internal node with leaf at slot 3
		let internal = make_internal_with_child(3, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);
		let internal_offset = (INTERNAL_NODE_HEADER + 3 * INTERNAL_NODE_SLOT_SIZE) as u32;

		// MSU root with internal at slot 7
		let msu_root = make_msu_root_with_child(7, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (7 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(internal, internal_offset, internal_offset + 32),
			node(leaf_data, 0, 0),
		];

		assert!(verify_pharos_proof(&proof, key, value, &root));
	}

	#[test]
	fn test_existence_proof_wrong_value_rejected() {
		let key = b"test_key";
		let value = b"test_value";

		let leaf_data = make_leaf(key, value);
		let leaf_hash = sha256(&leaf_data);

		let internal = make_internal_with_child(3, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);
		let internal_offset = (INTERNAL_NODE_HEADER + 3 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let msu_root = make_msu_root_with_child(7, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (7 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(internal, internal_offset, internal_offset + 32),
			node(leaf_data, 0, 0),
		];

		assert!(!verify_pharos_proof(&proof, key, b"wrong_value", &root));
		assert!(!verify_pharos_proof(&proof, b"wrong_key", value, &root));
	}

	#[test]
	fn test_membership_proof_returns_value_hash() {
		let key = b"test_key";
		let value = b"test_value";

		let leaf_data = make_leaf(key, value);
		let leaf_hash = sha256(&leaf_data);

		let internal = make_internal_with_child(3, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);
		let internal_offset = (INTERNAL_NODE_HEADER + 3 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let msu_root = make_msu_root_with_child(7, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (7 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(internal, internal_offset, internal_offset + 32),
			node(leaf_data, 0, 0),
		];

		let result = verify_pharos_proof_membership(&proof, key, &root);
		assert_eq!(result, Some(sha256(value)));

		// Wrong key returns None
		assert!(verify_pharos_proof_membership(&proof, b"wrong", &root).is_none());
	}

	#[test]
	fn test_non_existence_case1_leaf_mismatch() {
		// Proof ends at a leaf with a different key
		let other_key = b"other_key";
		let other_value = b"other_value";

		let leaf_data = make_leaf(other_key, other_value);
		let leaf_hash = sha256(&leaf_data);

		let internal = make_internal_with_child(3, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);
		let internal_offset = (INTERNAL_NODE_HEADER + 3 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let msu_root = make_msu_root_with_child(7, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (7 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(internal, internal_offset, internal_offset + 32),
			node(leaf_data, 0, 0),
		];

		// Non-existence for a different key succeeds
		assert!(verify_non_existence_proof(&proof, b"missing_key", &root, &[]));

		// Non-existence for the actual key fails (it exists!)
		assert!(!verify_non_existence_proof(&proof, other_key, &root, &[]));
	}

	#[test]
	fn test_non_existence_case2_empty_slot_all_zero_terminal() {
		// Terminal node is all zeros, no sibling proofs needed
		let empty_internal = vec![0u8; INTERNAL_NODE_LEN];
		let empty_hash = ZERO_HASH; // all-zero node hashes to zero

		let parent = make_internal_with_child(5, &empty_hash);
		let parent_hash = hash_internal_node(&parent);
		let parent_offset = (INTERNAL_NODE_HEADER + 5 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let msu_root = make_msu_root_with_child(2, &parent_hash);
		let root = sha256(&msu_root);
		let msu_offset = (2 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(parent, parent_offset, parent_offset + 32),
			node(empty_internal, 0, 0),
		];

		assert!(verify_non_existence_proof(&proof, b"any_key", &root, &[]));
	}

	#[test]
	fn test_non_existence_missing_sibling_rejected() {
		// Terminal node has a non-empty slot but no sibling proof provided
		let child_hash = sha256(b"some_child");
		let terminal = make_internal_with_child(5, &child_hash);
		let terminal_hash = hash_internal_node(&terminal);

		let parent_offset = (INTERNAL_NODE_HEADER + 3 * INTERNAL_NODE_SLOT_SIZE) as u32;
		let parent = make_internal_with_child(3, &terminal_hash);
		let parent_hash = hash_internal_node(&parent);

		let msu_root = make_msu_root_with_child(0, &parent_hash);
		let root = sha256(&msu_root);

		let proof = vec![
			node(msu_root, 0, 32),
			node(parent, parent_offset, parent_offset + 32),
			node(terminal, 0, 0),
		];

		// Terminal has 1 non-empty slot (slot 5) but 0 sibling proofs
		// This must fail, attacker could have moved a hash via SkipEmpty
		assert!(!verify_non_existence_proof(&proof, b"any_key", &root, &[]));
	}

	#[test]
	fn test_skip_empty_hash_slot_position_invariance() {
		// Demonstrates the SkipEmpty attack vector: moving a hash between slots
		// produces the same node hash, which is why sibling proofs are required
		let child_hash = sha256(b"data");
		let node_a = make_internal_with_child(3, &child_hash);
		let node_b = make_internal_with_child(11, &child_hash);

		// Same hash despite different slot positions
		assert_eq!(hash_internal_node(&node_a), hash_internal_node(&node_b));
	}

	#[test]
	fn test_empty_proof_rejected() {
		assert!(!verify_pharos_proof(&[], b"key", b"value", &[0; 32]));
		assert!(verify_pharos_proof_membership(&[], b"key", &[0; 32]).is_none());
		assert!(!verify_non_existence_proof(&[], b"key", &[0; 32], &[]));
	}

	#[test]
	fn test_nibble_at_depth() {
		let hash = [0xAB, 0xCD, 0xEF, 0x12]; // + more bytes
		let mut full_hash = [0u8; 32];
		full_hash[..4].copy_from_slice(&hash);

		// depth 0: low nibble of byte 0 = 0xB
		assert_eq!(nibble_at_depth(&full_hash, 0), 0x0B);
		// depth 1: high nibble of byte 0 = 0xA
		assert_eq!(nibble_at_depth(&full_hash, 1), 0x0A);
		// depth 2: low nibble of byte 1 = 0xD
		assert_eq!(nibble_at_depth(&full_hash, 2), 0x0D);
		// depth 3: high nibble of byte 1 = 0xC
		assert_eq!(nibble_at_depth(&full_hash, 3), 0x0C);
	}
}
