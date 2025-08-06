use cometbft_light_client_verifier::{
	errors::VerificationError,
	operations::{
		ProdCommitValidator, ProvidedVotingPowerCalculator, VotingPowerCalculator, VotingPowerTally,
	},
	options::Options,
	predicates::VerificationPredicates,
	types::{SignedHeader, TrustThreshold, TrustedBlockState, UntrustedBlockState, ValidatorSet},
	PredicateVerifier, Verdict, Verifier,
};
use cometbft_proto::types::v1::SimpleValidator;
use prost::alloc::vec::Vec;

use crate::hashing::{SpIoSha256, SpIoSignatureVerifier};

use cometbft::Hash;

use prost::Message;

/// Helper function to validate a validator set hash using both main and fallback strategies.
pub fn validate_validator_set_hash(
	validators: &ValidatorSet,
	header_validators_hash: Hash,
	is_next_validators: bool,
) -> Result<(), cometbft_light_client_verifier::errors::VerificationError> {
	let hash = validators.hash_with::<SpIoSha256>();
	if hash != header_validators_hash {
		let validator_bytes: Vec<Vec<u8>> = validators
			.validators
			.iter()
			.map(|validator| {
				let pub_key_bytes = {
					if let Some(secp256k1_key) = validator.pub_key.secp256k1() {
						secp256k1_key.to_encoded_point(false).as_bytes().to_vec()
					} else {
						validator.pub_key.to_bytes()
					}
				};

				let simple_validator_bytes = SimpleValidator {
					pub_key: Some(cometbft_proto::crypto::v1::PublicKey {
						sum: Some(cometbft_proto::crypto::v1::public_key::Sum::Bls12381(
							pub_key_bytes,
						)),
					}),
					voting_power: validator.power.into(),
				};

				simple_validator_bytes.encode_to_vec()
			})
			.collect();

		let simple_validator_hash =
			cometbft::merkle::simple_hash_from_byte_vectors::<SpIoSha256>(&validator_bytes);

		if Hash::Sha256(simple_validator_hash) != header_validators_hash {
			use cometbft_light_client_verifier::errors::VerificationError;
			return if is_next_validators {
				Err(VerificationError::invalid_next_validator_set(header_validators_hash, hash))
			} else {
				Err(VerificationError::invalid_validator_set(header_validators_hash, hash))
			};
		}
	}
	Ok(())
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpIoPredicates;

impl VerificationPredicates for SpIoPredicates {
	type Sha256 = SpIoSha256;

	fn validator_sets_match(
		&self,
		validators: &ValidatorSet,
		header_validators_hash: cometbft::Hash,
	) -> Result<(), VerificationError> {
		validate_validator_set_hash(validators, header_validators_hash, false)
	}

	fn next_validators_match(
		&self,
		next_validators: &ValidatorSet,
		header_next_validators_hash: cometbft::Hash,
	) -> Result<(), VerificationError> {
		validate_validator_set_hash(next_validators, header_next_validators_hash, true)
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpIoVotingPowerCalculator;

impl VotingPowerCalculator for SpIoVotingPowerCalculator {
	fn voting_power_in(
		&self,
		signed_header: &SignedHeader,
		validator_set: &ValidatorSet,
		trust_threshold: TrustThreshold,
	) -> Result<VotingPowerTally, VerificationError> {
		let calculator = ProvidedVotingPowerCalculator::<SpIoSignatureVerifier>::default();
		calculator.voting_power_in(signed_header, validator_set, trust_threshold)
	}

	fn voting_power_in_sets(
		&self,
		signed_header: &SignedHeader,
		first_set: (&ValidatorSet, TrustThreshold),
		second_set: (&ValidatorSet, TrustThreshold),
	) -> Result<(VotingPowerTally, VotingPowerTally), VerificationError> {
		let calculator = ProvidedVotingPowerCalculator::<SpIoSignatureVerifier>::default();
		calculator.voting_power_in_sets(signed_header, first_set, second_set)
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpIoVerifier;

impl Verifier for SpIoVerifier {
	fn verify_update_header(
		&self,
		untrusted_block_state: UntrustedBlockState,
		trusted_block_state: TrustedBlockState,
		options: &Options,
		now: cometbft::time::Time,
	) -> Verdict {
		let verifier =
			PredicateVerifier::new(SpIoPredicates, SpIoVotingPowerCalculator, ProdCommitValidator);
		verifier.verify_update_header(untrusted_block_state, trusted_block_state, options, now)
	}

	fn verify_misbehaviour_header(
		&self,
		untrusted_block_state: UntrustedBlockState,
		trusted_block_state: TrustedBlockState,
		options: &Options,
		now: cometbft::time::Time,
	) -> Verdict {
		let verifier =
			PredicateVerifier::new(SpIoPredicates, SpIoVotingPowerCalculator, ProdCommitValidator);
		verifier.verify_misbehaviour_header(
			untrusted_block_state,
			trusted_block_state,
			options,
			now,
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use cometbft::crypto::{default::Sha256 as DefaultSha256, Sha256};

	#[test]
	fn test_hash_consistency() {
		let test_data = b"Hello, CometBFT!";

		let sp_io_hash = SpIoSha256::digest(test_data);
		let default_hash = DefaultSha256::digest(test_data);
		println!("sp_io_hash: {:?}", sp_io_hash);
		println!("default_hash: {:?}", default_hash);

		assert_eq!(sp_io_hash, default_hash, "Hash implementations should be identical");
	}
}
