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

//! GRANDPA consensus client verification function

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::all)]
#![deny(missing_docs)]

#[cfg(test)]
mod tests;

extern crate alloc;

use alloc::collections::BTreeMap;
use anyhow::anyhow;
use codec::Decode;
use finality_grandpa::Chain;
use primitives::{
    justification::{find_scheduled_change, AncestryChain, GrandpaJustification},
    parachain_header_storage_key, ConsensusState, FinalityProof, ParachainHeadersWithFinalityProof,
};
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Header};
use sp_std::prelude::*;
use sp_trie::StorageProof;
use substrate_state_machine::read_proof_check;

/// This function verifies the GRANDPA finality proof for both standalone chain and parachain
/// headers.
pub fn verify_grandpa_finality_proof<H>(
    mut consensus_state: ConsensusState,
    finality_proof: FinalityProof<H>,
) -> Result<(ConsensusState, H, Vec<H256>, AncestryChain<H>), anyhow::Error>
where
    H: Header<Hash = H256, Number = u32>,
    H::Number: finality_grandpa::BlockNumberOps + Into<u32>,
{
    // First validate unknown headers.
    let headers = AncestryChain::<H>::new(&finality_proof.unknown_headers);

    let target = finality_proof
        .unknown_headers
        .iter()
        .max_by_key(|h| *h.number())
        .ok_or_else(|| anyhow!("Unknown headers can't be empty!"))?;

    // this is illegal
    if target.hash() != finality_proof.block {
        Err(anyhow!("Latest finalized block should be highest block in unknown_headers"))?;
    }

    let justification = GrandpaJustification::<H>::decode(&mut &finality_proof.justification[..])
        .map_err(|e| anyhow!("Failed to decode justificatio {:?}", e))?;

    if justification.commit.target_hash != finality_proof.block {
        Err(anyhow!("Justification target hash and finality proof block hash mismatch"))?;
    }

    let from = consensus_state.latest_hash;

    let base = finality_proof
        .unknown_headers
        .iter()
        .min_by_key(|h| *h.number())
        .ok_or_else(|| anyhow!("Unknown headers can't be empty!"))?;

    if base.number() < &consensus_state.latest_height {
        headers.ancestry(base.hash(), consensus_state.latest_hash).map_err(|_| {
            anyhow!(
                "[verify_grandpa_finality_proof] Invalid ancestry (base -> latest relay block)!"
            )
        })?;
    }

    let mut finalized = headers
        .ancestry(from, target.hash())
        .map_err(|_| anyhow!("[verify_grandpa_finality_proof] Invalid ancestry!"))?;
    finalized.sort();

    // 2. verify justification.
    justification.verify(consensus_state.current_set_id, &consensus_state.current_authorities)?;

    // Sets new consensus state, optionally rotating authorities
    consensus_state.latest_hash = target.hash();
    consensus_state.latest_height = (*target.number()).into();
    if let Some(scheduled_change) = find_scheduled_change::<H>(&target) {
        consensus_state.current_set_id += 1;
        consensus_state.current_authorities = scheduled_change.next_authorities;
    }

    Ok((consensus_state, target.clone(), finalized, headers))
}
/// This function verifies the GRANDPA finality proof for relay chain headers.
///
/// Next, we prove the finality of parachain headers, by verifying patricia-merkle trie state proofs
/// of these headers, stored at the recently finalized relay chain heights.
/// Returns the new Consensus state alongside a map of para id to a vector that contains a tuple of
/// finalized parachain header and timestamp
pub fn verify_parachain_headers_with_grandpa_finality_proof<H>(
    consensus_state: ConsensusState,
    proof: ParachainHeadersWithFinalityProof<H>,
) -> Result<(ConsensusState, BTreeMap<u32, Vec<H>>), anyhow::Error>
where
    H: Header<Hash = H256, Number = u32>,
    H::Number: finality_grandpa::BlockNumberOps + Into<u32>,
{
    let ParachainHeadersWithFinalityProof { finality_proof, parachain_headers } = proof;

    let (consensus_state, _, finalized_hashes, headers) =
        verify_grandpa_finality_proof(consensus_state, finality_proof)?;
    // verifies state proofs of parachain headers in finalized relay chain headers.
    let mut verified_parachain_headers: BTreeMap<u32, Vec<H>> = BTreeMap::new();
    for (hash, proof) in parachain_headers {
        if finalized_hashes.binary_search(&hash).is_err() {
            // seems relay hash isn't in the finalized chain.
            continue
        }
        let relay_chain_header =
            headers.header(&hash).expect("Headers have been checked by AncestryChain; qed");
        let state_proof = proof.state_proof;
        let mut keys = BTreeMap::new();
        for para_id in proof.para_ids {
            // ensure the para id is in the consensus state before proof verification
            if !consensus_state.para_ids.contains_key(&para_id) {
                continue
            }

            let key = parachain_header_storage_key(para_id);

            keys.insert(key.0, para_id);
        }

        let proof = StorageProof::new(state_proof);

        // verify patricia-merkle state proofs
        let mut result = read_proof_check::<BlakeTwo256, _>(
            relay_chain_header.state_root(),
            proof,
            keys.keys().map(|key| key.as_slice()),
        )
        .map_err(|err| anyhow!("error verifying parachain header state proof: {err:?}"))?;
        for (key, para_id) in keys {
            let header = result
                .remove(&key)
                .flatten()
                .ok_or_else(|| anyhow!("Invalid proof, parachain header not found"))?;
            let parachain_header =
                H::decode(&mut &header[..]).map_err(|e| anyhow!("error decoding header: {e:?}"))?;
            verified_parachain_headers.entry(para_id).or_default().push(parachain_header);
        }
    }

    Ok((consensus_state, verified_parachain_headers))
}
