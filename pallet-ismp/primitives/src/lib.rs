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

extern crate alloc;

use alloc::{format, vec::Vec};
use codec::{Decode, Encode};
use core::{fmt::Debug, time::Duration};
use ismp::{error::Error, host::StateMachine};
use sp_consensus_aura::{Slot, AURA_ENGINE_ID};
use sp_core::H256;
use sp_runtime::{Digest, DigestItem};

pub mod mmr;

/// The `ConsensusEngineId` of ISMP digest in the parachain header.
pub const ISMP_ID: sp_runtime::ConsensusEngineId = *b"ISMP";

/// Queries a request leaf in the mmr
#[derive(codec::Encode, codec::Decode, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct LeafIndexQuery {
    /// The source of the request
    pub source_chain: StateMachine,
    /// the request destination
    pub dest_chain: StateMachine,
    /// The request nonce
    pub nonce: u64,
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
) -> Result<(u64, H256), Error> {
    let (mut timestamp, mut overlay_root) = (0, H256::default());

    for digest in digest.logs.iter() {
        match digest {
            DigestItem::PreRuntime(consensus_engine_id, value)
                if *consensus_engine_id == AURA_ENGINE_ID =>
            {
                let slot = Slot::decode(&mut &value[..])
                    .map_err(|e| Error::ImplementationSpecific(format!("Cannot slot: {e:?}")))?;
                timestamp = Duration::from_millis(*slot * slot_duration).as_secs();
            }
            DigestItem::Consensus(consensus_engine_id, value)
                if *consensus_engine_id == ISMP_ID =>
            {
                if value.len() != 32 {
                    Err(Error::ImplementationSpecific(
                        "Header contains an invalid ismp root".into(),
                    ))?
                }

                overlay_root = H256::from_slice(&value);
            }
            // don't really care about the rest
            _ => {}
        };
    }

    Ok((timestamp, overlay_root))
}
