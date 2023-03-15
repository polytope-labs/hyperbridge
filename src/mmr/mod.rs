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
use core::fmt;
pub mod mmr;
pub mod storage;
mod utils;

use crate::Config;
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use ismp_rust::router::{Request, Response};
use sp_runtime::traits;

pub use self::mmr::Mmr;
pub type LeafIndex = u64;
pub type NodeIndex = u64;

pub(crate) type HashingOf<T> = <T as Config>::Hashing;

#[derive(Debug, Clone, Decode, Encode, PartialEq, Eq)]
pub enum Leaf {
    Request(Request),
    Response(Response),
}

/// A full leaf content stored in the offchain-db.
pub trait FullLeaf<H: traits::Hash>: Clone + fmt::Debug + PartialEq + Eq + codec::Codec {
    /// Returns the hash of the leaf
    fn hash(&self) -> H::Output;
}

impl<H: traits::Hash> FullLeaf<H> for Leaf {
    fn hash(&self) -> H::Output {
        todo!()
    }
}

/// An element representing either full data or its hash.
#[derive(RuntimeDebug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum DataOrHash<H: traits::Hash, L> {
    /// Arbitrary data in its full form.
    Data(L),
    /// A hash of some data.
    Hash(H::Output),
}

impl<H: traits::Hash, L> From<L> for DataOrHash<H, L> {
    fn from(l: L) -> Self {
        Self::Data(l)
    }
}

impl<H: traits::Hash, L: FullLeaf<H>> DataOrHash<H, L> {
    /// Retrieve a hash of this item.
    ///
    /// Depending on the node type it's going to either be a contained value for [DataOrHash::Hash]
    /// node, or a hash of SCALE-encoded [DataOrHash::Data] data.
    pub fn hash(&self) -> H::Output {
        match *self {
            Self::Data(ref leaf) => leaf.hash(),
            Self::Hash(ref hash) => *hash,
        }
    }
}
/// Node type for runtime `T`.
pub type NodeOf<T, L> = Node<<T as crate::Config>::Hashing, L>;

/// A node stored in the MMR.
pub type Node<H, L> = DataOrHash<H, L>;

/// Default Merging & Hashing behavior for MMR.
pub struct Hasher<H, L>(sp_std::marker::PhantomData<(H, L)>);

impl<H: traits::Hash, L: FullLeaf<H>> mmr_lib::Merge for Hasher<H, L> {
    type Item = Node<H, L>;

    fn merge(left: &Self::Item, right: &Self::Item) -> mmr_lib::Result<Self::Item> {
        let mut concat = left.hash().as_ref().to_vec();
        concat.extend_from_slice(right.hash().as_ref());

        Ok(Node::Hash(<H as traits::Hash>::hash(&concat)))
    }
}
