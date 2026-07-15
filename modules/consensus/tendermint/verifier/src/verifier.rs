use core::time::Duration;
use prost::alloc::{collections::BTreeSet, format, string::ToString};

use cometbft::{block::Height, chain::Id, trust_threshold::TrustThresholdFraction, Hash, Time};
use cometbft_light_client_verifier::{
	options::Options,
	types::{TrustedBlockState, UntrustedBlockState, ValidatorSet},
	Verdict, Verifier,
};
use cometbft_proto::google::protobuf::Timestamp;

use crate::SpIoVerifier;

use tendermint_primitives::{
	ConsensusProof, TrustedState, UpdatedTrustedState, VerificationError, VerificationOptions,
};

use crate::sp_io_verifier::validate_validator_set_hash;

/// Main verification function for header updates
pub fn verify_header_update(
	trusted_state: TrustedState,
	consensus_proof: ConsensusProof,
	current_time: u64,
) -> Result<UpdatedTrustedState, VerificationError> {
	consensus_proof.validate().map_err(|e| VerificationError::Invalid(e))?;

	let chain_id = Id::try_from(trusted_state.chain_id.clone())
		.map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let height = Height::try_from(trusted_state.height)
		.map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let timestamp = Timestamp { seconds: trusted_state.timestamp as i64, nanos: 0 };
	let time = Time::try_from(timestamp).map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let next_validators = ValidatorSet::new(trusted_state.next_validators.clone(), None);
	let next_validators_hash = Hash::Sha256(trusted_state.next_validators_hash);

	let tendermint_trusted_state = TrustedBlockState {
		chain_id: &chain_id,
		header_time: time,
		height,
		next_validators: &next_validators,
		next_validators_hash,
	};

	let validators = extract_validators(&trusted_state, &consensus_proof)?;
	let next_validators = consensus_proof
		.next_validators
		.as_ref()
		.map(|validators| ValidatorSet::new(validators.clone(), None));

	let untrusted_block_state = UntrustedBlockState {
		signed_header: &consensus_proof.signed_header,
		validators: &validators,
		next_validators: next_validators.as_ref(),
	};

	let verifier_options = convert_verification_options(
		&trusted_state.verification_options,
		trusted_state.trusting_period_duration(),
	)?;
	let now = convert_timestamp(current_time)?;

	let verifier = SpIoVerifier::default();
	let result = verifier.verify_update_header(
		untrusted_block_state,
		tendermint_trusted_state,
		&verifier_options,
		now,
	);

	match result {
		Verdict::Success => {
			let updated_state = create_updated_trusted_state(&trusted_state, &consensus_proof)?;
			Ok(updated_state)
		},
		Verdict::NotEnoughTrust(tally) =>
			Err(VerificationError::NotEnoughTrust(format!("Voting power tally: {}", tally))),
		Verdict::Invalid(detail) => Err(VerificationError::Invalid(format!("{:?}", detail))),
	}
}

