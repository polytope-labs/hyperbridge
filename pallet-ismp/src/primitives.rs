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

//! Pallet primitives
use frame_support::{PalletId, RuntimeDebug};
use ismp_primitives::mmr::{LeafIndex, NodeIndex};
use ismp_rs::consensus::{ConsensusClient, ConsensusClientId};
use scale_info::TypeInfo;
use sp_core::{crypto::AccountId32, H160};
use sp_std::prelude::*;

/// The `ConsensusEngineId` of ISMP.
pub const ISMP_ID: sp_runtime::ConsensusEngineId = *b"ISMP";

/// An MMR proof data for a group of leaves.
#[derive(codec::Encode, codec::Decode, RuntimeDebug, Clone, PartialEq, Eq, TypeInfo)]
pub struct Proof<Hash> {
    /// The indices of the leaves the proof is for.
    pub leaf_indices: Vec<LeafIndex>,
    /// Number of leaves in MMR, when the proof was generated.
    pub leaf_count: NodeIndex,
    /// Proof elements (hashes of siblings of inner nodes on the path to the leaf).
    pub items: Vec<Hash>,
}

/// Merkle Mountain Range operation error.
#[derive(RuntimeDebug, codec::Encode, codec::Decode, PartialEq, Eq, scale_info::TypeInfo)]
#[allow(missing_docs)]
pub enum Error {
    InvalidNumericOp,
    Push,
    GetRoot,
    Commit,
    GenerateProof,
    Verify,
    LeafNotFound,
    PalletNotIncluded,
    InvalidLeafIndex,
    InvalidBestKnownBlock,
}

/// A trait that returns a reference to a consensus client based on its Id
/// This trait should be implemented in the runtime
pub trait ConsensusClientProvider {
    /// Returns a reference to a consensus client
    fn consensus_client(
        id: ConsensusClientId,
    ) -> Result<Box<dyn ConsensusClient>, ismp_rs::error::Error>;
}

/// Module identification types supported by ismp
#[derive(codec::Encode, codec::Decode, PartialEq, Eq, scale_info::TypeInfo)]
pub enum ModuleId {
    /// Unique Pallet identification in runtime
    Pallet(PalletId),
    /// Contract account id
    Contract(AccountId32),
    /// Evm contract
    Evm(H160),
}
