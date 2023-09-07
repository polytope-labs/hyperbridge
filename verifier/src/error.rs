use core::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
	SyncCommitteeParticipantsTooLow,
	InvalidUpdate,
	DomainError,
	InvalidMerkleBranch,
	InvalidRoot,
	MerkleizationError,
	SignatureVerification,
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		match self {
			Error::SyncCommitteeParticipantsTooLow => {
				write!(f, "Sync committee participants are too low")
			},
			Error::InvalidUpdate => write!(f, "Invalid update"),
			Error::DomainError => write!(f, "Couldn't get domain"),
			Error::InvalidMerkleBranch => write!(f, "Invalid merkle branch"),
			Error::InvalidRoot => write!(f, "Invalid root"),
			Error::MerkleizationError => write!(f, "Merkleization error"),
			Error::SignatureVerification => write!(f, "Signature verification failed"),
		}
	}
}
