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

/// Errors returned by SPV proof verification.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Proof contains no nodes")]
	EmptyProof,
	#[error("Terminal proof node is not a valid leaf")]
	InvalidLeaf,
	#[error("Leaf key hash does not match the expected key")]
	KeyMismatch,
	#[error("Leaf value hash does not match the expected value")]
	ValueMismatch,
	#[error("Proof node has an unrecognized length")]
	InvalidNodeLength,
	#[error("Child hash in parent does not match computed child hash")]
	HashChainBroken,
	#[error("Slot offset is out of bounds for the parent node")]
	SlotOutOfBounds,
	#[error("Computed root hash does not match the expected root")]
	RootMismatch,
	#[error("Terminal node is not a valid internal node")]
	InvalidTerminalNode,
	#[error("Target nibble slot is not empty")]
	TargetSlotNotEmpty,
	#[error("Key exists in the trie")]
	KeyExists,
	#[error("Sibling proof count does not match non-empty slot count")]
	SiblingCountMismatch,
	#[error("Sibling proof references an invalid or disallowed slot index")]
	InvalidSiblingSlot,
	#[error("Duplicate sibling proof for the same slot")]
	DuplicateSiblingSlot,
	#[error("Sibling proof has an empty proof path")]
	EmptySiblingPath,
	#[error("Sibling leaf nibble does not route through declared slot index")]
	SiblingNibbleMismatch,
	#[error("Sibling proof failed verification: {0}")]
	SiblingProofInvalid(alloc::boxed::Box<Error>),
}

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
	slot == ZERO_HASH
}

