// Copyright (C) 2023 Polytope Labs.
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
use codec::Encode;
use frame_support::log::{debug, trace};
use ismp_primitives::mmr::{DataOrHash, NodeIndex};
use mmr_lib::helper;
use sp_core::{offchain::StorageKind, H256};
use sp_std::iter::Peekable;
#[cfg(not(feature = "std"))]
use sp_std::prelude::*;

use crate::{host::Host, mmr::utils::NodesUtils, Config, Pallet};

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
pub struct Storage<StorageType, T>(sp_std::marker::PhantomData<(StorageType, T)>);

impl<StorageType, T> Default for Storage<StorageType, T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> mmr_lib::MMRStore<DataOrHash<T>> for Storage<OffchainStorage, T>
where
    T: Config,
{
    fn get_elem(&self, pos: NodeIndex) -> mmr_lib::Result<Option<DataOrHash<T>>> {
        let key = Pallet::<T>::offchain_key(pos);
        debug!(
            target: "runtime::mmr::offchain", "offchain db get {}: key {:?}",
            pos, key
        );
        // Try to retrieve the element from Off-chain DB.
        if let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
            return Ok(codec::Decode::decode(&mut &*elem).ok())
        }

        Ok(None)
    }

    fn append(&mut self, _: NodeIndex, _: Vec<DataOrHash<T>>) -> mmr_lib::Result<()> {
        panic!("MMR must not be altered in the off-chain context.")
    }
}

impl<T> mmr_lib::MMRStore<DataOrHash<T>> for Storage<RuntimeStorage, T>
where
    T: Config,
    <T as frame_system::Config>::Hash: From<H256>,
{
    fn get_elem(&self, pos: NodeIndex) -> mmr_lib::Result<Option<DataOrHash<T>>> {
        Ok(Pallet::<T>::get_node(pos))
    }

    fn append(&mut self, pos: NodeIndex, elems: Vec<DataOrHash<T>>) -> mmr_lib::Result<()> {
        if elems.is_empty() {
            return Ok(())
        }

        trace!(
            target: "runtime::mmr", "elems: {:?}",
            elems.iter().map(|elem| elem.hash::<Host<T>>()).collect::<Vec<_>>()
        );

        let leaves = Pallet::<T>::get_num_leaves();
        let size = NodesUtils::new(leaves).size();

        if pos != size {
            return Err(mmr_lib::Error::InconsistentStore)
        }

        let new_size = size + elems.len() as NodeIndex;

        // A sorted (ascending) iterator over peak indices to prune and persist.
        let (peaks_to_prune, mut peaks_to_store) = peaks_to_prune_and_store(size, new_size);

        // Now we are going to iterate over elements to insert
        // and keep track of the current `node_index` and `leaf_index`.
        let mut leaf_index = leaves;
        let mut node_index = size;

        for elem in elems {
            // On-chain we are going to only store new peaks.
            if peaks_to_store.next_if_eq(&node_index).is_some() {
                Pallet::<T>::insert_node(node_index, elem.hash::<Host<T>>());
            }
            // We are storing full node off-chain (using indexing API).
            Self::store_to_offchain(node_index, &elem);

            // Increase the indices.
            if let DataOrHash::Data(..) = elem {
                leaf_index += 1;
            }
            node_index += 1;
        }

        // Update current number of leaves.
        Pallet::<T>::set_num_leaves(leaf_index);

        // And remove all remaining items from `peaks_before` collection.
        for pos in peaks_to_prune {
            Pallet::<T>::remove_node(pos);
        }

        Ok(())
    }
}

impl<T> Storage<RuntimeStorage, T>
where
    T: Config,
{
    /// Store a node in the offchain db
    fn store_to_offchain(pos: NodeIndex, node: &DataOrHash<T>) {
        let encoded_node = node.encode();

        let key = Pallet::<T>::offchain_key(pos);
        debug!(
            target: "runtime::mmr::offchain", "offchain db set: pos {} key {:?}",
            pos, key
        );
        // Indexing API is used to store the full node content.
        sp_io::offchain_index::set(&key, &encoded_node);
    }
}

/// Calculate peaks to prune and store
fn peaks_to_prune_and_store(
    old_size: NodeIndex,
    new_size: NodeIndex,
) -> (impl Iterator<Item = NodeIndex>, Peekable<impl Iterator<Item = NodeIndex>>) {
    // A sorted (ascending) collection of peak indices before and after insertion.
    // both collections may share a common prefix.
    let peaks_before = if old_size == 0 { vec![] } else { helper::get_peaks(old_size) };
    let peaks_after = helper::get_peaks(new_size);
    trace!(target: "runtime::mmr", "peaks_before: {:?}", peaks_before);
    trace!(target: "runtime::mmr", "peaks_after: {:?}", peaks_after);
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
