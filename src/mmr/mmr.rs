// This file is part of Substrate.

// Copyright (C) 2020-2022 Parity Technologies (UK) Ltd.
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

use crate::mmr::storage::{OffchainKeyGenerator, StorageReadWrite};
use crate::mmr::utils::NodesUtils;
use crate::mmr::{FullLeaf, HashingOf, NodeIndex};
use crate::primitives::Proof;
use crate::{
    mmr::{
        storage::{OffchainStorage, RuntimeStorage, Storage},
        Hasher, Node, NodeOf,
    },
    primitives::Error,
    Config, RequestOffchainKey, RequestsStore, ResponseOffchainKey, ResponseStore,
};
use sp_std::prelude::*;

/// A wrapper around an MMR library to expose limited functionality.
///
/// Available functions depend on the storage kind ([Runtime](crate::mmr::storage::RuntimeStorage)
/// vs [Off-chain](crate::mmr::storage::OffchainStorage)).
pub struct Mmr<StorageType, T, L, K, ReadWrite>
where
    T: Config,
    L: FullLeaf<<T as Config>::Hashing>,
    ReadWrite: StorageReadWrite<T, L>,
    K: OffchainKeyGenerator,
    Storage<StorageType, T, L, K, ReadWrite>: mmr_lib::MMRStore<NodeOf<T, L>>,
{
    mmr: mmr_lib::MMR<
        NodeOf<T, L>,
        Hasher<HashingOf<T>, L>,
        Storage<StorageType, T, L, K, ReadWrite>,
    >,
    leaves: NodeIndex,
}

impl<StorageType, T, L, K, ReadWrite> Mmr<StorageType, T, L, K, ReadWrite>
where
    T: Config,
    L: FullLeaf<<T as Config>::Hashing>,
    ReadWrite: StorageReadWrite<T, L>,
    K: OffchainKeyGenerator,
    Storage<StorageType, T, L, K, ReadWrite>: mmr_lib::MMRStore<NodeOf<T, L>>,
{
    /// Create a pointer to an existing MMR with given number of leaves.
    pub fn new(leaves: NodeIndex) -> Self {
        let size = NodesUtils::new(leaves).size();
        Self {
            mmr: mmr_lib::MMR::new(size, Default::default()),
            leaves,
        }
    }

    /// Return the internal size of the MMR (number of nodes).
    #[cfg(test)]
    pub fn size(&self) -> NodeIndex {
        self.mmr.mmr_size()
    }
}

/// Runtime specific MMR functions.
impl<T, L, K, ReadWrite> Mmr<RuntimeStorage, T, L, K, ReadWrite>
where
    T: Config,
    L: FullLeaf<<T as Config>::Hashing>,
    ReadWrite: StorageReadWrite<T, L>,
    K: OffchainKeyGenerator,
{
    /// Push another item to the MMR.
    ///
    /// Returns element position (index) in the MMR.
    pub fn push(&mut self, leaf: L) -> Option<NodeIndex> {
        let position = self
            .mmr
            .push(Node::Data(leaf))
            .map_err(|_| Error::Push)
            .ok()?;

        self.leaves += 1;

        Some(position)
    }

    /// Commit the changes to underlying storage, return current number of leaves and
    /// calculate the new MMR's root hash.
    pub fn finalize(self) -> Result<(NodeIndex, <T as Config>::Hash), Error> {
        let root = self.mmr.get_root().map_err(|_| Error::GetRoot)?;
        self.mmr.commit().map_err(|_| Error::Commit)?;
        Ok((self.leaves, root.hash()))
    }
}

/// Off-chain specific MMR functions.
impl<T, L, K, ReadWrite> Mmr<OffchainStorage, T, L, K, ReadWrite>
where
    T: Config,
    L: FullLeaf<<T as Config>::Hashing> + codec::Decode,
    ReadWrite: StorageReadWrite<T, L>,
    K: OffchainKeyGenerator,
{
    /// Generate a proof for given leaf indices.
    ///
    /// Proof generation requires all the nodes (or their hashes) to be available in the storage.
    /// (i.e. you can't run the function in the pruned storage).
    pub fn generate_request_proof(
        &self,
        leaf_indices: Vec<NodeIndex>,
    ) -> Result<(Vec<L>, Proof<<T as Config>::Hash>), Error> {
        let positions = leaf_indices
            .iter()
            .map(|index| mmr_lib::leaf_index_to_pos(*index))
            .collect::<Vec<_>>();
        let store =
            <Storage<OffchainStorage, T, L, RequestOffchainKey<T, L>, RequestsStore<T>>>::default();
        let leaves = positions
            .iter()
            .map(|pos| match mmr_lib::MMRStore::get_elem(&store, *pos) {
                Ok(Some(Node::Data(leaf))) => Ok(leaf),
                _ => Err(Error::LeafNotFound),
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let leaf_count = self.leaves;
        self.mmr
            .gen_proof(positions)
            .map_err(|_| Error::GenerateProof)
            .map(|p| Proof {
                leaf_indices,
                leaf_count,
                items: p.proof_items().iter().map(|x| x.hash()).collect(),
            })
            .map(|p| (leaves, p))
    }

    pub fn generate_response_proof(
        &self,
        leaf_indices: Vec<NodeIndex>,
    ) -> Result<(Vec<L>, Proof<<T as Config>::Hash>), Error> {
        let positions = leaf_indices
            .iter()
            .map(|index| mmr_lib::leaf_index_to_pos(*index))
            .collect::<Vec<_>>();
        let store =
            <Storage<OffchainStorage, T, L, ResponseOffchainKey<T, L>, ResponseStore<T>>>::default(
            );
        let leaves = positions
            .iter()
            .map(|pos| match mmr_lib::MMRStore::get_elem(&store, *pos) {
                Ok(Some(Node::Data(leaf))) => Ok(leaf),
                _ => Err(Error::LeafNotFound),
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let leaf_count = self.leaves;
        self.mmr
            .gen_proof(positions)
            .map_err(|_| Error::GenerateProof)
            .map(|p| Proof {
                leaf_indices,
                leaf_count,
                items: p.proof_items().iter().map(|x| x.hash()).collect(),
            })
            .map(|p| (leaves, p))
    }
}
