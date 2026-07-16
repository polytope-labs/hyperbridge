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

//! Arc consensus verifier.
//!
//! Verifies Malachite commit certificates: per-validator ed25519 signatures
//! over SSZ-encoded precommits for the execution block hash, requiring more
//! than 2/3 of the trusted validator set's voting power. After a certificate
//! is accepted, the active validator set is read from the finalized header's
//! state root (an EIP-1186 proof of the ValidatorRegistry contract) and
//! adopted as the new trusted set — so adopting a new set always requires the
//! existing set's signatures on the block that installs it.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod error;

use alloc::{collections::BTreeSet, format, vec::Vec};
use arc_primitives::{
	active_set_element_slot, active_set_length_slot, derive_address, precommit_sign_bytes,
	validator_slots, CommitCertificate, Validator, ValidatorSet, ValidatorSetProof, VerifierState,
	VerifierStateUpdate, PUBLIC_KEY_HEADER_VALUE, VALIDATOR_REGISTRY_ADDRESS,
	VALIDATOR_STATUS_ACTIVE,
};
use curve25519_dalek::edwards::CompressedEdwardsY;
use error::Error;
use evm_state_machine::{get_contract_account, get_values_from_proof};
use geth_primitives::Header;
use ismp::messaging::Keccak256;
use polkadot_sdk::{sp_core::ed25519, sp_io};
use primitive_types::{H256, U256};

/// Verifies an Arc block update and returns the new trusted state.
///
/// The commit certificate is checked against the *trusted* validator set;
/// the update then re-proves the active set at the new height, which becomes
/// the trusted set for subsequent updates. Since Arc rotates its set through
/// ordinary contract state changes (no epochs), a certificate signed by a set
/// the client never adopted will fail — updates must be submitted at or
/// before each rotation boundary.
pub fn verify_arc_update<H: Keccak256 + Send + Sync>(
	trusted_state: VerifierState,
	update: VerifierStateUpdate,
) -> Result<VerifierState, Error> {
	let update_height = update.certificate.height;
	if update_height <= trusted_state.finalized_height {
		return Err(Error::StaleUpdate {
			current: trusted_state.finalized_height,
			update: update_height,
		});
	}

	let header_number = update.header.number.low_u64();
	if update_height != header_number {
		return Err(Error::HeightMismatch { certificate: update_height, header: header_number });
	}

	let computed_hash = Header::from(&update.header).hash::<H>();
	if computed_hash != update.certificate.block_hash {
		return Err(Error::BlockHashMismatch {
			certificate: update.certificate.block_hash,
			computed: computed_hash,
		});
	}

	verify_certificate(&trusted_state.current_validators, &update.certificate)?;

	let current_validators =
		extract_validator_set::<H>(update.header.state_root, &update.validator_set_proof)?;

	Ok(VerifierState {
		current_validators,
		finalized_height: update_height,
		finalized_hash: computed_hash,
	})
}

/// Verify a commit certificate against a trusted validator set, mirroring
/// Malachite's `verify_commit_certificate`: every signature must be from a
/// distinct, known validator and valid over the reconstructed precommit
/// sign-bytes, and the signers must hold strictly more than 2/3 of the total
/// voting power.
pub fn verify_certificate(
	validators: &ValidatorSet,
	certificate: &CommitCertificate,
) -> Result<(), Error> {
	let mut seen = BTreeSet::new();
	let mut signed_power = 0u64;

	for commit_signature in &certificate.commit_signatures {
		let address = commit_signature.address;
		if !seen.insert(address) {
			return Err(Error::DuplicateVote { address });
		}
		let validator = validators.get(&address).ok_or(Error::UnknownValidator { address })?;

		let message = precommit_sign_bytes(
			certificate.height,
			certificate.round,
			&certificate.block_hash,
			&address,
		);
		let signature = ed25519::Signature::from_raw(commit_signature.signature);
		let public_key = ed25519::Public::from_raw(validator.public_key.0);
		if !sp_io::crypto::ed25519_verify(&signature, &message, &public_key) {
			return Err(Error::InvalidSignature { address });
		}

		signed_power = signed_power.saturating_add(validator.voting_power);
	}

	if !validators.has_quorum(signed_power) {
		return Err(Error::InsufficientVotingPower {
			signed: signed_power,
			total: validators.total_voting_power,
		});
	}

	Ok(())
}

/// Verify the ValidatorRegistry account proof against `state_root` and return
/// the contract's storage root.
///
/// Also used by provers to discover which block a `"latest"`-anchored
/// `eth_getProof` response actually proves: the account proof only verifies
/// against the state root of its anchor block.
pub fn registry_storage_root<H: Keccak256 + Send + Sync>(
	state_root: H256,
	account_proof: Vec<Vec<u8>>,
) -> Result<H256, Error> {
	let account =
		get_contract_account::<H>(account_proof, VALIDATOR_REGISTRY_ADDRESS.as_bytes(), state_root)
			.map_err(|e| Error::ValidatorSetProof(format!("account proof: {e:?}")))?;
	Ok(H256(account.storage_root.0))
}

