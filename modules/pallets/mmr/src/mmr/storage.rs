// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

//! An MMR storage implementation.

use alloc::{vec, vec::Vec};
use codec::Encode;
use log::{debug, trace};
use merkle_mountain_range::helper;
use pallet_ismp::offchain::ForkIdentifier;
use polkadot_sdk::*;
use sp_core::{offchain::StorageKind, H256};
use sp_io::offchain_index;
use sp_mmr_primitives::{utils::NodesUtils, NodeIndex};
use sp_std::iter::Peekable;

use crate::{
	mmr::{Node, NodeOf},
	Config, HashOf, Nodes, NumberOfLeaves, Pallet,
};

/// A marker type for runtime-specific storage implementation.
///
/// Allows appending new items to the MMR and proof verification.
/// MMR nodes are appended to two different storages:
/// 1. We add nodes (leaves) hashes to the on-chain storage (see [crate::Nodes]).
/// 2. We add full leaves (and all inner nodes as well) into the `IndexingAPI` during block
///    processing, so the values end up in the Offchain DB if indexing is enabled.
pub struct RuntimeStorage;

/// A marker type for offchain-specific storage implementation.
///
/// Allows proof generation and verification, but does not support appending new items.
/// MMR nodes are assumed to be stored in the Off-Chain DB. Note this storage type
/// DOES NOT support adding new items to the MMR.
pub struct OffchainStorage;

/// A storage layer for MMR.
///
/// There are two different implementations depending on the use case.
/// See docs for [RuntimeStorage] and [OffchainStorage].
pub struct Storage<StorageType, T, I, L>(sp_std::marker::PhantomData<(StorageType, T, I, L)>);

impl<StorageType, T, I, L> Default for Storage<StorageType, T, I, L> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<T, I, L> merkle_mountain_range::MMRStore<NodeOf<T, I, L>> for Storage<OffchainStorage, T, I, L>
where
	T: Config<I>,
	I: 'static,
	L: pallet_ismp::offchain::FullLeaf,
	HashOf<T, I>: Into<H256>,
{
	fn get_elem(&self, pos: NodeIndex) -> merkle_mountain_range::Result<Option<NodeOf<T, I, L>>> {
		// We should only get here when trying to generate proofs. The client requests
		// for proofs for finalized blocks, which should usually be already canonicalized,
		// unless the MMR client gadget has a delay.
		let key = Pallet::<T, I>::node_canon_offchain_key(pos);
		debug!(
			target: "pallet-mmr", "offchain db get {}: canon key {:?}",
			pos, key
		);
		// Try to retrieve the element from Off-chain DB.
		if let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
			return Ok(codec::Decode::decode(&mut &*elem).ok());
		} else {
			// alas the store hasn't been canonicalized yet
			Err(merkle_mountain_range::Error::InconsistentStore)?
		}
	}

	fn append(
		&mut self,
		_: NodeIndex,
		_: Vec<NodeOf<T, I, L>>,
	) -> merkle_mountain_range::Result<()> {
		panic!("MMR must not be altered in the off-chain context.")
	}
}

