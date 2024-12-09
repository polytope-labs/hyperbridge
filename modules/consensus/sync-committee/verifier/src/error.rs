use alloc::string::String;
use core::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
	SyncCommitteeParticipantsTooLow,
	InvalidUpdate(String),
	DomainError,
	InvalidMerkleBranch(String),
	InvalidRoot(String),
	MerkleizationError(String),
	BlsError(bls::errors::BLSError),
	SignatureVerification,
}

impl From<bls::errors::BLSError> for Error {
	fn from(value: bls::errors::BLSError) -> Self {
		Error::BlsError(value)
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		match self {
			Error::SyncCommitteeParticipantsTooLow => {
				write!(f, "Sync committee participants are too low")
			},
			Error::InvalidUpdate(err) => write!(f, "Invalid update {err:?}"),
			Error::DomainError => write!(f, "Couldn't get domain"),
			Error::BlsError(err) => write!(f, "BlsError: {err:?}"),
			Error::InvalidMerkleBranch(err) => write!(f, "Invalid merkle branch {err:?}"),
			Error::InvalidRoot(err) => write!(f, "Invalid root {err:?}"),
			Error::MerkleizationError(err) => write!(f, "Merkleization error {err:?}"),
			Error::SignatureVerification => write!(f, "Signature verification failed"),
		}
	}
}