/// SkipEmpty: `sha256(3-byte header || non-zero slots)`. All-zero node hashes to `[0; 32]`.
fn hash_internal_node(proof_node: &[u8]) -> [u8; 32] {
	let mut data = Vec::with_capacity(INTERNAL_NODE_LEN);
	data.extend_from_slice(&proof_node[..INTERNAL_NODE_HEADER]);

	for i in 0..INTERNAL_NODE_SLOTS {
		let start = INTERNAL_NODE_HEADER + i * INTERNAL_NODE_SLOT_SIZE;
		let slot = &proof_node[start..start + INTERNAL_NODE_SLOT_SIZE];
		if !is_zero_slot(slot) {
			data.extend_from_slice(slot);
		}
	}

	if data.len() == INTERNAL_NODE_HEADER {
		ZERO_HASH
	} else {
		sha256(&data)
	}
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
///
/// Uses `nibble_at_depth(sha256(key))` to locate child slots in internal nodes
/// (index > 0), ensuring the proof path follows the key's trie path. The MSU root
/// (index 0) uses `next_begin_offset` because its 256-slot addressing scheme is
/// Pharos-specific and opaque; the MSU root content is pinned to the state root
/// via its hash, so an attacker cannot substitute a different MSU root.
fn verify_proof_walk(
	proof_nodes: &[PharosProofNode],
	key: &[u8],
	root: &[u8; 32],
) -> Result<(), Error> {
	let last = proof_nodes.last().ok_or(Error::EmptyProof)?;
	let mut current_hash = compute_node_hash(&last.proof_node).ok_or(Error::InvalidNodeLength)?;
	let key_hash = sha256(key);

	for i in (0..proof_nodes.len()).rev().skip(1) {
		let parent = &proof_nodes[i];

		let start = if i == 0 {
			parent.next_begin_offset as usize
		} else {
			let trie_depth = i - 1;
			let nibble = nibble_at_depth(&key_hash, trie_depth) as usize;
			INTERNAL_NODE_HEADER + nibble * INTERNAL_NODE_SLOT_SIZE
		};

		let slot = parent
			.proof_node
			.get(start..start + INTERNAL_NODE_SLOT_SIZE)
			.ok_or(Error::SlotOutOfBounds)?;

		if slot != current_hash {
			return Err(Error::HashChainBroken);
		}

		current_hash = compute_node_hash(&parent.proof_node).ok_or(Error::InvalidNodeLength)?;
	}

	if current_hash == *root {
		Ok(())
	} else {
		Err(Error::RootMismatch)
	}
}

/// Builds a storage trie key by concatenating address and slot hash.
pub fn build_storage_key(address: &[u8; 20], slot_hash: &[u8; 32]) -> [u8; 52] {
	let mut key = [0u8; 52];
	key[..20].copy_from_slice(address);
	key[20..].copy_from_slice(slot_hash);
	key
}

/// Verify that a key-value pair exists in the trie.
pub fn verify_proof(
	proof_nodes: &[PharosProofNode],
	key: &[u8],
	value: &[u8],
	root: &[u8; 32],
) -> Result<(), Error> {
	let last = proof_nodes.last().ok_or(Error::EmptyProof)?;

	if !is_leaf(&last.proof_node) {
		return Err(Error::InvalidLeaf);
	}

	if last.proof_node[1..33] != sha256(key) {
		return Err(Error::KeyMismatch);
	}

	if last.proof_node[33..65] != sha256(value) {
		return Err(Error::ValueMismatch);
	}

	verify_proof_walk(proof_nodes, key, root)
}

/// Verify that a key exists in the trie (inclusion proof).
/// Returns the value hash from the leaf on success.
pub fn verify_membership_proof(
	proof_nodes: &[PharosProofNode],
	key: &[u8],
	root: &[u8; 32],
) -> Result<[u8; 32], Error> {
	let last = proof_nodes.last().ok_or(Error::EmptyProof)?;

	if !is_leaf(&last.proof_node) {
		return Err(Error::InvalidLeaf);
	}

	if last.proof_node[1..33] != sha256(key) {
		return Err(Error::KeyMismatch);
	}

	let mut value_hash = [0u8; 32];
	value_hash.copy_from_slice(&last.proof_node[33..65]);

	verify_proof_walk(proof_nodes, key, root)?;
	Ok(value_hash)
}

/// Verify that a key does NOT exist in the trie (non-inclusion proof).
///
/// Case 1: Proof ends at a leaf with a different key_hash (path collision).
/// Case 2: Proof ends at an internal node where the target nibble slot is empty.
///         Sibling proofs pin the non-empty slots to the same root, preventing forgery.
pub fn verify_non_existence_proof(
	proof_nodes: &[PharosProofNode],
	key: &[u8],
	root: &[u8; 32],
	sibling_proofs: &[SiblingLeftmostLeafProof],
) -> Result<(), Error> {
	let last_node = proof_nodes.last().ok_or(Error::EmptyProof)?;
	let last = &last_node.proof_node;
	let key_hash = sha256(key);

	// Case 1: leaf with different key
	if is_leaf(last) {
		if last[1..33] == key_hash {
			return Err(Error::KeyExists);
		}
		return verify_proof_walk(proof_nodes, key, root);
	}

	// Case 2: internal node with empty target slot
	if last.len() != INTERNAL_NODE_LEN {
		return Err(Error::InvalidTerminalNode);
	}

	let depth = proof_nodes.len().saturating_sub(2);
	let nibble = nibble_at_depth(&key_hash, depth) as usize;
	let slot_start = INTERNAL_NODE_HEADER + nibble * INTERNAL_NODE_SLOT_SIZE;

	if slot_start + INTERNAL_NODE_SLOT_SIZE > last.len() {
		return Err(Error::SlotOutOfBounds);
	}

	if !is_zero_slot(&last[slot_start..slot_start + INTERNAL_NODE_SLOT_SIZE]) {
		return Err(Error::TargetSlotNotEmpty);
	}

	verify_proof_walk(proof_nodes, key, root)?;

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
			return Err(Error::SiblingCountMismatch);
		}

		let parent_nodes = &proof_nodes[..proof_nodes.len() - 1];
		let mut proven_slots = [false; INTERNAL_NODE_SLOTS];

		for sib in sibling_proofs {
			let idx = sib.slot_index as usize;
			if idx >= INTERNAL_NODE_SLOTS || idx == nibble {
				return Err(Error::InvalidSiblingSlot);
			}
			let s = INTERNAL_NODE_HEADER + idx * INTERNAL_NODE_SLOT_SIZE;
			if is_zero_slot(&last[s..s + INTERNAL_NODE_SLOT_SIZE]) {
				return Err(Error::InvalidSiblingSlot);
			}

			if proven_slots[idx] {
				return Err(Error::DuplicateSiblingSlot);
			}
			proven_slots[idx] = true;

			if sib.proof_path.is_empty() {
				return Err(Error::EmptySiblingPath);
			}

			let sib_key_hash = sha256(&sib.leftmost_leaf_key);
			if nibble_at_depth(&sib_key_hash, depth) as usize != idx {
				return Err(Error::SiblingNibbleMismatch);
			}

			let mut combined: Vec<PharosProofNode> = parent_nodes.to_vec();
			combined.extend_from_slice(&sib.proof_path);

			let is_valid_leaf = combined.last().map_or(false, |last| {
				is_leaf(&last.proof_node) &&
					last.proof_node[1..33] == sha256(&sib.leftmost_leaf_key)
			});

			if !is_valid_leaf {
				return Err(Error::SiblingProofInvalid(alloc::boxed::Box::new(Error::InvalidLeaf)));
			}

			verify_proof_walk(&combined, &sib.leftmost_leaf_key, root)
				.map_err(|e| Error::SiblingProofInvalid(alloc::boxed::Box::new(e)))?;
		}
	}

	Ok(())
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

	/// Build a 3-node proof (MSU root → internal → leaf) that follows the
	/// key's nibble path. The internal node slot is derived from the key.
	fn build_proof_for_key(key: &[u8], value: &[u8]) -> (Vec<PharosProofNode>, [u8; 32]) {
		let leaf_data = make_leaf(key, value);
		let leaf_hash = sha256(&leaf_data);

		// Internal node slot must match key's nibble at depth 0
		let key_hash = sha256(key);
		let nibble = nibble_at_depth(&key_hash, 0) as usize;
		let internal = make_internal_with_child(nibble, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);

		// MSU root — slot is arbitrary (uses next_begin_offset)
		let msu_root = make_msu_root_with_child(7, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (7 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(internal, 0, 0), // offsets unused for i > 0
			node(leaf_data, 0, 0),
		];
		(proof, root)
	}

	#[test]
	fn test_existence_proof_valid() {
		let key = b"test_key";
		let value = b"test_value";
		let (proof, root) = build_proof_for_key(key, value);
		assert!(verify_proof(&proof, key, value, &root).is_ok());
	}

	#[test]
	fn test_existence_proof_wrong_value_rejected() {
		let key = b"test_key";
		let value = b"test_value";
		let (proof, root) = build_proof_for_key(key, value);

		assert!(verify_proof(&proof, key, b"wrong_value", &root).is_err());
		assert!(verify_proof(&proof, b"wrong_key", value, &root).is_err());
	}

	#[test]
	fn test_membership_proof_returns_value_hash() {
		let key = b"test_key";
		let value = b"test_value";
		let (proof, root) = build_proof_for_key(key, value);

		let result = verify_membership_proof(&proof, key, &root);
		assert_eq!(result.unwrap(), sha256(value));

		// Wrong key returns error
		assert!(verify_membership_proof(&proof, b"wrong", &root).is_err());
	}

	#[test]
	fn test_non_existence_case1_leaf_mismatch() {
		// Proof ends at a leaf with a different key. For Case 1 to be valid,
		// the query key and the leaf key must share the same nibble path through
		// the trie (they collide at the leaf level but have different key_hashes).
		let other_key = b"other_key";
		let other_value = b"other_value";
		let query_key = b"missing_key";

		let leaf_data = make_leaf(other_key, other_value);
		let leaf_hash = sha256(&leaf_data);

		// Place the leaf at the slot matching the QUERY key's nibble at depth 0.
		let query_key_hash = sha256(query_key);
		let query_nibble = nibble_at_depth(&query_key_hash, 0) as usize;

		let internal = make_internal_with_child(query_nibble, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);

		let msu_root = make_msu_root_with_child(7, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (7 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(internal, 0, 0), // offsets unused by key-aware walk for i > 0
			node(leaf_data, 0, 0),
		];

		// Non-existence for query key succeeds (leaf has different key_hash)
		assert!(verify_non_existence_proof(&proof, query_key, &root, &[]).is_ok());

		// Non-existence for the actual key fails (it exists!)
		assert!(verify_non_existence_proof(&proof, other_key, &root, &[]).is_err());
	}

	#[test]
	fn test_non_existence_case1_wrong_path_rejected() {
		// A leaf exists in the trie, but the proof path does NOT follow the
		// query key's nibbles at the internal node level. This should be rejected.
		let other_key = b"other_key";
		let other_value = b"other_value";
		let query_key = b"missing_key";

		let leaf_data = make_leaf(other_key, other_value);
		let leaf_hash = sha256(&leaf_data);

		// Place the leaf at a DIFFERENT internal-node slot than the query key's nibble.
		let query_key_hash = sha256(query_key);
		let query_nibble = nibble_at_depth(&query_key_hash, 0) as usize;
		let wrong_slot = (query_nibble + 1) % INTERNAL_NODE_SLOTS;

		let internal = make_internal_with_child(wrong_slot, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);
		let wrong_offset = (INTERNAL_NODE_HEADER + wrong_slot * INTERNAL_NODE_SLOT_SIZE) as u32;

		let msu_root = make_msu_root_with_child(7, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (7 * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(internal, wrong_offset, wrong_offset + 32),
			node(leaf_data, 0, 0),
		];

		// Rejected: the internal node has the leaf at the wrong nibble slot
		assert!(verify_non_existence_proof(&proof, query_key, &root, &[]).is_err());
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

		assert!(verify_non_existence_proof(&proof, b"any_key", &root, &[]).is_ok());
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
		assert!(verify_non_existence_proof(&proof, b"any_key", &root, &[]).is_err());
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
		assert!(verify_proof(&[], b"key", b"value", &[0; 32]).is_err());
		assert!(verify_membership_proof(&[], b"key", &[0; 32]).is_err());
		assert!(verify_non_existence_proof(&[], b"key", &[0; 32], &[]).is_err());
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
