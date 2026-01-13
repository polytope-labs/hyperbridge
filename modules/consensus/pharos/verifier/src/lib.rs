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

//! Pharos consensus verifier.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod error;
pub mod state_proof;

use error::Error;
use geth_primitives::{CodecHeader, Header};
use ismp::messaging::Keccak256;
use pharos_primitives::{BlockProof, Config, ValidatorSet, VerifierState, VerifierStateUpdate};
use primitive_types::H256;
use sync_committee_primitives::constants::BlsPublicKey;

/// Domain Separation Tag for Pharos BLS signatures.
pub const PHAROS_BLS_DST: &str = "BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

/// Verifies a Pharos block proof and update the verifier state.
pub fn verify_pharos_block<C: Config, H: Keccak256 + Send + Sync>(
	trusted_state: VerifierState,
	update: VerifierStateUpdate,
) -> Result<VerifierState, Error> {
	let update_block_number = update.block_number();
	let current_block_number = trusted_state.finalized_block_number();

	if update_block_number <= current_block_number {
		return Err(Error::StaleUpdate {
			current: current_block_number,
			update: update_block_number,
		});
	}

	verify_validator_membership(
		&trusted_state.current_validator_set,
		&update.block_proof.participant_keys,
	)?;

	verify_stake_threshold(
		&trusted_state.current_validator_set,
		&update.block_proof.participant_keys,
	)?;

	verify_bls_signature(&update.block_proof.participant_keys, &update.block_proof)?;

	let computed_hash = compute_header_hash::<H>(&update.header);
	if computed_hash != update.block_proof.block_hash {
		return Err(Error::HeaderHashMismatch {
			expected: update.block_proof.block_hash,
			actual: computed_hash,
		});
	}

	let new_validator_set = if C::is_epoch_boundary(update_block_number) {
		// Epoch boundary block must always have validator set proof
		let validator_set_proof = update
			.validator_set_proof
			.ok_or(Error::MissingValidatorSetProof { block_number: update_block_number })?;

		state_proof::verify_validator_set_proof::<C, H>(
			update.header.state_root,
			&validator_set_proof,
		)?;

		Some(validator_set_proof.validator_set)
	} else {
		// If not an epoch boundary, then, no validator set update expected
		if update.validator_set_proof.is_some() {
			return Err(Error::UnexpectedValidatorSetProof { block_number: update_block_number });
		}
		None
	};

	let new_state = if let Some(new_set) = new_validator_set.clone() {
		VerifierState {
			current_validator_set: trusted_state
				.next_validator_set
				.unwrap_or(trusted_state.current_validator_set),
			next_validator_set: Some(new_set),
			finalized_header: update.header,
			finalized_hash: computed_hash,
			current_epoch: C::compute_epoch(update_block_number) + 1,
		}
	} else {
		VerifierState {
			finalized_header: update.header,
			finalized_hash: computed_hash,
			..trusted_state
		}
	};

	Ok(new_state)
}

/// Compute the hash of a block header using RLP encoding and Keccak256.
///
/// This follows the standard Ethereum header hash computation.
pub fn compute_header_hash<H: Keccak256>(header: &CodecHeader) -> H256 {
	let rlp_header = Header::from(header);
	let encoding = alloy_rlp::encode(&rlp_header);
	H::keccak256(&encoding)
}

/// Verify that all participating validators are members of the trusted validator set.
fn verify_validator_membership(
	validator_set: &ValidatorSet,
	participants: &[BlsPublicKey],
) -> Result<(), Error> {
	if let Some(key) = participants.iter().find(|key| !validator_set.contains(key)) {
		return Err(Error::UnknownValidator { key: key.clone() });
	}
	Ok(())
}

/// Verify that participating validators have more than 2/3 of total stake.
fn verify_stake_threshold(
	validator_set: &ValidatorSet,
	participants: &[BlsPublicKey],
) -> Result<(), Error> {
	let participating_stake = validator_set.participating_stake(participants);
	let total_stake = validator_set.total_stake;

	// participating_stake > (2 * total_stake) / 3
	// participating_stake * 3 > 2 * total_stake
	let participating_times_3 = participating_stake.saturating_mul(primitive_types::U256::from(3));
	let total_times_2 = total_stake.saturating_mul(primitive_types::U256::from(2));

	if participating_times_3 <= total_times_2 {
		return Err(Error::InsufficientStake {
			participating: participating_stake,
			required: (total_stake * 2 / 3) + 1,
			total: total_stake,
		});
	}

	Ok(())
}

/// Verify the BLS aggregate signature.
fn verify_bls_signature(
	participants: &[BlsPublicKey],
	block_proof: &BlockProof,
) -> Result<(), Error> {
	if participants.is_empty() {
		return Err(Error::NoParticipants);
	}

	let aggregate_pubkey = bsc_verifier::aggregate_public_keys(participants);

	// The message signed is the block_proof_hash
	let message = block_proof.block_proof_hash.as_bytes().to_vec();

	let is_valid = bls::verify(
		&aggregate_pubkey,
		&message,
		&block_proof.aggregate_signature,
		&PHAROS_BLS_DST.as_bytes().to_vec(),
	);

	if !is_valid {
		return Err(Error::InvalidSignature);
	}

	Ok(())
}
