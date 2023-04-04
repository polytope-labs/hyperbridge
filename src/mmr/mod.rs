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
use core::{fmt, fmt::Formatter};

pub mod mmr;
pub mod storage;
mod utils;

use crate::{host::Host, Config};
use codec::{Decode, Encode};
use ismp_rust::{
    host::ISMPHost,
    router::{Request, Response},
};
use sp_runtime::traits;

pub use self::mmr::Mmr;
pub type LeafIndex = u64;
pub type NodeIndex = u64;

#[derive(Debug, Clone, Decode, Encode, PartialEq, Eq)]
pub enum Leaf {
    Request(Request),
    Response(Response),
}

/// A full leaf content stored in the offchain-db.
pub trait FullLeaf<T: Config>: Clone + fmt::Debug + PartialEq + Eq + codec::Codec {
    /// Returns the hash of the leaf
    fn hash(&self) -> <<T as Config>::Hashing as traits::Hash>::Output;
}

impl<T: Config> FullLeaf<T> for Leaf {
    fn hash(&self) -> <<T as Config>::Hashing as traits::Hash>::Output {
        let host = Host::<T>::default();
        match self {
            Leaf::Request(req) => {
                let commitment = host.get_request_commitment(req);
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&commitment[..]);
                <T as Config>::Hash::from(hash)
            }
            Leaf::Response(res) => {
                let commitment = host.get_response_commitment(res);
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&commitment[..]);
                <T as Config>::Hash::from(hash)
            }
        }
    }
}

/// An element representing either full data or its hash.
#[derive(Clone, PartialEq, Eq, Encode, Decode)]
pub enum DataOrHash<T: Config, L> {
    /// Arbitrary data in its full form.
    Data(L),
    /// A hash of some data.
    Hash(<<T as Config>::Hashing as traits::Hash>::Output),
}

impl<T: Config, L: core::fmt::Debug> core::fmt::Debug for DataOrHash<T, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            DataOrHash::Data(leaf) => f.debug_struct("DataOrHash").field("Data", leaf).finish(),
            DataOrHash::Hash(hash) => f.debug_struct("DataOrHash").field("Hash", hash).finish(),
        }
    }
}

impl<T: Config, L> From<L> for DataOrHash<T, L> {
    fn from(l: L) -> Self {
        Self::Data(l)
    }
}

impl<T: Config, L: FullLeaf<T>> DataOrHash<T, L> {
    /// Retrieve a hash of this item.
    ///
    /// Depending on the node type it's going to either be a contained value for [DataOrHash::Hash]
    /// node, or a hash of SCALE-encoded [DataOrHash::Data] data.
    pub fn hash(&self) -> <<T as Config>::Hashing as traits::Hash>::Output {
        match *self {
            Self::Data(ref leaf) => leaf.hash(),
            Self::Hash(ref hash) => *hash,
        }
    }
}
/// Node type for runtime `T`.
pub type NodeOf<T, L> = DataOrHash<T, L>;

/// Default Merging & Hashing behavior for MMR.
pub struct Hasher<T, L>(sp_std::marker::PhantomData<(T, L)>);

impl<T: Config, L: FullLeaf<T>> mmr_lib::Merge for Hasher<T, L> {
    type Item = NodeOf<T, L>;

    fn merge(left: &Self::Item, right: &Self::Item) -> mmr_lib::Result<Self::Item> {
        let mut concat = left.hash().as_ref().to_vec();
        concat.extend_from_slice(right.hash().as_ref());

        Ok(NodeOf::Hash(<<T as Config>::Hashing as traits::Hash>::hash(&concat)))
    }
}