/// Reconstruct the active validator set from an EIP-1186 proof of the
/// ValidatorRegistry contract against `state_root`.
///
/// Mirrors the filtering the Arc node applies to `getActiveValidatorSet()`:
/// only validators with `status == Active`, non-zero voting power and a
/// well-formed 32-byte public key are included. Slots proven absent decode as
/// zero, exactly as an `eth_call` would read them.
pub fn extract_validator_set<H: Keccak256 + Send + Sync>(
	state_root: H256,
	proof: &ValidatorSetProof,
) -> Result<ValidatorSet, Error> {
	let storage_root = registry_storage_root::<H>(state_root, proof.account_proof.clone())?;

	let length = {
		let values = read_slots::<H>(storage_root, proof, &[active_set_length_slot()])?;
		word_to_u64(values[0].unwrap_or_default())
			.ok_or_else(|| Error::ValidatorSetProof("invalid active set length".into()))?
	};
	if length == 0 {
		return Err(Error::InvalidValidatorSet("active validator set is empty".into()));
	}

	let element_slots: Vec<H256> = (0..length).map(active_set_element_slot::<H>).collect();
	let registration_ids = read_slots::<H>(storage_root, proof, &element_slots)?
		.into_iter()
		.enumerate()
		.map(|(i, value)| {
			value.ok_or_else(|| Error::ValidatorSetProof(format!("missing active set element {i}")))
		})
		.collect::<Result<Vec<_>, _>>()?;

	// [status, public key header, public key data, voting power] per validator
	let record_slots: Vec<H256> = registration_ids
		.iter()
		.flat_map(|id| {
			let slots = validator_slots::<H>(*id);
			[slots.status, slots.public_key_header, slots.public_key_data, slots.voting_power]
		})
		.collect();
	let records = read_slots::<H>(storage_root, proof, &record_slots)?;

	let mut validators = Vec::new();
	for record in records.chunks_exact(4) {
		let status = record[0].unwrap_or_default().0[31];
		let public_key_header = word_to_u64(record[1].unwrap_or_default());
		let voting_power = word_to_u64(record[3].unwrap_or_default())
			.ok_or_else(|| Error::ValidatorSetProof("voting power exceeds u64".into()))?;

		if status != VALIDATOR_STATUS_ACTIVE || voting_power == 0 {
			continue;
		}
		// The registry enforces 32-byte keys at registration; skip anything
		// else the same way the Arc node skips malformed keys.
		if public_key_header != Some(PUBLIC_KEY_HEADER_VALUE) {
			continue;
		}
		// An absent data slot reads as zero, exactly as `eth_call` would.
		let public_key = record[2].unwrap_or_default();
		// Mirror the node's `PublicKey::from_bytes`: keys that don't
		// decompress to a curve point are skipped and their power excluded
		// from the quorum denominator.
		if CompressedEdwardsY(public_key.0).decompress().is_none() {
			continue;
		}

		validators.push(Validator::new::<H>(public_key, voting_power));
	}

	ValidatorSet::new(validators).ok_or_else(|| {
		Error::InvalidValidatorSet(
			"no active validators with valid public keys, or voting power overflow".into(),
		)
	})
}

/// Read the given storage slots from the proof, decoding each present value
/// to a left-padded 32-byte word.
fn read_slots<H: Keccak256 + Send + Sync>(
	storage_root: H256,
	proof: &ValidatorSetProof,
	slots: &[H256],
) -> Result<Vec<Option<H256>>, Error> {
	let keys = slots.iter().map(|slot| H::keccak256(slot.as_bytes()).0.to_vec()).collect();
	let values = get_values_from_proof::<H>(keys, storage_root, proof.storage_proof.clone())
		.map_err(|e| Error::ValidatorSetProof(format!("storage proof: {e:?}")))?;

	values
		.into_iter()
		.map(|value| value.map(|raw| decode_storage_value(&raw)).transpose())
		.collect()
}

/// Decode an RLP-encoded storage trie value to a left-padded 32-byte word.
fn decode_storage_value(raw: &[u8]) -> Result<H256, Error> {
	let mut buf = raw;
	let data = alloy_rlp::Header::decode_bytes(&mut buf, false)
		.map_err(|e| Error::ValidatorSetProof(format!("storage value rlp: {e:?}")))?;
	if data.len() > 32 {
		return Err(Error::ValidatorSetProof("storage value exceeds 32 bytes".into()));
	}
	let mut word = [0u8; 32];
	word[32 - data.len()..].copy_from_slice(data);
	Ok(H256(word))
}

/// Interpret a 32-byte storage word as a u64, rejecting larger values.
fn word_to_u64(word: H256) -> Option<u64> {
	let value = U256::from_big_endian(word.as_bytes());
	if value > U256::from(u64::MAX) {
		return None;
	}
	Some(value.low_u64())
}

