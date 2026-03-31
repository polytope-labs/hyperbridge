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

//! SP1 BEEFY proof verifier
use alloc::vec::Vec;
use codec::{Decode, Encode};

use crate::types::{BeefyAuthoritySet, BeefyConsensusState};

/// Decoded SP1 BEEFY proof
#[derive(Debug, Clone, Encode, Decode)]
pub struct Sp1BeefyProof {
	pub commitment: MiniCommitment,
	pub mmr_leaf: PartialBeefyMmrLeaf,
	pub headers: Vec<ParachainHeader>,
	pub proof: Vec<u8>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct MiniCommitment {
	pub block_number: u32,
	pub validator_set_id: u64,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct PartialBeefyMmrLeaf {
	pub version: u32,
	pub parent_number: u32,
	pub parent_hash: [u8; 32],
	pub next_authority_set: AuthoritySetCommitment,
	pub extra: [u8; 32],
	pub k_index: u32,
	pub leaf_index: u32,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct AuthoritySetCommitment {
	pub id: u64,
	pub len: u32,
	pub root: [u8; 32],
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ParachainHeader {
	pub id: u32,
	pub header: Vec<u8>,
}

/// Result of verifying a BEEFY proof, the updated consensus state
pub struct VerificationResult {
	pub new_state: BeefyConsensusState,
}

/// Errors from proof verification
#[derive(Debug)]
pub enum VerificationError {
	DecodeFailed,
	StaleHeight,
	UnknownAuthoritySet,
	InvalidProof,
}

impl core::fmt::Display for VerificationError {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		match self {
			Self::DecodeFailed => write!(f, "Failed to decode proof"),
			Self::StaleHeight => write!(f, "Proof height is stale"),
			Self::UnknownAuthoritySet => write!(f, "Unknown authority set"),
			Self::InvalidProof => write!(f, "SP1 proof verification failed"),
		}
	}
}

pub fn verify_beefy_proof(
	trusted_state: &BeefyConsensusState,
	raw_proof: &[u8],
	vkey_hash: &str,
) -> Result<VerificationResult, VerificationError> {
	let proof =
		Sp1BeefyProof::decode(&mut &raw_proof[..]).map_err(|_| VerificationError::DecodeFailed)?;

	if trusted_state.latest_height >= proof.commitment.block_number {
		return Err(VerificationError::StaleHeight);
	}

	let authority = if proof.commitment.validator_set_id == trusted_state.next_authority_set.id {
		&trusted_state.next_authority_set
	} else if proof.commitment.validator_set_id == trusted_state.current_authority_set.id {
		&trusted_state.current_authority_set
	} else {
		return Err(VerificationError::UnknownAuthoritySet);
	};

	let public_inputs = build_public_inputs(&proof, authority.root, authority.len);

	#[cfg(feature = "sp1")]
	{
		sp1_verifier::PlonkVerifier::verify(
			&proof.proof,
			&public_inputs,
			vkey_hash,
			&sp1_verifier::PLONK_VK_BYTES,
		)
		.map_err(|_| VerificationError::InvalidProof)?;
	}

	let mut new_state = trusted_state.clone();

	if proof.mmr_leaf.next_authority_set.id > trusted_state.next_authority_set.id {
		new_state.current_authority_set = trusted_state.next_authority_set.clone();
		new_state.next_authority_set = BeefyAuthoritySet {
			id: proof.mmr_leaf.next_authority_set.id,
			len: proof.mmr_leaf.next_authority_set.len,
			root: proof.mmr_leaf.next_authority_set.root,
		};
	}

	new_state.latest_height = proof.commitment.block_number;

	Ok(VerificationResult { new_state })
}

fn build_public_inputs(
	proof: &Sp1BeefyProof,
	authority_root: [u8; 32],
	authority_len: u32,
) -> Vec<u8> {
	let leaf_hash = polkadot_sdk::sp_io::hashing::keccak_256(&proof.mmr_leaf.encode());

	let headers: Vec<[u8; 32]> = proof
		.headers
		.iter()
		.map(|h| polkadot_sdk::sp_io::hashing::keccak_256(&h.header))
		.collect();

	let mut encoded = Vec::new();
	encoded.extend_from_slice(&authority_root);
	encoded.extend_from_slice(&{
		let mut buf = [0u8; 32];
		buf[28..32].copy_from_slice(&authority_len.to_be_bytes());
		buf
	});
	encoded.extend_from_slice(&leaf_hash);
	headers.iter().for_each(|h| encoded.extend_from_slice(h));

	encoded
}
