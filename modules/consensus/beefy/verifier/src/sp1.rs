// Copyright (C) Polytope Labs Ltd.
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

//! SP1 BEEFY proof verification

use crate::error::Error;
use alloc::vec::Vec;
use alloy_sol_types::{
	SolValue,
	private::{FixedBytes, U256},
	sol,
};
use beefy_verifier_primitives::{ConsensusState, ParachainHeader, Sp1BeefyProof};
use codec::Encode;
use ismp::messaging::Keccak256;

// Matches `PublicInputs` and `ParachainHeaderHash` in evm/src/consensus/Types.sol
sol! {
	struct ParachainHeaderHash {
		uint256 id;
		bytes32 hash;
	}

	struct PublicInputs {
		bytes32 authorities_root;
		uint256 authorities_len;
		bytes32 leaf_hash;
		uint256 block_number;
		ParachainHeaderHash[] headers;
		bytes32 nonce;
	}
}

/// Verify an SP1 BEEFY consensus proof and return the updated consensus state
/// and verified parachain headers. Mirrors the Solidity `SP1Beefy.verifyConsensus` flow:
/// SP1 proves authority-set membership, commitment signatures, MMR leaf inclusion and
/// parachain header inclusion — so no additional merkle verification is done here.
///
/// SP1 proves only that the leaf is *in* the mmr, not that it is the *latest* leaf, so leaf
/// freshness (`parent_number + 1 == block_number`) is enforced here. Keep this in step with
/// the equivalent check in `SP1Beefy.verifyConsensus`.
pub fn verify_sp1_consensus<H: Keccak256 + Send + Sync>(
	trusted_state: ConsensusState,
	proof: Sp1BeefyProof,
	vkey: &str,
) -> Result<(Vec<u8>, Vec<ParachainHeader>), Error> {
	if trusted_state.latest_beefy_height >= proof.block_number {
		Err(Error::StaleHeight {
			trusted_height: trusted_state.latest_beefy_height,
			current_height: proof.block_number,
		})?;
	}

	// An mmr is append-only, so a historical leaf also proves against the commitment's root.
	// Accepting one would advance `latest_beefy_height` while replaying an old leaf, suppressing
	// the rotation below and stranding the client on a set the relay chain has retired.
	// `parent_number` is part of the leaf preimage hashed into `leaf_hash`, so pinning it here
	// pins the leaf itself.
	let parent_number = proof.mmr_leaf.parent_number_and_hash.0;
	if parent_number.saturating_add(1) != proof.block_number {
		Err(Error::StaleMmrLeaf { parent_number, block_number: proof.block_number })?;
	}

	let authority = if proof.validator_set_id == trusted_state.next_authorities.id {
		&trusted_state.next_authorities
	} else if proof.validator_set_id == trusted_state.current_authorities.id {
		&trusted_state.current_authorities
	} else {
		Err(Error::UnknownAuthoritySet { id: proof.validator_set_id })?
	};

	let headers = proof
		.headers
		.iter()
		.map(|h| ParachainHeaderHash {
			id: U256::from(h.para_id),
			hash: FixedBytes::from(Into::<[u8; 32]>::into(H::keccak256(&h.header))),
		})
		.collect();

	let public_inputs = PublicInputs {
		authorities_root: FixedBytes::from(Into::<[u8; 32]>::into(authority.keyset_commitment)),
		authorities_len: U256::from(authority.len),
		leaf_hash: FixedBytes::from(Into::<[u8; 32]>::into(H::keccak256(&proof.mmr_leaf.encode()))),
		block_number: U256::from(proof.block_number),
		headers,
		nonce: FixedBytes::from(proof.nonce.0),
	}
	.abi_encode();

	sp1_verifier::Groth16Verifier::verify(
		&proof.proof,
		&public_inputs,
		vkey,
		sp1_verifier::GROTH16_VK_BYTES,
	)
	.map_err(|_| Error::Sp1VerificationFailed)?;

	let mut new_state = trusted_state;
	if proof.mmr_leaf.beefy_next_authority_set.id > new_state.next_authorities.id {
		new_state.current_authorities = new_state.next_authorities.clone();
		new_state.next_authorities = proof.mmr_leaf.beefy_next_authority_set.clone();
	}
	new_state.latest_beefy_height = proof.block_number;

	Ok((new_state.encode(), proof.headers))
}
