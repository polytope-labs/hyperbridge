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
use crate::mmr::primitives::NodeIndex;
use alloc::format;
use codec::{Decode, Encode};
use core::time::Duration;
use frame_support::{weights::Weight, PalletId};
use ismp::consensus::ConsensusClient;
use scale_info::TypeInfo;
use sp_consensus_aura::{Slot, AURA_ENGINE_ID};
use sp_core::{
    crypto::{AccountId32, ByteArray},
    H160, H256,
};
use sp_runtime::{Digest, DigestItem, RuntimeDebug};
use sp_std::prelude::*;

/// An MMR proof data for a group of leaves.
#[derive(codec::Encode, codec::Decode, RuntimeDebug, Clone, PartialEq, Eq, TypeInfo)]
pub struct Proof<Hash> {
    /// The positions of the leaves the proof is for.
    pub leaf_positions: Vec<LeafIndexAndPos>,
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

/// A trait that returns a list of all configured consensus clients
/// This trait should be implemented in the runtime
pub trait ConsensusClientProvider {
    /// Returns a list of all configured consensus clients
    fn consensus_clients() -> Vec<Box<dyn ConsensusClient>>;
}

fortuples::fortuples! {
    #[tuples::max_size(30)]
    impl ConsensusClientProvider for #Tuple
    where
        #(#Member: ConsensusClient + Default + 'static),*
    {

        fn consensus_clients() -> Vec<Box<dyn ConsensusClient>> {
            vec![
                #( Box::new(#Member::default()) as Box<dyn ConsensusClient> ),*
            ]
        }
    }
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

/// The `ConsensusEngineId` of ISMP digest in the parachain header.
pub const ISMP_ID: sp_runtime::ConsensusEngineId = *b"ISMP";

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

/// Hashing algorithm for the state proof
#[derive(Debug, Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub enum HashAlgorithm {
    /// For chains that use keccak as their hashing algo
    Keccak,
    /// For chains that use blake2 as their hashing algo
    Blake2,
}

/// Holds the relevant data needed for state proof verification
#[derive(Debug, Encode, Decode, Clone)]
pub struct SubstrateStateProof {
    /// Algorithm to use for state proof verification
    pub hasher: HashAlgorithm,
    /// Storage proof for the parachain headers
    pub storage_proof: Vec<Vec<u8>>,
}

/// Holds the relevant data needed for request/response proof verification
#[derive(Debug, Encode, Decode, Clone)]
pub struct MembershipProof {
    /// Size of the mmr at the time this proof was generated
    pub mmr_size: u64,
    /// Leaf indices for the proof
    pub leaf_indices: Vec<u64>,
    /// Mmr proof
    pub proof: Vec<H256>,
}

/// Fetches the overlay(ismp) root and timestamp from the header digest
pub fn fetch_overlay_root_and_timestamp(
    digest: &Digest,
    slot_duration: u64,
) -> Result<(u64, H256), ismp::error::Error> {
    let (mut timestamp, mut overlay_root) = (0, H256::default());

    for digest in digest.logs.iter() {
        match digest {
            DigestItem::PreRuntime(consensus_engine_id, value)
                if *consensus_engine_id == AURA_ENGINE_ID =>
            {
                let slot = Slot::decode(&mut &value[..]).map_err(|e| {
                    ismp::error::Error::ImplementationSpecific(format!("Cannot slot: {e:?}"))
                })?;
                timestamp = Duration::from_millis(*slot * slot_duration).as_secs();
            },
            DigestItem::Consensus(consensus_engine_id, value)
                if *consensus_engine_id == ISMP_ID =>
            {
                if value.len() != 32 {
                    Err(ismp::error::Error::ImplementationSpecific(
                        "Header contains an invalid ismp root".into(),
                    ))?
                }

                overlay_root = H256::from_slice(&value);
            },
            // don't really care about the rest
            _ => {},
        };
    }

    Ok((timestamp, overlay_root))
}
