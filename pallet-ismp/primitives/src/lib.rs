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

//! The primitive types used by pallet-ismp

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

//! Primitives for the MMR implementation
use ismp::host::StateMachine;

pub mod mmr;

/// Queries a request leaf in the mmr
#[derive(codec::Encode, codec::Decode)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct LeafIndexQuery {
    /// The source of the request
    pub source_chain: StateMachine,
    /// the request destination
    pub dest_chain: StateMachine,
    /// The request nonce
    pub nonce: u64,
}
