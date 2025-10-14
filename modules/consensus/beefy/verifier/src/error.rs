use thiserror::Error;
#[derive(Error, Debug)]
pub enum Error {
	#[error("Stale height: trusted height {trusted_height} >= current_height {current_height}")]
	StaleHeight { trusted_height: u32, current_height: u32 },
	#[error("Super majority of signatures required")]
	SuperMajorityRequired,
	#[error("Unkown authority set id {id}")]
	UnknownAuthoritySet { id: u64 },
	#[error("MMR root hash is missing from commitment payload")]
	MmrRootHashMissing,
	#[error("Invalid MMR root hash length: expected 32, found {len}")]
	InvalidMmrRootHashLength { len: usize },
	#[error("Invalid signature recovery ID")]
	InvalidRecoveryId,
	#[error("Invalid signature format")]
	InvalidSignatureFormat,
	#[error("Failed to recover public key from signature")]
	FailedToRecoverPublicKey,
	#[error("Invalid authorities proof")]
	InvalidAuthoritiesProof,
	#[error("MMR verification failed during calculation: {0}")]
	MmrVerificationFailed(String),
	#[error("Invalid MMR proof: calculated root does not match provided root")]
	InvalidMmrProof,
}
