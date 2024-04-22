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

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::prelude::*;

use ismp::router::{Request, Response};

use mmr_primitives::FullLeaf;

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
