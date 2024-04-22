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
//! Mmr utilities

use crate::{
    child_trie::{RequestCommitments, ResponseCommitments},
    primitives::Proof,
    Config,
};
use codec::{Decode, Encode, Input};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::prelude::*;

use ismp::router::{Request, Response};
use sp_mmr_primitives::Error;
use sp_runtime::traits::Hash;

use sp_mmr_primitives::FullLeaf;

/// A concrete Leaf for the MMR
#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub enum Leaf {
    /// A request variant
    Request(Request),
    /// A response variant
    Response(Response),
}

impl codec::Encode for Leaf {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        let encoded = match self {
            Leaf::Request(req) => req.encode(),
            Leaf::Response(res) => res.encode(),
        };
        f(&encoded)
    }
}
impl codec::Decode for Leaf {
    fn decode<I: Input>(input: &mut I) -> Result<Self, codec::Error> {
        todo!()
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

/// Used for interacting with pallet mmr
pub struct Mmr<T>(core::marker::PhantomData<T>);

/// On-chain specific MMR functions.
impl<T> Mmr<T>
where
    T: Config,
    Leaf: From<<T as pallet_mmr::Config>::Leaf>,
{
    /// Generate a proof for given leaf indices.
    ///
    /// Proof generation requires all the nodes (or their hashes) to be available in the storage.
    /// (i.e. you can't run the function in the pruned storage).
    pub fn generate_proof(
        keys: ProofKeys,
    ) -> Result<(Vec<Leaf>, Proof<<<T as pallet_mmr::Config>::Hashing as Hash>::Output>), Error>
    {
        let leaf_indices_and_positions = match keys {
            ProofKeys::Requests(commitments) => commitments
                .into_iter()
                .map(|commitment| {
                    let val = RequestCommitments::<T>::get(commitment)
                        .ok_or_else(|| Error::LeafNotFound)?
                        .mmr;
                    Ok(val)
                })
                .collect::<Result<Vec<_>, _>>()?,
            ProofKeys::Responses(commitments) => commitments
                .into_iter()
                .map(|commitment| {
                    let val = ResponseCommitments::<T>::get(commitment)
                        .ok_or_else(|| Error::LeafNotFound)?
                        .mmr;
                    Ok(val)
                })
                .collect::<Result<Vec<_>, _>>()?,
        };
        let indices =
            leaf_indices_and_positions.iter().map(|val| val.leaf_index).collect::<Vec<_>>();
        let (leaves, proof) = pallet_mmr::Pallet::<T>::generate_proof(indices)?;
        let proof = Proof {
            leaf_positions: leaf_indices_and_positions,
            leaf_count: proof.leaf_count,
            items: proof.items,
        };

        Ok((leaves.into_iter().map(|leaf| leaf.into()).collect(), proof))
    }
}
