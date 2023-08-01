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

use crate::{
    host::Host,
    mmr::{
        storage::{OffchainStorage, RuntimeStorage, Storage},
        utils::NodesUtils,
    },
    primitives::{Error, Proof},
    Config,
};
use ismp_primitives::mmr::{DataOrHash, Leaf, MmrHasher, NodeIndex};
use sp_core::H256;
use sp_std::prelude::*;

/// A wrapper around an MMR library to expose limited functionality.
///
/// Available functions depend on the storage kind ([Runtime](crate::mmr::storage::RuntimeStorage)
/// vs [Off-chain](crate::mmr::storage::OffchainStorage)).
pub struct Mmr<StorageType, T>
where
    T: Config,
    Storage<StorageType, T>: mmr_lib::MMRStore<DataOrHash>,
{
    mmr: mmr_lib::MMR<DataOrHash, MmrHasher<Host<T>>, Storage<StorageType, T>>,
    leaves: NodeIndex,
}

impl<StorageType, T> Mmr<StorageType, T>
where
    T: Config,
    Storage<StorageType, T>: mmr_lib::MMRStore<DataOrHash>,
{
    /// Create a pointer to an existing MMR with given number of leaves.
    pub fn new(leaves: NodeIndex) -> Self {
        let size = NodesUtils::new(leaves).size();
        Self { mmr: mmr_lib::MMR::new(size, Default::default()), leaves }
    }
}

/// Runtime specific MMR functions.
impl<T> Mmr<RuntimeStorage, T>
where
    T: Config,
{
    /// Push another item to the MMR and commit
    ///
    /// Returns number of leaves and the element position (index) in the MMR.
    pub fn push(mut self, leaf: Leaf) -> Option<NodeIndex> {
        let position = self.mmr.push(DataOrHash::Data(leaf)).map_err(|_| Error::Push).ok()?;
        let num_leaves = self.leaves + 1;
        self.leaves = num_leaves;
        self.mmr.commit().ok()?;
        Some(position)
    }

    /// Calculate the new MMR's root hash.
    pub fn finalize(self) -> Result<H256, Error> {
        let root = self.mmr.get_root().map_err(|_| Error::GetRoot)?;
        Ok(root.hash::<Host<T>>())
    }
}

/// Off-chain specific MMR functions.
impl<T> Mmr<OffchainStorage, T>
where
    T: Config,
{
    /// Generate a proof for given leaf indices.
    ///
    /// Proof generation requires all the nodes (or their hashes) to be available in the storage.
    /// (i.e. you can't run the function in the pruned storage).
    pub fn generate_proof(
        &self,
        positions: Vec<NodeIndex>,
    ) -> Result<(Vec<Leaf>, Proof<H256>), Error> {
        let store = <Storage<OffchainStorage, T>>::default();
        let leaves = positions
            .iter()
            .map(|pos| match mmr_lib::MMRStore::get_elem(&store, *pos) {
                Ok(Some(DataOrHash::Data(leaf))) => Ok(leaf),
                _ => Err(Error::LeafNotFound),
            })
            .collect::<Result<Vec<_>, Error>>()?;
        log::trace!(target: "runtime::mmr", "Positions {:?}", positions);
        let leaf_count = self.leaves;
        self.mmr
            .gen_proof(positions.clone())
            .map_err(|_| Error::GenerateProof)
            .map(|p| Proof {
                leaf_indices: positions,
                leaf_count,
                items: p.proof_items().iter().map(|x| x.hash::<Host<T>>()).collect(),
            })
            .map(|p| (leaves, p))
    }
}
