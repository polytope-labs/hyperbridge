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

//! MMR utilities

use core::fmt::Formatter;

use codec::{Decode, Encode};
use frame_support::sp_io;
use ismp::{
    router::{Request, Response},
    util::{hash_request, hash_response, Keccak256},
};
use primitive_types::H256;

/// Index of a leaf in the MMR
pub type LeafIndex = u64;
/// Index of a node in the MMR
pub type NodeIndex = u64;

/// A concrete Leaf for the MMR
#[derive(Debug, Clone, Decode, Encode, PartialEq, Eq, scale_info::TypeInfo)]
pub enum Leaf {
    /// A request variant
    Request(Request),
    /// A response variant
    Response(Response),
}

impl Leaf {
    /// Returns the hash of a leaf
    fn hash<H: Keccak256>(&self) -> H256 {
        match self {
            Leaf::Request(req) => hash_request::<H>(req),
            Leaf::Response(res) => hash_response::<H>(res),
        }
    }
}

/// An element representing either full data or its hash.
#[derive(Clone, PartialEq, Eq, Encode, Decode, scale_info::TypeInfo)]
pub enum DataOrHash {
    /// Arbitrary data in its full form.
    Data(Leaf),
    /// A hash of some data.
    Hash(H256),
}

impl core::fmt::Debug for DataOrHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            DataOrHash::Data(leaf) => f.debug_struct("DataOrHash").field("Data", leaf).finish(),
            DataOrHash::Hash(hash) => f.debug_struct("DataOrHash").field("Hash", hash).finish(),
        }
    }
}

impl From<Leaf> for DataOrHash {
    fn from(l: Leaf) -> Self {
        Self::Data(l)
    }
}

impl DataOrHash {
    /// Retrieve a hash of this item.
    ///
    /// Depending on the node type it's going to either be a contained value for [DataOrHash::Hash]
    /// node, or a hash of SCALE-encoded [DataOrHash::Data] data.
    pub fn hash<H: Keccak256>(&self) -> H256 {
        match *self {
            Self::Data(ref leaf) => leaf.hash::<H>(),
            Self::Hash(ref hash) => *hash,
        }
    }
}

/// Default Merging & Hashing behavior for MMR.
pub struct MmrHasher<H>(core::marker::PhantomData<H>);

impl<H> merkle_mountain_range::Merge for MmrHasher<H>
where
    H: Keccak256,
{
    type Item = DataOrHash;

    fn merge(left: &Self::Item, right: &Self::Item) -> merkle_mountain_range::Result<Self::Item> {
        let mut concat = left.hash::<H>().as_ref().to_vec();
        concat.extend_from_slice(right.hash::<H>().as_ref());

        Ok(DataOrHash::Hash(sp_io::hashing::keccak_256(&concat).into()))
    }
}
