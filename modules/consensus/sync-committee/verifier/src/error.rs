use alloc::string::{String, ToString};

/// Failure modes for sync-committee consensus proof verification and
/// the ismp client wrapper. The same enum spans both layers so the
/// ismp client doesn't have to redefine variants; downstream callers
/// map to `ismp::error::Error` via the `From` impl below.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	// -- verifier-level (produced by `verify_sync_committee_attestation`) --
	#[error("Sync committee participants are too low")]
	SyncCommitteeParticipantsTooLow,
	#[error("Invalid update: {0:?}")]
	InvalidUpdate(String),
	#[error("Couldn't get domain")]
	DomainError,
	#[error("Invalid merkle branch: {0:?}")]
	InvalidMerkleBranch(String),
	#[error("Invalid root: {0:?}")]
	InvalidRoot(String),
	#[error("Merkleization error: {0:?}")]
	MerkleizationError(String),
	#[error("BLS error: {0:?}")]
	BlsError(bls::errors::BLSError),
	#[error("Signature verification failed")]
	SignatureVerification,

	// -- ismp-sync-committee client wrapper --
	/// The submitted consensus proof failed to SCALE-decode into a `BeaconClientUpdate`.
	#[error("Cannot decode beacon client update")]
	DecodeBeaconClientUpdate,
	/// The submitted trusted state failed to SCALE-decode into a `ConsensusState`.
	#[error("Cannot decode trusted consensus state")]
	DecodeConsensusState,
	/// Failed to convert the verifier's light client state into the codec-friendly form.
	#[error("Cannot convert light client state to codec type")]
	ConvertLightClientState,
	/// Fraud-proof verification is not implemented for this client.
	#[error("Fraud proof verification unimplemented")]
	FraudProofUnimplemented,
	/// Asked to serve a state machine the client doesn't support.
	#[error("State machine not supported")]
	UnsupportedStateMachine,
	/// An ISMP-level error surfaced from a nested call (kept as-is
	/// rather than stringified, so we can re-emit it unchanged).
	#[error("{0:?}")]
	Ismp(#[from] ismp::error::Error),
}

impl From<bls::errors::BLSError> for Error {
	fn from(value: bls::errors::BLSError) -> Self {
		Error::BlsError(value)
	}
}

impl From<Error> for ismp::error::Error {
	fn from(value: Error) -> Self {
		match value {
			// Preserve the original ISMP error so callers can match on
			// it; everything else folds into `Custom`.
			Error::Ismp(err) => err,
			other => ismp::error::Error::Custom(other.to_string()),
		}
	}
}
