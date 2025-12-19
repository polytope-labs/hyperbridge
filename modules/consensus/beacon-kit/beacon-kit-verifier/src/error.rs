#![cfg_attr(not(feature = "std"), no_std)]

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BeaconKitError {
	#[error("Not enough signers to meet consensus threshold")]
	InsufficientSigners,
	#[error("Update contains a signer not present in the trusted validator set")]
	UnknownSigner,
	#[error("Failed to compute domain")]
	DomainComputationFailed,
	#[error("Failed to compute signing root")]
	SigningRootComputationFailed,
	#[error("Failed to verify aggregate BLS signature")]
	SignatureVerificationFailed,
	#[error("Failed to hash execution payload")]
	ExecutionPayloadHashFailed,
	#[error("Invalid Execution Payload Merkle Proof")]
	InvalidExecutionPayloadProof,
	#[error("Failed to hash validator set")]
	ValidatorSetHashFailed,
	#[error("Invalid Validator Set Merkle Proof")]
	InvalidValidatorSetProof,
	#[error("Failed to create SSZ List from validators")]
	SszListCreationFailure,
	#[error("Invalid public key provided")]
	InvalidPublicKey,
}