/// Verify a header for misbehaviour detection (more relaxed verification)
pub fn verify_misbehaviour_header(
	trusted_state: TrustedState,
	consensus_proof: ConsensusProof,
	current_time: u64,
) -> Result<UpdatedTrustedState, VerificationError> {
	consensus_proof.validate().map_err(|e| VerificationError::Invalid(e))?;

	let validators = extract_validators(&trusted_state, &consensus_proof)?;
	let next_validators = consensus_proof
		.next_validators
		.as_ref()
		.map(|validators| ValidatorSet::new(validators.clone(), None));

	let chain_id = Id::try_from(trusted_state.chain_id.clone())
		.map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let height = Height::try_from(trusted_state.height)
		.map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let timestamp = Timestamp { seconds: trusted_state.timestamp as i64, nanos: 0 };
	let time = Time::try_from(timestamp).map_err(|e| VerificationError::Invalid(e.to_string()))?;
	let trusted_next_validators = ValidatorSet::new(trusted_state.next_validators.clone(), None);
	let trusted_next_validators_hash = Hash::Sha256(trusted_state.next_validators_hash);

	let tendermint_trusted_state = TrustedBlockState {
		chain_id: &chain_id,
		header_time: time,
		height,
		next_validators: &trusted_next_validators,
		next_validators_hash: trusted_next_validators_hash,
	};

	let untrusted_block_state = UntrustedBlockState {
		signed_header: &consensus_proof.signed_header,
		validators: &validators,
		next_validators: next_validators.as_ref(),
	};

	let verifier_options = convert_verification_options(
		&trusted_state.verification_options,
		trusted_state.trusting_period_duration(),
	)?;
	let now = convert_timestamp(current_time)?;

	let verifier = SpIoVerifier::default();

	let result = verifier.verify_misbehaviour_header(
		untrusted_block_state,
		tendermint_trusted_state,
		&verifier_options,
		now,
	);

	match result {
		Verdict::Success => {
			let updated_state = create_updated_trusted_state(&trusted_state, &consensus_proof)?;
			Ok(updated_state)
		},
		Verdict::NotEnoughTrust(tally) =>
			Err(VerificationError::NotEnoughTrust(format!("Voting power tally: {}", tally))),
		Verdict::Invalid(detail) => Err(VerificationError::Invalid(format!("{:?}", detail))),
	}
}

/// Validates which validator set the signed header references and check next_validators_hash
/// rotation. Returns the correct ValidatorSet for the header and validates next_validators if
/// rotation is signaled.
fn extract_validators<'a>(
	trusted_state: &'a TrustedState,
	consensus_proof: &'a ConsensusProof,
) -> Result<ValidatorSet, VerificationError> {
	let header = &consensus_proof.signed_header.header;
	let current_set = ValidatorSet::new(trusted_state.validators.clone(), None);
	let next_set = ValidatorSet::new(trusted_state.next_validators.clone(), None);

	// Validate current and next validator set hashes using the shared helper
	let current_hash_result =
		validate_validator_set_hash(&current_set, header.validators_hash, false);
	let next_hash_result = validate_validator_set_hash(&next_set, header.validators_hash, true);

	let validators = if current_hash_result.is_ok() {
		current_set
	} else if next_hash_result.is_ok() {
		next_set
	} else {
		return Err(VerificationError::Invalid(format!(
			"Unknown validator set hash: {:?}",
			header.validators_hash
		)));
	};

	let next_header_hash = &header.next_validators_hash;
	let next_hash = Hash::Sha256(trusted_state.next_validators_hash);
	if next_header_hash.is_empty() && consensus_proof.next_validators.is_some() {
		return Err(VerificationError::ValidatorSetError(
			"Next validators from Consensus Proof does not match signed header".to_string(),
		));
	} else if next_header_hash != &next_hash {
		let provided = consensus_proof.next_validators.as_ref().ok_or_else(|| {
			VerificationError::Invalid(
				"Header signals next_validators_hash rotation but consensus proof has no next_validators".to_string()
			)
		})?;
		let provided_set = ValidatorSet::new(provided.clone(), None);
		let provided_hash_result =
			validate_validator_set_hash(&provided_set, *next_header_hash, true);
		if provided_hash_result.is_err() {
			return Err(VerificationError::Invalid(format!(
				"Provided next_validators hash does not match signed_header.next_validators_hash"
			)));
		}
	}

	Ok(validators)
}

// Helper functions for type conversion

fn convert_verification_options(
	options: &VerificationOptions,
	trusting_period: Duration,
) -> Result<Options, VerificationError> {
	let trust_threshold = TrustThresholdFraction::new(
		options.trust_threshold_numerator,
		options.trust_threshold_denominator,
	)
	.map_err(|e| VerificationError::ConversionError(format!("Invalid trust threshold: {}", e)))?;

	Ok(Options { trust_threshold, trusting_period, clock_drift: options.clock_drift_duration() })
}

fn convert_timestamp(timestamp: u64) -> Result<Time, VerificationError> {
	Time::from_unix_timestamp(timestamp as i64, 0)
		.map_err(|e| VerificationError::Invalid(e.to_string()))
}

