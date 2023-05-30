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
use ismp::{
    host::IsmpHost,
    router::{Request, Response},
    util::{hash_request, hash_response},
};
use primitive_types::H256;
use sp_runtime::traits;

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
    fn hash<H: IsmpHost>(&self) -> H256 {
        match self {
            Leaf::Request(req) => hash_request::<H>(req),
            Leaf::Response(res) => hash_response::<H>(res),
        }
    }
}

/// An element representing either full data or its hash.
#[derive(Clone, PartialEq, Eq, Encode, Decode, scale_info::TypeInfo)]
pub enum DataOrHash<T: frame_system::Config> {
    /// Arbitrary data in its full form.
    Data(Leaf),
    /// A hash of some data.
    Hash(<<T as frame_system::Config>::Hashing as traits::Hash>::Output),
}

impl<T: frame_system::Config> core::fmt::Debug for DataOrHash<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            DataOrHash::Data(leaf) => f.debug_struct("DataOrHash").field("Data", leaf).finish(),
            DataOrHash::Hash(hash) => f.debug_struct("DataOrHash").field("Hash", hash).finish(),
        }
    }
}

impl<T: frame_system::Config> From<Leaf> for DataOrHash<T> {
    fn from(l: Leaf) -> Self {
        Self::Data(l)
    }
}

impl<T> DataOrHash<T>
where
    T: frame_system::Config,
    T::Hash: From<H256>,
{
    /// Retrieve a hash of this item.
    ///
    /// Depending on the node type it's going to either be a contained value for [DataOrHash::Hash]
    /// node, or a hash of SCALE-encoded [DataOrHash::Data] data.
    pub fn hash<H: IsmpHost>(
        &self,
    ) -> <<T as frame_system::Config>::Hashing as traits::Hash>::Output {
        match *self {
            Self::Data(ref leaf) => <T::Hash>::from(leaf.hash::<H>()),
            Self::Hash(ref hash) => *hash,
        }
    }
}

/// Default Merging & Hashing behavior for MMR.
pub struct MmrHasher<T, H>(core::marker::PhantomData<(T, H)>);

impl<T, H> merkle_mountain_range::Merge for MmrHasher<T, H>
where
    T: frame_system::Config,
    T::Hash: From<H256>,
    H: IsmpHost,
{
    type Item = DataOrHash<T>;

    fn merge(left: &Self::Item, right: &Self::Item) -> merkle_mountain_range::Result<Self::Item> {
        let mut concat = left.hash::<H>().as_ref().to_vec();
        concat.extend_from_slice(right.hash::<H>().as_ref());

        Ok(DataOrHash::Hash(<<T as frame_system::Config>::Hashing as traits::Hash>::hash(&concat)))
    }
}
