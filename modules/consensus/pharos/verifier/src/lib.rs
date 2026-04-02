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
use geth_primitives::Header;
use ismp::messaging::Keccak256;
use pharos_primitives::{
	BlockProof, BlsPublicKey, Config, ValidatorSet, VerifierState, VerifierStateUpdate,
};
use primitive_types::H256;

/// Domain Separation Tag for Pharos BLS signatures.
pub const PHAROS_BLS_DST: &str = "BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

/// Verifies a Pharos block proof and update the verifier state.
pub fn verify_pharos_block<C: Config, H: Keccak256 + Send + Sync>(
	trusted_state: VerifierState,
	update: VerifierStateUpdate,
) -> Result<VerifierState, Error> {
	let update_block_number = update.block_number();
	let current_block_number = trusted_state.finalized_block_number;

	if update_block_number <= current_block_number {
		return Err(Error::StaleUpdate {
			current: current_block_number,
			update: update_block_number,
		});
	}

	let update_epoch = C::compute_epoch(update_block_number);
	if update_epoch != trusted_state.current_epoch {
		return Err(Error::EpochMismatch {
			update_epoch,
			expected_epoch: trusted_state.current_epoch,
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

	let computed_hash = Header::from(&update.header).hash::<H>();

	if computed_hash != update.block_proof.block_proof_hash {
		return Err(Error::BlockProofHashMismatch {
			computed: computed_hash,
			provided: update.block_proof.block_proof_hash,
		});
	}

	verify_bls_signature(
		&update.block_proof.participant_keys,
		&update.block_proof,
		update.block_proof.block_proof_hash,
	)?;

	let new_state = if C::is_epoch_boundary(update_block_number) {
		// Epoch boundary block must always have validator set proof
		let validator_set_proof = update
			.validator_set_proof
			.ok_or(Error::MissingValidatorSetProof { block_number: update_block_number })?;

		let next_epoch = C::compute_epoch(update_block_number) + 1;
		let new_validator_set = state_proof::verify_validator_set_proof::<H>(
			update.header.state_root,
			&validator_set_proof,
			next_epoch,
		)?;

		VerifierState {
			current_validator_set: new_validator_set,
			finalized_block_number: update_block_number,
			finalized_hash: computed_hash,
			current_epoch: next_epoch,
		}
	} else {
		if update.validator_set_proof.is_some() {
			return Err(Error::UnexpectedValidatorSetProof { block_number: update_block_number });
		}

		VerifierState {
			finalized_block_number: update_block_number,
			finalized_hash: computed_hash,
			..trusted_state
		}
	};

	Ok(new_state)
}

/// Verify that all participating validators are members of the trusted validator set.
fn verify_validator_membership(
	validator_set: &ValidatorSet,
	participants: &[BlsPublicKey],
) -> Result<(), Error> {
	let deduped: alloc::collections::BTreeSet<&[u8]> =
		participants.iter().map(|k| k.as_ref()).collect();
	if deduped.len() != participants.len() {
		return Err(Error::DuplicateParticipant);
	}
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
	let required = (total_stake * 2 / 3) + 1;

	if participating_stake >= required {
		Ok(())
	} else {
		Err(Error::InsufficientStake {
			participating: participating_stake,
			required,
			total: total_stake,
		})
	}
}

/// Verify the BLS aggregate signature.
fn verify_bls_signature(
	participants: &[BlsPublicKey],
	block_proof: &BlockProof,
	block_proof_hash: H256,
) -> Result<(), Error> {
	if participants.is_empty() {
		return Err(Error::NoParticipants);
	}

	let aggregate_pubkey = crypto_utils::aggregate_public_keys(participants);

	// The message signed is the block_proof_hash
	let message = block_proof_hash.as_bytes().to_vec();

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

#[cfg(test)]
mod tests {
	use super::*;
	use pharos_primitives::{ValidatorInfo, ValidatorSet};
	use primitive_types::U256;

	fn make_key(byte: u8) -> BlsPublicKey {
		let mut data = [0u8; 48];
		data[0] = byte;
		BlsPublicKey::try_from(data.as_slice()).unwrap()
	}

	fn make_validator_set(keys: &[BlsPublicKey]) -> ValidatorSet {
		let mut set = ValidatorSet::new(1);
		for key in keys {
			set.add_validator(ValidatorInfo {
				bls_public_key: key.clone(),
				pool_id: Default::default(),
				stake: U256::from(1000),
			});
		}
		set
	}

	#[test]
	fn test_duplicate_participant_keys_rejected() {
		let key_a = make_key(1);
		let key_b = make_key(2);
		let set = make_validator_set(&[key_a.clone(), key_b.clone()]);

		// No duplicates: OK
		assert!(verify_validator_membership(&set, &[key_a.clone(), key_b.clone()]).is_ok());

		// Duplicate key: rejected
		let result = verify_validator_membership(&set, &[key_a.clone(), key_a.clone()]);
		assert!(matches!(result, Err(Error::DuplicateParticipant)));
	}

	#[test]
	fn test_unknown_participant_rejected() {
		let key_a = make_key(1);
		let key_unknown = make_key(99);
		let set = make_validator_set(&[key_a.clone()]);

		let result = verify_validator_membership(&set, &[key_unknown]);
		assert!(matches!(result, Err(Error::UnknownValidator { .. })));
	}

	#[test]
	fn test_stake_threshold() {
		let keys: Vec<BlsPublicKey> = (1..=10).map(make_key).collect();
		let set = make_validator_set(&keys); // 10 validators, 1000 each, total 10000

		// 7 out of 10 (7000 stake) > 2/3 + 1 (6668): passes
		assert!(verify_stake_threshold(&set, &keys[..7]).is_ok());

		// 6 out of 10 (6000 stake) < 6668: fails
		assert!(matches!(
			verify_stake_threshold(&set, &keys[..6]),
			Err(Error::InsufficientStake { .. })
		));
	}
}
