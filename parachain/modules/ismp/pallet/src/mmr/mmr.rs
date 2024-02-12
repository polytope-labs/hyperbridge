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
    mmr_primitives::{DataOrHash, Leaf, MmrHasher, NodeIndex},
    primitives::{Error, Proof},
    Config, Pallet,
};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::prelude::*;

/// A wrapper around an MMR library to expose limited functionality.
///
/// Available functions depend on the storage kind ([Runtime](crate::mmr::storage::RuntimeStorage)
/// vs [Off-chain](crate::mmr::storage::OffchainStorage)).
pub struct Mmr<StorageType, T>
where
    T: Config,
    Storage<StorageType, T>: merkle_mountain_range::MMRStore<DataOrHash>,
{
    mmr: merkle_mountain_range::MMR<DataOrHash, MmrHasher<Host<T>>, Storage<StorageType, T>>,
}

impl<StorageType, T> Mmr<StorageType, T>
where
    T: Config,
    Storage<StorageType, T>: merkle_mountain_range::MMRStore<DataOrHash>,
{
    /// Create a pointer to an existing MMR with given number of leaves.
    pub fn new(leaves: NodeIndex) -> Self {
        let size = NodesUtils::new(leaves).size();
        Self { mmr: merkle_mountain_range::MMR::new(size, Default::default()) }
    }
}

/// Runtime specific MMR functions.
impl<T> Mmr<RuntimeStorage, T>
where
    T: Config,
{
    /// Push another item to the MMR
    ///
    /// Returns the element position (index).
    pub fn push(&mut self, leaf: Leaf) -> Option<NodeIndex> {
        let pos = self.mmr.push(DataOrHash::Data(leaf)).map_err(|_| Error::Push).ok()?;
        Some(pos)
    }

    /// Calculate the new MMR's root hash.
    pub fn finalize(self) -> Result<H256, Error> {
        let root = self.mmr.get_root().map_err(|_| Error::GetRoot)?;
        Ok(root.hash::<Host<T>>())
    }

    /// Commit the changes to the mmr
    pub fn commit(self) -> Result<(), Error> {
        self.mmr.commit().map_err(|_| Error::Commit)
    }
}

/// Distinguish between requests and responses
#[derive(TypeInfo, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum ProofKeys {
    /// Request commitments
    Requests(Vec<H256>),
    /// Response commitments
    Responses(Vec<H256>),
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
    pub fn generate_proof(&self, keys: ProofKeys) -> Result<(Vec<Leaf>, Proof<H256>), Error> {
        let leaf_indices_and_positions = match keys {
            ProofKeys::Requests(commitments) => commitments
                .into_iter()
                .map(|commitment| {
                    let val = Pallet::<T>::request_commitments(commitment)
                        .ok_or_else(|| Error::LeafNotFound)?
                        .mmr;
                    Ok(val)
                })
                .collect::<Result<Vec<_>, _>>()?,
            ProofKeys::Responses(commitments) => commitments
                .into_iter()
                .map(|commitment| {
                    let val = Pallet::<T>::response_commitments(commitment)
                        .ok_or_else(|| Error::LeafNotFound)?
                        .mmr;
                    Ok(val)
                })
                .collect::<Result<Vec<_>, _>>()?,
        };
        let store = <Storage<OffchainStorage, T>>::default();
        let positions = leaf_indices_and_positions.iter().map(|val| val.pos).collect::<Vec<_>>();
        let leaves = positions
            .iter()
            .map(|pos| match merkle_mountain_range::MMRStore::get_elem(&store, *pos) {
                Ok(Some(DataOrHash::Data(leaf))) => Ok(leaf),
                e => {
                    println!("Error fetching {pos} {e:?}");
                    Err(Error::LeafNotFound)
                },
            })
            .collect::<Result<Vec<_>, Error>>()?;
        log::trace!(target: "runtime::mmr", "Positions {:?}", positions);
        let leaf_count = Pallet::<T>::number_of_leaves();
        self.mmr
            .gen_proof(positions)
            .map_err(|_| Error::GenerateProof)
            .map(|p| Proof {
                leaf_positions: leaf_indices_and_positions,
                leaf_count,
                items: p.proof_items().iter().map(|x| x.hash::<Host<T>>()).collect(),
            })
            .map(|p| (leaves, p))
    }
}
