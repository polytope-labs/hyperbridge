//! Error types raised by the BEEFY verifier and its SP1 sibling.

use alloc::string::String;
use thiserror::Error;

/// All failures the BEEFY proof verifiers can raise. Dispatchers may match on
/// [`Error::StaleHeight`] to distinguish a benign uncle attempt from a hard verification
/// failure; everything else is opaque and indicates the proof itself is invalid.
#[derive(Error, Debug)]
pub enum Error {
	/// `trusted_state.latest_beefy_height >= proof.block_number`. Surfaced before any
	/// cryptographic work, so it's cheap to recognise and re-route as an uncle.
	#[error("Stale height: trusted height {trusted_height} >= current_height {current_height}")]
	StaleHeight {
		/// Trusted state height at verification time.
		trusted_height: u32,
		/// Block number reported by the proof.
		current_height: u32,
	},
	/// Fewer than the BEEFY supermajority threshold of authorities signed the commitment.
	#[error("Super majority of signatures required")]
	SuperMajorityRequired,
	/// The commitment was signed by an authority set the verifier does not know about.
	#[error("Unkown authority set id {id}")]
	UnknownAuthoritySet {
		/// Unknown authority set id from the commitment.
		id: u64,
	},
	/// The signed commitment payload is missing its MMR root hash entry.
	#[error("MMR root hash is missing from commitment payload")]
	MmrRootHashMissing,
	/// The MMR root hash entry is the wrong length (expected 32 bytes).
	#[error("Invalid MMR root hash length: expected 32, found {len}")]
	InvalidMmrRootHashLength {
		/// Actual length found.
		len: usize,
	},
	/// `secp256k1` ecrecover did not return a public key for one of the signatures.
	#[error("Failed to recover public key from signature")]
	FailedToRecoverPublicKey,
	/// The merkle multi-proof of the signing authorities does not verify.
	#[error("Invalid authorities proof")]
	InvalidAuthoritiesProof,
	/// MMR-leaf-vs-root verification raised an internal error.
	#[error("MMR verification failed during calculation: {0}")]
	MmrVerificationFailed(String),
	/// MMR-leaf-vs-root verification ran but the calculated root differs from the proven root.
	#[error("Invalid MMR proof: calculated root does not match provided root")]
	InvalidMmrProof,
	/// The merkle proof of parachain headers does not verify.
	#[error("Invalid parachain header proof: merkle proof verification failed")]
	InvalidParachainProof,
	/// The SP1 Groth16 verifier rejected the proof bytes.
	#[error("SP1 proof verification failed")]
	Sp1VerificationFailed,
}