impl<T, I, L> merkle_mountain_range::MMRStore<NodeOf<T, I, L>> for Storage<RuntimeStorage, T, I, L>
where
	T: Config<I>,
	I: 'static,
	L: pallet_ismp::offchain::FullLeaf,
	HashOf<T, I>: Into<H256>,
{
	fn get_elem(&self, pos: NodeIndex) -> merkle_mountain_range::Result<Option<NodeOf<T, I, L>>> {
		Ok(Nodes::<T, I>::get(pos).map(Node::Hash))
	}

	fn append(
		&mut self,
		pos: NodeIndex,
		elems: Vec<NodeOf<T, I, L>>,
	) -> merkle_mountain_range::Result<()> {
		if elems.is_empty() {
			return Ok(());
		}

		trace!(
			target: "pallet-mmr", "elems: {:?}",
			elems.iter().map(|elem| elem.hash()).collect::<Vec<_>>()
		);

		let leaves = NumberOfLeaves::<T, I>::get();
		let size = NodesUtils::new(leaves).size();

		if pos != size {
			return Err(merkle_mountain_range::Error::InconsistentStore);
		}

		let new_size = size + elems.len() as NodeIndex;

		// A sorted (ascending) iterator over peak indices to prune and persist.
		let (peaks_to_prune, mut peaks_to_store) = peaks_to_prune_and_store(size, new_size);

		// Now we are going to iterate over elements to insert
		// and keep track of the current `node_index` and `leaf_index`.
		let mut leaf_index = leaves;
		let mut node_index = size;

		// Use a uniquely generated hash for every block as an extra identifier
		// in offchain DB to avoid DB collisions and overwrites in case of forks.
		let fork_identifier = <T::ForkIdentifierProvider as ForkIdentifier<T>>::identifier();
		for elem in elems {
			// On-chain we are going to only store new peaks.
			if peaks_to_store.next_if_eq(&node_index).is_some() {
				Nodes::<T, I>::insert(node_index, elem.hash());
			}
			// We are storing full node off-chain (using indexing API).
			Self::store_to_offchain(node_index, fork_identifier, &elem);

			// Increase the indices.
			if let Node::Data(..) = elem {
				leaf_index += 1;
			}
			node_index += 1;
		}

		// Update current number of leaves.
		NumberOfLeaves::<T, I>::put(leaf_index);

		// And remove all remaining items from `peaks_before` collection.
		for pos in peaks_to_prune {
			Nodes::<T, I>::remove(pos);
		}

		Ok(())
	}
}

impl<T, I, L> Storage<RuntimeStorage, T, I, L>
where
	T: Config<I>,
	I: 'static,
	L: pallet_ismp::offchain::FullLeaf,
	HashOf<T, I>: Into<H256>,
{
	fn store_to_offchain(
		pos: NodeIndex,
		fork_identifier: <T as frame_system::Config>::Hash,
		node: &NodeOf<T, I, L>,
	) {
		let encoded_node = node.encode();
		// We store this leaf offchain keyed by `(prefix, node_index)` to make it
		// fork-resistant. The MMR client gadget task will "canonicalize" it on the first
		// finality notification that follows, when we are not worried about forks anymore.
		let temp_key = Pallet::<T, I>::node_temp_offchain_key(pos, fork_identifier);
		debug!(
			target: "pallet-mmr::offchain", "offchain db set: pos {} fork_identifier {:?} key {:?}",
			pos, fork_identifier, temp_key
		);
		// Indexing API is used to store the full node content.
		offchain_index::set(&temp_key, &encoded_node);

		// if its a leaf, make it immediately available
		if let Node::Data(leaf) = node {
			let encoded = leaf.preimage();
			let commitment = sp_io::hashing::keccak_256(&encoded);
			let offchain_key = pallet_ismp::offchain::leaf_default_key(commitment.into());
			sp_io::offchain_index::set(&offchain_key, &leaf.encode());
		}
	}
}

fn peaks_to_prune_and_store(
	old_size: NodeIndex,
	new_size: NodeIndex,
) -> (impl Iterator<Item = NodeIndex>, Peekable<impl Iterator<Item = NodeIndex>>) {
	// A sorted (ascending) collection of peak indices before and after insertion.
	// both collections may share a common prefix.
	let peaks_before = if old_size == 0 { vec![] } else { helper::get_peaks(old_size) };
	let peaks_after = helper::get_peaks(new_size);
	trace!(target: "pallet-mmr", "peaks_before: {:?}", peaks_before);
	trace!(target: "pallet-mmr", "peaks_after: {:?}", peaks_after);
	let mut peaks_before = peaks_before.into_iter().peekable();
	let mut peaks_after = peaks_after.into_iter().peekable();

	// Consume a common prefix between `peaks_before` and `peaks_after`,
	// since that's something we will not be touching anyway.
	while peaks_before.peek() == peaks_after.peek() {
		peaks_before.next();
		peaks_after.next();
	}

	// what's left in both collections is:
	// 1. Old peaks to remove from storage
	// 2. New peaks to persist in storage
	(peaks_before, peaks_after)
}
