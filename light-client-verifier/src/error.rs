use core::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    SyncCommitteeParticiapntsTooLow,
    InvalidUpdate,
    DomainError,
    FastAggregateError(ethereum_consensus::crypto::Error),
    InvalidMerkleBranch,
}

impl From<ethereum_consensus::crypto::Error> for Error {
    fn from(error: ethereum_consensus::crypto::Error) -> Self {
        Error::FastAggregateError(error)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::SyncCommitteeParticiapntsTooLow => {
                write!(f, "Sync committee participants are too low")
            }
            Error::InvalidUpdate => write!(f, "Invalid update"),
            Error::DomainError => write!(f, "Couldn't get domain"),
            Error::FastAggregateError(err) => write!(f, "Fast aggregate error"),
            Error::InvalidMerkleBranch => write!(f, "Invalid merkle branch"),
        }
    }
}
