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

use alloc::{collections::BTreeSet, vec::Vec};

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
	#[error("MSU root slot offset does not match the offset derived from the key")]
	MsuOffsetMismatch,
	#[error("Key is empty, cannot derive the MSU root slot offset")]
	EmptyKey,
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
	#[error("Proof exceeds maximum allowed depth")]
	ProofTooDeep,
	#[error("Terminal leaf is not bound to the queried key's trie path")]
	UnboundTerminalLeaf,
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

// Max legitimate proof length for a SHA-256 hexary trie: 64 nibbles of trie
// depth (one per hash byte nibble) plus the MSU root. Anything beyond this
// cannot correspond to a real trie path and is rejected to bound verifier
// work and prevent adversarial proofs from driving `nibble_at_depth` past
// the end of the 32-byte key hash.
pub const MAX_PROOF_DEPTH: usize = 65;

pub fn sha256(data: &[u8]) -> [u8; 32] {
	sp_io::hashing::sha2_256(data)
}

/// Pharos nibble extraction: low nibble first at even depths, high nibble at odd depths.
///
/// Returns `None` if `depth` is beyond the nibbles addressable by `key_hash`
/// (i.e. `depth >= key_hash.len() * 2`). Entry points additionally enforce
/// `MAX_PROOF_DEPTH`, so this `None` case should be unreachable in practice,
/// but surfacing it as an `Option` keeps adversarial callers from panicking.
pub fn nibble_at_depth(key_hash: &[u8], depth: usize) -> Option<u8> {
	let byte_index = depth / 2;
	let byte = *key_hash.get(byte_index)?;
	if depth % 2 == 0 {
		Some(byte & 0x0F)
	} else {
		Some((byte >> 4) & 0x0F)
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
/// Internal nodes (index > 0) locate their child slot with `nibble_at_depth(sha256(key))`,
/// so the path follows the key's trie route. The MSU root (index 0) is addressed by the
/// key's last byte: it holds 256 slots of 32 bytes each, so the slot offset is
/// `key[last] * 32`. We derive that offset here and reject the proof when the prover's
/// `next_begin_offset` disagrees, rather than trusting it. Trusting the prover at this
/// layer would let it aim the proof at an unrelated MSU subtree.
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
			let msu_slot = *key.last().ok_or(Error::EmptyKey)? as usize;
			let expected_offset = msu_slot * INTERNAL_NODE_SLOT_SIZE;
			if parent.next_begin_offset as usize != expected_offset {
				return Err(Error::MsuOffsetMismatch);
			}
			expected_offset
		} else {
			let trie_depth = i - 1;
			let nibble =
				nibble_at_depth(&key_hash, trie_depth).ok_or(Error::ProofTooDeep)? as usize;
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
	if proof_nodes.len() > MAX_PROOF_DEPTH {
		return Err(Error::ProofTooDeep);
	}

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
	if proof_nodes.len() > MAX_PROOF_DEPTH {
		return Err(Error::ProofTooDeep);
	}

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
/// Case 1: proof ends at a leaf with a different key hash (path collision).
/// Case 2: proof ends at an internal node where the queried slot is empty. When the
///         terminal has siblings they are verified directly; when it is all-zero the
///         sibling evidence is taken from the deepest ancestor with non-empty non-path slots.
pub fn verify_non_existence_proof(
	proof_nodes: &[PharosProofNode],
	key: &[u8],
	root: &[u8; 32],
	sibling_proofs: &[SiblingLeftmostLeafProof],
) -> Result<(), Error> {
	if proof_nodes.len() > MAX_PROOF_DEPTH {
		return Err(Error::ProofTooDeep);
	}

	let last_node = proof_nodes.last().ok_or(Error::EmptyProof)?;
	let last = &last_node.proof_node;
	let key_hash = sha256(key);

	// Case 1: leaf with different key
	if is_leaf(last) {
		let leaf_key_hash = &last[1..33];
		if leaf_key_hash == key_hash {
			return Err(Error::KeyExists);
		}

		// The terminal leaf must be the genuine occupant of the queried key's
		// trie path: its key hash must share the queried key's nibble prefix
		// for every internal node on the path. `verify_proof_walk` routes the
		// path by the queried key's nibbles, but `hash_internal_node` is
		// SkipEmpty — for a single-child node the hash is independent of which
		// slot holds the child. Without this binding an attacker could relabel
		// any unrelated leaf's inclusion path onto the queried key's nibbles
		// (hash-preserving) and forge a non-existence proof for a key that
		// genuinely exists.
		let internal_count = proof_nodes.len().saturating_sub(2);
		if internal_count == 0 {
			return Err(Error::UnboundTerminalLeaf);
		}
		for depth in 0..internal_count {
			let key_nibble = nibble_at_depth(&key_hash, depth).ok_or(Error::ProofTooDeep)?;
			let leaf_nibble = nibble_at_depth(leaf_key_hash, depth).ok_or(Error::ProofTooDeep)?;
			if key_nibble != leaf_nibble {
				return Err(Error::UnboundTerminalLeaf);
			}
		}

		return verify_proof_walk(proof_nodes, key, root);
	}

	// Case 2: internal node with empty target slot
	if last.len() != INTERNAL_NODE_LEN {
		return Err(Error::InvalidTerminalNode);
	}

	let depth = proof_nodes.len().saturating_sub(2);
	let nibble = nibble_at_depth(&key_hash, depth).ok_or(Error::ProofTooDeep)? as usize;
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

	// When the terminal is all-zero it only marks the end of an empty path; the actual
	// sibling evidence lives in the deepest ancestor that has non-empty non-path slots.
	//
	// The two branches differ in parent_end by design: for a non-empty terminal the
	// anchor is excluded and must reappear as sib.proof_path[0] (re-routed via the
	// sibling slot); for an all-zero terminal the anchor is an ancestor already
	// committed in parent_nodes, so sib.proof_path begins at the sibling slot's child.
	let (anchor, anchor_depth, anchor_queried_nibble, anchor_non_empty_count, parent_end) =
		if non_empty_count > 0 {
			(last as &[u8], depth, nibble, non_empty_count, proof_nodes.len() - 1)
		} else {
			let anchor_idx = (1..proof_nodes.len().saturating_sub(1)).rev().find(|&ni| {
				let node = &proof_nodes[ni].proof_node;
				if node.len() != INTERNAL_NODE_LEN {
					return false;
				}
				let d = ni - 1;
				let q = match nibble_at_depth(&key_hash, d) {
					Some(n) => n as usize,
					None => return false,
				};
				(0..INTERNAL_NODE_SLOTS).any(|i| {
					if i == q {
						return false;
					}
					let s = INTERNAL_NODE_HEADER + i * INTERNAL_NODE_SLOT_SIZE;
					!is_zero_slot(&node[s..s + INTERNAL_NODE_SLOT_SIZE])
				})
			});

			let Some(ni) = anchor_idx else {
				if !sibling_proofs.is_empty() {
					return Err(Error::SiblingCountMismatch);
				}
				return Ok(());
			};

			let d = ni - 1;
			let q = nibble_at_depth(&key_hash, d).ok_or(Error::ProofTooDeep)? as usize;
			let cnt = (0..INTERNAL_NODE_SLOTS)
				.filter(|&i| {
					if i == q {
						return false;
					}
					let s = INTERNAL_NODE_HEADER + i * INTERNAL_NODE_SLOT_SIZE;
					!is_zero_slot(&proof_nodes[ni].proof_node[s..s + INTERNAL_NODE_SLOT_SIZE])
				})
				.count();
			(&proof_nodes[ni].proof_node[..], d, q, cnt, ni + 1)
		};

	if sibling_proofs.len() != anchor_non_empty_count {
		return Err(Error::SiblingCountMismatch);
	}

	let parent_nodes = &proof_nodes[..parent_end];
	let mut seen: BTreeSet<usize> = BTreeSet::new();

	for sib in sibling_proofs {
		let idx = sib.slot_index as usize;
		if idx >= INTERNAL_NODE_SLOTS || idx == anchor_queried_nibble {
			return Err(Error::InvalidSiblingSlot);
		}
		let s = INTERNAL_NODE_HEADER + idx * INTERNAL_NODE_SLOT_SIZE;
		if is_zero_slot(&anchor[s..s + INTERNAL_NODE_SLOT_SIZE]) {
			return Err(Error::InvalidSiblingSlot);
		}

		if !seen.insert(idx) {
			return Err(Error::DuplicateSiblingSlot);
		}

		if sib.proof_path.is_empty() {
			return Err(Error::EmptySiblingPath);
		}

		let sib_key_hash = sha256(&sib.leftmost_leaf_key);
		if nibble_at_depth(&sib_key_hash, anchor_depth).ok_or(Error::ProofTooDeep)? as usize != idx
		{
			return Err(Error::SiblingNibbleMismatch);
		}

		let mut combined: Vec<PharosProofNode> = parent_nodes.to_vec();
		combined.extend_from_slice(&sib.proof_path);

		let is_valid_leaf = combined.last().map_or(false, |last| {
			is_leaf(&last.proof_node) && last.proof_node[1..33] == sha256(&sib.leftmost_leaf_key)
		});

		if !is_valid_leaf {
			return Err(Error::SiblingProofInvalid(alloc::boxed::Box::new(Error::InvalidLeaf)));
		}

		verify_proof_walk(&combined, &sib.leftmost_leaf_key, root)
			.map_err(|e| Error::SiblingProofInvalid(alloc::boxed::Box::new(e)))?;
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

	/// Build a 3-node proof (MSU root -> internal -> leaf) that follows the key's
	/// trie path. The internal node slot comes from the key's nibble at depth 0 and
	/// the MSU root slot from the key's last byte.
	fn build_proof_for_key(key: &[u8], value: &[u8]) -> (Vec<PharosProofNode>, [u8; 32]) {
		let leaf_data = make_leaf(key, value);
		let leaf_hash = sha256(&leaf_data);

		let key_hash = sha256(key);
		let nibble = nibble_at_depth(&key_hash, 0).unwrap() as usize;
		let internal = make_internal_with_child(nibble, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);

		let msu_slot = *key.last().unwrap() as usize;
		let msu_root = make_msu_root_with_child(msu_slot, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (msu_slot * INTERNAL_NODE_SLOT_SIZE) as u32;

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
	fn test_msu_offset_derived_from_key_not_prover() {
		// The MSU root slot offset comes from the key's last byte. A proof whose
		// child sits at the right slot but whose prover-supplied offset points
		// elsewhere is rejected, so the prover can no longer aim the walk at an
		// unrelated MSU subtree.
		let key = b"test_key";
		let value = b"test_value";
		let (mut proof, root) = build_proof_for_key(key, value);

		assert!(verify_proof(&proof, key, value, &root).is_ok());

		let bad_slot = (*key.last().unwrap() as usize + 1) % 256;
		proof[0].next_begin_offset = (bad_slot * INTERNAL_NODE_SLOT_SIZE) as u32;
		assert!(matches!(verify_proof(&proof, key, value, &root), Err(Error::MsuOffsetMismatch)));
	}

	#[test]
	fn test_non_existence_case1_leaf_mismatch() {
		// Proof ends at a leaf with a different key. For Case 1 to be valid,
		// the query key and the leaf key must share the same nibble path through
		// the trie (they collide at the leaf level but have different key_hashes).
		let query_key = b"missing_key";
		let query_key_hash = sha256(query_key);
		let query_nibble = nibble_at_depth(&query_key_hash, 0).unwrap();
		let msu_slot = *query_key.last().unwrap() as usize;

		// The terminal leaf belongs to a different key, but it must genuinely
		// occupy the query key's trie path: it shares the query key's last byte
		// (same MSU slot) and collides on nibble 0 (same internal slot).
		let other_value = b"other_value";
		let (other_key, _) = (0u32..)
			.map(|i| {
				let mut k = b"collide_".to_vec();
				k.extend_from_slice(&i.to_le_bytes());
				k.push(query_key[query_key.len() - 1]);
				let h = sha256(&k);
				(k, h)
			})
			.find(|(_, h)| nibble_at_depth(h, 0).unwrap() == query_nibble && *h != query_key_hash)
			.unwrap();

		let leaf_data = make_leaf(&other_key, other_value);
		let leaf_hash = sha256(&leaf_data);

		// Place the leaf at the slot matching the QUERY key's nibble at depth 0.
		let internal = make_internal_with_child(query_nibble as usize, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);

		let msu_root = make_msu_root_with_child(msu_slot, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (msu_slot * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(internal, 0, 0), // offsets unused by key-aware walk for i > 0
			node(leaf_data, 0, 0),
		];

		// Non-existence for query key succeeds (leaf has different key_hash)
		assert!(verify_non_existence_proof(&proof, query_key, &root, &[]).is_ok());

		// Non-existence for the actual key fails (it exists!)
		assert!(verify_non_existence_proof(&proof, &other_key, &root, &[]).is_err());
	}

	#[test]
	fn test_non_existence_case1_forged_relabel_rejected() {
		// SkipEmpty hashing makes a single-child internal node's hash independent
		// of which slot holds the child. An attacker can take the genuine inclusion
		// path of an unrelated leaf and relabel it onto the queried key's nibble
		// without changing any hash, forging a non-existence proof. The terminal
		// leaf binding check is what rejects it.
		//
		// The unrelated leaf shares the queried key's last byte, so the forged proof
		// clears the MSU offset check and reaches the binding check, which then
		// catches the divergent internal nibble.
		let query_key = b"genuine_key_K";
		let query_hash = sha256(query_key);
		let query_nibble0 = nibble_at_depth(&query_hash, 0).unwrap();
		let msu_slot = *query_key.last().unwrap() as usize;

		let (other_key, other_hash) = (0u32..)
			.map(|i| {
				let mut k = b"unrelated_".to_vec();
				k.extend_from_slice(&i.to_le_bytes());
				k.push(query_key[query_key.len() - 1]);
				let h = sha256(&k);
				(k, h)
			})
			.find(|(_, h)| nibble_at_depth(h, 0).unwrap() != query_nibble0)
			.unwrap();
		let other_value = b"other_value";
		let other_nibble0 = nibble_at_depth(&other_hash, 0).unwrap() as usize;

		// Genuine single-child path for the unrelated leaf.
		let leaf = make_leaf(&other_key, other_value);
		let leaf_hash = sha256(&leaf);
		let internal = make_internal_with_child(other_nibble0, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);

		let msu_root = make_msu_root_with_child(msu_slot, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (msu_slot * INTERNAL_NODE_SLOT_SIZE) as u32;

		// Relabel the single-child node onto the queried key's nibble. SkipEmpty
		// leaves the hash unchanged, so the chain still validates against the root.
		let forged_internal = make_internal_with_child(query_nibble0 as usize, &leaf_hash);
		assert_eq!(hash_internal_node(&forged_internal), internal_hash);

		let forged_proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(forged_internal, 0, 0),
			node(leaf, 0, 0),
		];

		// The hash chain still validates against the trusted root; this is the
		// SkipEmpty structural weakness the binding check defends.
		assert!(verify_proof_walk(&forged_proof, query_key, &root).is_ok());

		// Non-existence verification rejects it: the terminal leaf is not bound to
		// the queried key's trie path.
		assert!(matches!(
			verify_non_existence_proof(&forged_proof, query_key, &root, &[]),
			Err(Error::UnboundTerminalLeaf)
		));
	}

	#[test]
	fn test_non_existence_case1_no_internal_node_rejected() {
		// A Case 1 proof with no internal node ([MSU root, leaf]) cannot bind
		// the terminal leaf to the queried key's path and must be rejected.
		let query_key = b"queried_key";
		let msu_slot = *query_key.last().unwrap() as usize;
		let leaf_data = make_leaf(b"some_other_key", b"v");
		let leaf_hash = sha256(&leaf_data);
		let msu_root = make_msu_root_with_child(msu_slot, &leaf_hash);
		let root = sha256(&msu_root);
		let msu_offset = (msu_slot * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![node(msu_root, msu_offset, msu_offset + 32), node(leaf_data, 0, 0)];

		assert!(matches!(
			verify_non_existence_proof(&proof, query_key, &root, &[]),
			Err(Error::UnboundTerminalLeaf)
		));
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
		let query_nibble = nibble_at_depth(&query_key_hash, 0).unwrap() as usize;
		let wrong_slot = (query_nibble + 1) % INTERNAL_NODE_SLOTS;

		let internal = make_internal_with_child(wrong_slot, &leaf_hash);
		let internal_hash = hash_internal_node(&internal);

		let msu_slot = *query_key.last().unwrap() as usize;
		let msu_root = make_msu_root_with_child(msu_slot, &internal_hash);
		let root = sha256(&msu_root);
		let msu_offset = (msu_slot * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(internal, 0, 0),
			node(leaf_data, 0, 0),
		];

		// Rejected: the internal node has the leaf at the wrong nibble slot
		assert!(verify_non_existence_proof(&proof, query_key, &root, &[]).is_err());
	}

	#[test]
	fn test_non_existence_case2_empty_slot_all_zero_terminal() {
		// An all-zero terminal hashes to zero, so its parent and the MSU root are
		// all-zero too. The target nibble slot is empty and no sibling proofs are needed.
		let query = b"any_key";
		let msu_slot = *query.last().unwrap() as usize;

		let empty_internal = vec![0u8; INTERNAL_NODE_LEN];
		let parent = vec![0u8; INTERNAL_NODE_LEN];
		let msu_root = vec![0u8; MSU_ROOT_NODE_LEN];
		let root = sha256(&msu_root);
		let msu_offset = (msu_slot * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(parent, 0, 0),
			node(empty_internal, 0, 0),
		];

		assert!(verify_non_existence_proof(&proof, query, &root, &[]).is_ok());
	}

	// Honest Pharos non-existence proof: all-zero terminal with a branching ancestor.
	// The anchor (deepest ancestor with non-empty non-path siblings) must be covered by
	// sibling proofs; the proof is rejected without them.
	#[test]
	fn test_non_existence_case2_all_zero_terminal_with_anchor_sibling() {
		let query_key = b"missing_key";
		let key_hash = sha256(query_key);
		let msu_slot = *query_key.last().unwrap() as usize;
		let queried_nibble = nibble_at_depth(&key_hash, 0).unwrap() as usize;
		let sibling_slot = (queried_nibble + 1) % INTERNAL_NODE_SLOTS;
		let query_last_byte = *query_key.last().unwrap();

		// Find a sibling key that routes through sibling_slot at depth 0 and shares the
		// same last byte as the query key (same MSU subtree, so the walk root check passes).
		let (sib_key, _) = (0u32..)
			.map(|i| {
				let mut k = b"sibling_".to_vec();
				k.extend_from_slice(&i.to_le_bytes());
				k.push(query_last_byte);
				let h = sha256(&k);
				(k, h)
			})
			.find(|(_, h)| nibble_at_depth(h, 0).unwrap() as usize == sibling_slot)
			.unwrap();

		let sib_leaf = make_leaf(&sib_key, b"v");
		let sib_leaf_hash = sha256(&sib_leaf);

		let mut anchor_data = vec![0u8; INTERNAL_NODE_LEN];
		let s = INTERNAL_NODE_HEADER + sibling_slot * INTERNAL_NODE_SLOT_SIZE;
		anchor_data[s..s + 32].copy_from_slice(&sib_leaf_hash);
		let anchor_hash = hash_internal_node(&anchor_data);

		let msu_root = make_msu_root_with_child(msu_slot, &anchor_hash);
		let root = sha256(&msu_root);
		let msu_offset = (msu_slot * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(anchor_data, 0, 0),
			node(vec![0u8; INTERNAL_NODE_LEN], 0, 0),
		];

		let sibling = SiblingLeftmostLeafProof {
			slot_index: sibling_slot as u8,
			leftmost_leaf_key: sib_key,
			proof_path: vec![node(sib_leaf, 0, 0)],
		};

		assert!(verify_non_existence_proof(&proof, query_key, &root, &[sibling]).is_ok());
		assert!(matches!(
			verify_non_existence_proof(&proof, query_key, &root, &[]),
			Err(Error::SiblingCountMismatch)
		));
	}

	// Moving the leaf to a sibling slot makes the parent the deepest anchor; it requires
	// a sibling proof that the attacker cannot supply for the moved slot.
	#[test]
	fn test_poc_empty_terminal_forgery_rejected() {
		let key = b"delivered_receipt_key";
		let value = b"receipt_exists";
		let (membership_proof, root) = build_proof_for_key(key, value);

		assert!(verify_membership_proof(&membership_proof, key, &root).is_ok());

		let leaf_data = &membership_proof[2].proof_node;
		let leaf_hash = sha256(leaf_data);
		let key_hash = sha256(key);
		let queried_slot = nibble_at_depth(&key_hash, 0).unwrap() as usize;
		let forged_slot = (queried_slot + 1) % INTERNAL_NODE_SLOTS;

		let forged_parent = make_internal_with_child(forged_slot, &leaf_hash);
		assert_eq!(
			hash_internal_node(&membership_proof[1].proof_node),
			hash_internal_node(&forged_parent)
		);

		let queried_slot_start = INTERNAL_NODE_HEADER + queried_slot * INTERNAL_NODE_SLOT_SIZE;
		assert_eq!(
			&forged_parent[queried_slot_start..queried_slot_start + INTERNAL_NODE_SLOT_SIZE],
			&ZERO_HASH
		);

		let forged_proof = vec![
			membership_proof[0].clone(),
			node(forged_parent, 0, 0),
			node(vec![0u8; INTERNAL_NODE_LEN], 0, 0),
		];

		assert!(verify_proof_walk(&forged_proof, key, &root).is_ok());
		assert!(matches!(
			verify_non_existence_proof(&forged_proof, key, &root, &[]),
			Err(Error::SiblingCountMismatch)
		));
	}

	#[test]
	fn test_non_existence_missing_sibling_rejected() {
		// Route the proof by the query key's nibbles so the terminal node is the
		// one the walk lands on. The terminal holds a child in a slot other than
		// the target, so a sibling proof is required to pin it.
		let query = b"any_key";
		let key_hash = sha256(query);
		let parent_slot = nibble_at_depth(&key_hash, 0).unwrap() as usize;
		let target_slot = nibble_at_depth(&key_hash, 1).unwrap() as usize;
		let sibling_slot = (target_slot + 1) % INTERNAL_NODE_SLOTS;

		let child_hash = sha256(b"some_child");
		let terminal = make_internal_with_child(sibling_slot, &child_hash);
		let terminal_hash = hash_internal_node(&terminal);

		let parent = make_internal_with_child(parent_slot, &terminal_hash);
		let parent_hash = hash_internal_node(&parent);

		let msu_slot = *query.last().unwrap() as usize;
		let msu_root = make_msu_root_with_child(msu_slot, &parent_hash);
		let root = sha256(&msu_root);
		let msu_offset = (msu_slot * INTERNAL_NODE_SLOT_SIZE) as u32;

		let proof = vec![
			node(msu_root, msu_offset, msu_offset + 32),
			node(parent, 0, 0),
			node(terminal, 0, 0),
		];

		// One non-empty sibling slot but no sibling proof: SkipEmpty means the
		// attacker could have moved that hash, so the proof must be rejected.
		assert!(matches!(
			verify_non_existence_proof(&proof, query, &root, &[]),
			Err(Error::SiblingCountMismatch)
		));
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
		assert_eq!(nibble_at_depth(&full_hash, 0), Some(0x0B));
		// depth 1: high nibble of byte 0 = 0xA
		assert_eq!(nibble_at_depth(&full_hash, 1), Some(0x0A));
		// depth 2: low nibble of byte 1 = 0xD
		assert_eq!(nibble_at_depth(&full_hash, 2), Some(0x0D));
		// depth 3: high nibble of byte 1 = 0xC
		assert_eq!(nibble_at_depth(&full_hash, 3), Some(0x0C));

		// Regression: depth beyond the hash length must surface as `None`
		// rather than panicking on an out-of-bounds index. Without this
		// guard an adversarial proof of length >= 66 drives `byte_index`
		// past the end of the 32-byte key hash.
		assert_eq!(nibble_at_depth(&full_hash, 64), None);
		assert_eq!(nibble_at_depth(&full_hash, 65), None);
	}

	#[test]
	fn test_over_deep_proof_rejected() {
		// Regression: prior to the MAX_PROOF_DEPTH guard, a proof with more
		// than 65 nodes would drive `nibble_at_depth` past the 32-byte key
		// hash and panic with index-out-of-bounds inside on-chain execution.
		// Now it must return `ProofTooDeep` cleanly.
		let dummy_leaf = make_leaf(b"k", b"v");
		let mut proof: Vec<PharosProofNode> = Vec::with_capacity(MAX_PROOF_DEPTH + 1);
		for _ in 0..MAX_PROOF_DEPTH {
			proof.push(node(vec![0u8; INTERNAL_NODE_LEN], 0, 0));
		}
		proof.push(node(dummy_leaf, 0, 0));
		assert_eq!(proof.len(), MAX_PROOF_DEPTH + 1);

		let root = [0u8; 32];
		assert!(matches!(verify_proof(&proof, b"k", b"v", &root), Err(Error::ProofTooDeep)));
		assert!(matches!(verify_membership_proof(&proof, b"k", &root), Err(Error::ProofTooDeep)));
		assert!(matches!(
			verify_non_existence_proof(&proof, b"k", &root, &[]),
			Err(Error::ProofTooDeep)
		));
	}
}