/// Derive the consensus address for an ed25519 public key using the runtime's
/// keccak host function. Re-exported for provers and tests.
pub fn validator_address<H: Keccak256>(public_key: &H256) -> primitive_types::H160 {
	derive_address::<H>(public_key)
}

#[cfg(test)]
mod tests {
	use super::*;
	use arc_primitives::{CommitSignature, Validator};
	use polkadot_sdk::sp_core::{ed25519, Pair};

	struct Hasher;

	impl Keccak256 for Hasher {
		fn keccak256(bytes: &[u8]) -> H256 {
			sp_io::hashing::keccak_256(bytes).into()
		}
	}

	fn validator(seed: u8, voting_power: u64) -> (ed25519::Pair, Validator) {
		let pair = ed25519::Pair::from_seed(&[seed; 32]);
		let validator = Validator::new::<Hasher>(H256(pair.public().0), voting_power);
		(pair, validator)
	}

	fn sign(pair: &ed25519::Pair, certificate: &CommitCertificate) -> CommitSignature {
		let address = derive_address::<Hasher>(&H256(pair.public().0));
		let message = precommit_sign_bytes(
			certificate.height,
			certificate.round,
			&certificate.block_hash,
			&address,
		);
		CommitSignature { address, signature: pair.sign(&message).0 }
	}

	fn certificate() -> CommitCertificate {
		CommitCertificate {
			height: 42,
			round: 1,
			block_hash: H256::repeat_byte(0xAB),
			commit_signatures: Vec::new(),
		}
	}

	#[test]
	fn accepts_a_quorum_of_valid_signatures() {
		let (pairs, validators): (Vec<_>, Vec<_>) =
			(1..=4u8).map(|seed| validator(seed, 1)).unzip();
		let set = ValidatorSet::new(validators).unwrap();

		let mut cert = certificate();
		cert.commit_signatures = pairs.iter().take(3).map(|pair| sign(pair, &cert)).collect();

		// 3 of 4: 9 > 8
		assert!(verify_certificate(&set, &cert).is_ok());
	}

	#[test]
	fn rejects_exactly_two_thirds() {
		let (pairs, validators): (Vec<_>, Vec<_>) =
			(1..=3u8).map(|seed| validator(seed, 1)).unzip();
		let set = ValidatorSet::new(validators).unwrap();

		let mut cert = certificate();
		cert.commit_signatures = pairs.iter().take(2).map(|pair| sign(pair, &cert)).collect();

		// Malachite's quorum rule is strict: 2 of 3 is 6 > 6, not met.
		assert!(matches!(
			verify_certificate(&set, &cert),
			Err(Error::InsufficientVotingPower { signed: 2, total: 3 })
		));
	}

	#[test]
	fn rejects_duplicate_votes() {
		let (pairs, validators): (Vec<_>, Vec<_>) =
			(1..=3u8).map(|seed| validator(seed, 1)).unzip();
		let set = ValidatorSet::new(validators).unwrap();

		let mut cert = certificate();
		let signature = sign(&pairs[0], &cert);
		cert.commit_signatures = alloc::vec![signature.clone(), signature];

		assert!(matches!(verify_certificate(&set, &cert), Err(Error::DuplicateVote { .. })));
	}

	#[test]
	fn rejects_unknown_validators() {
		let (pairs, validators): (Vec<_>, Vec<_>) =
			(1..=3u8).map(|seed| validator(seed, 1)).unzip();
		let set = ValidatorSet::new(validators.into_iter().skip(1).collect::<Vec<_>>()).unwrap();

		let mut cert = certificate();
		cert.commit_signatures = alloc::vec![sign(&pairs[0], &cert)];

		assert!(matches!(verify_certificate(&set, &cert), Err(Error::UnknownValidator { .. })));
	}

	#[test]
	fn rejects_signatures_over_a_different_block_hash() {
		let (pairs, validators): (Vec<_>, Vec<_>) =
			(1..=3u8).map(|seed| validator(seed, 1)).unzip();
		let set = ValidatorSet::new(validators).unwrap();

		let mut cert = certificate();
		cert.commit_signatures = pairs.iter().map(|pair| sign(pair, &cert)).collect();
		cert.block_hash = H256::repeat_byte(0xCD);

		assert!(matches!(verify_certificate(&set, &cert), Err(Error::InvalidSignature { .. })));
	}

	#[test]
	fn weighs_quorum_by_voting_power() {
		let (heavy_pair, heavy) = validator(1, 10);
		let (_, light_a) = validator(2, 1);
		let (_, light_b) = validator(3, 1);
		let set = ValidatorSet::new(alloc::vec![heavy, light_a, light_b]).unwrap();

		let mut cert = certificate();
		cert.commit_signatures = alloc::vec![sign(&heavy_pair, &cert)];

		// 10 of 12: 30 > 24
		assert!(verify_certificate(&set, &cert).is_ok());
	}
}
