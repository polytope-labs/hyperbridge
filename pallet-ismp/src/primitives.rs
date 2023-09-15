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
use codec::{Decode, Encode};
use frame_support::{weights::Weight, PalletId};
use ismp_primitives::mmr::{LeafIndex, NodeIndex};
use ismp_rs::consensus::{ConsensusClient, ConsensusClientId};
use scale_info::TypeInfo;
use sp_core::{
    crypto::{AccountId32, ByteArray},
    H160,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

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
#[derive(PartialEq, Eq, scale_info::TypeInfo)]
pub enum ModuleId {
    /// Unique Pallet identification in runtime
    Pallet(PalletId),
    /// Contract account id
    Contract(AccountId32),
    /// Evm contract
    Evm(H160),
}

impl ModuleId {
    /// Convert module id to raw bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            ModuleId::Pallet(pallet_id) => pallet_id.0.to_vec(),
            ModuleId::Contract(account_id) => account_id.as_slice().to_vec(),
            ModuleId::Evm(account_id) => account_id.0.to_vec(),
        }
    }

    /// Derive module id from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() == 8 {
            let mut inner = [0u8; 8];
            inner.copy_from_slice(bytes);
            Ok(Self::Pallet(PalletId(inner)))
        } else if bytes.len() == 32 {
            Ok(Self::Contract(AccountId32::from_slice(bytes).expect("Infallible")))
        } else if bytes.len() == 20 {
            Ok(Self::Evm(H160::from_slice(bytes)))
        } else {
            Err("Unknown Module ID format")
        }
    }
}

/// Accumulated Weight consumed by contract callbacks in a transaction
#[derive(Default, scale_info::TypeInfo, Encode, Decode)]
pub struct WeightUsed {
    /// Total weight used in executing contract callbacks in a transaction
    pub weight_used: Weight,
    /// Total weight limit used in executing contract callbacks in a transaction
    pub weight_limit: Weight,
}
