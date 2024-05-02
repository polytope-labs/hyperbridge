// Copyright (c) 2024 Polytope Labs.
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

use codec::{Decode, Encode};
use frame_support::__private::RuntimeDebug;
use ismp::router::{Request, Response};
use mmr_primitives::FullLeaf;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_mmr_primitives::NodeIndex;
use sp_std::prelude::*;

/// Queries a request leaf in the mmr
#[derive(codec::Encode, codec::Decode, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct LeafIndexQuery {
    /// Request or response commitment
    pub commitment: H256,
}

/// Leaf index and position
#[derive(
    codec::Encode,
    codec::Decode,
    scale_info::TypeInfo,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Clone,
    Copy,
    RuntimeDebug,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct LeafIndexAndPos {
    /// Leaf index
    pub leaf_index: u64,
    /// Leaf position
    pub pos: u64,
}

/// A concrete Leaf for the MMR
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
pub enum Leaf {
    /// A request variant
    Request(Request),
    /// A response variant
    Response(Response),
}

impl FullLeaf for Leaf {
    fn preimage(&self) -> Vec<u8> {
        match self {
            Leaf::Request(req) => req.encode(),
            Leaf::Response(res) => res.encode(),
        }
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

/// An MMR proof data for a group of leaves.
#[derive(codec::Encode, codec::Decode, RuntimeDebug, Clone, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Proof<Hash> {
    /// The indices and positions of the leaves in the proof.
    pub leaf_indices_and_pos: Vec<LeafIndexAndPos>,
    /// Number of leaves in MMR, when the proof was generated.
    pub leaf_count: NodeIndex,
    /// Proof elements (hashes of siblings of inner nodes on the path to the leaf).
    pub items: Vec<Hash>,
}