/// Rejects a validator set that reuses the same address twice. Each address is a
/// hash of its public key, so a duplicate signals a set that was not built honestly.
fn ensure_unique_addresses(
	validators: &[cometbft::validator::Info],
) -> Result<(), VerificationError> {
	let mut seen = BTreeSet::new();
	for validator in validators {
		if !seen.insert(validator.address) {
			return Err(VerificationError::ValidatorSetError(
				"duplicate validator address in set".to_string(),
			));
		}
	}
	Ok(())
}

fn create_updated_trusted_state(
	old_trusted_state: &TrustedState,
	consensus_proof: &ConsensusProof,
) -> Result<UpdatedTrustedState, VerificationError> {
	let header = &consensus_proof.signed_header.header;

	// Promote next_validators to validators
	let validators = old_trusted_state.next_validators.clone();

	// Only a signalled rotation may replace the stored next set, and only with a list
	// that hashes to the new signed next_validators_hash. When the interval is unchanged
	// we keep the trusted set and ignore any list the proof happened to carry.
	let old_next_hash = Hash::Sha256(old_trusted_state.next_validators_hash);
	let rotates =
		!header.next_validators_hash.is_empty() && header.next_validators_hash != old_next_hash;
	let next_validators = if rotates {
		let provided = consensus_proof.next_validators.as_ref().ok_or_else(|| {
			VerificationError::ValidatorSetError(
				"next validator set rotated but the proof carried no next validators".to_string(),
			)
		})?;
		ensure_unique_addresses(provided)?;
		validate_validator_set_hash(
			&ValidatorSet::new(provided.clone(), None),
			header.next_validators_hash,
			true,
		)
		.map_err(|e| VerificationError::ValidatorSetError(e.to_string()))?;
		provided.clone()
	} else {
		old_trusted_state.next_validators.clone()
	};

	// Use new next_validators_hash if present, else keep old
	let next_validators_hash =
		if !header.next_validators_hash.is_empty() {
			header.next_validators_hash.as_bytes().try_into().map_err(|_| {
				VerificationError::Invalid("Invalid next_validators_hash".to_string())
			})?
		} else {
			old_trusted_state.next_validators_hash
		};

	let new_trusted_state =
		TrustedState {
			chain_id: consensus_proof.chain_id().to_string(),
			height: consensus_proof.height(),
			timestamp: consensus_proof.timestamp(),
			validators,
			next_validators,
			next_validators_hash,
			trusting_period: old_trusted_state.trusting_period,
			verification_options: old_trusted_state.verification_options.clone(),
			finalized_header_hash: header.hash().as_bytes().try_into().map_err(|_| {
				VerificationError::Invalid("Invalid finalized_header_hash".to_string())
			})?,
		};

	Ok(UpdatedTrustedState::new(
		new_trusted_state,
		consensus_proof.height(),
		consensus_proof.timestamp(),
	))
}

#[cfg(test)]
mod tests {
	use super::ensure_unique_addresses;
	use cometbft::{
		validator::{Info, ProposerPriority},
		vote::Power,
		PublicKey,
	};
	use tendermint_primitives::account_id_from_public_key;

	// A valid ed25519 public key (RFC 8032 test vector 1).
	const ED25519_PUBKEY: [u8; 32] = [
		0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64, 0x07,
		0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68, 0xf7, 0x07,
		0x51, 0x1a,
	];

	fn validator() -> Info {
		let pub_key = PublicKey::from_raw_ed25519(&ED25519_PUBKEY).unwrap();
		Info {
			address: account_id_from_public_key(&pub_key).unwrap(),
			pub_key,
			power: Power::try_from(10u64).unwrap(),
			name: None,
			proposer_priority: ProposerPriority::from(0i64),
		}
	}

	#[test]
	fn accepts_unique_addresses() {
		assert!(ensure_unique_addresses(&[validator()]).is_ok());
	}

	#[test]
	fn rejects_duplicate_addresses() {
		assert!(ensure_unique_addresses(&[validator(), validator()]).is_err());
	}
}
