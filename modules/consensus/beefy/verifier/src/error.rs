//! Typed errors for the BEEFY consensus support — both the verifier
//! (`verify_consensus`, `verify_sp1_consensus`) and the `ismp-beefy`
//! client wrapper.
//!
//! The same enum spans both layers so the ismp client doesn't have to
//! redefine variants; downstream callers map to `ismp::error::Error`
//! via the `From` impl below.

use alloc::string::String;
use ismp::host::StateMachine;
use thiserror::Error;

/// Failure modes for BEEFY consensus proof verification and the ismp
/// client wrapper. Dispatchers may match on [`Error::StaleHeight`] to
/// distinguish a benign uncle attempt from a hard verification failure;
/// everything else is opaque and indicates the proof itself is invalid.
#[derive(Error, Debug)]
pub enum Error {
	// -- verifier-level --
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

	// -- ismp-beefy client wrapper --
	/// The trusted state failed to SCALE-decode into a `ConsensusState`.
	#[error("Cannot decode consensus state: {0}")]
	DecodeConsensusState(String),
	/// The proof bytes had no leading type byte.
	#[error("Empty proof")]
	EmptyProof,
	/// The naive (in-runtime) BEEFY proof payload failed to SCALE-decode.
	#[error("Cannot decode naive proof: {0}")]
	DecodeNaiveProof(String),
	/// The SP1 proof payload failed to SCALE-decode.
	#[error("Cannot decode SP1 proof: {0}")]
	DecodeSp1Proof(String),
	/// The leading proof byte didn't match either `PROOF_TYPE_NAIVE` or `PROOF_TYPE_SP1`.
	#[error("Unknown proof type: {0}")]
	UnknownProofType(u8),
	/// A parachain header in the consensus message failed to SCALE-decode.
	#[error("Error decoding parachain header: {0}")]
	DecodeParachainHeader(String),
	/// A parachain header's timestamp digest failed to SCALE-decode.
	#[error("Failed to decode timestamp digest: {0}")]
	DecodeTimestampDigest(String),
	/// A parachain header's ISMP consensus digest could not be decoded.
	#[error("Header contains an invalid ismp consensus log")]
	InvalidIsmpConsensusLog,
	/// A parachain header is missing its timestamp digest.
	#[error("Timestamp not found")]
	TimestampNotFound,
	/// The ismp host this client runs on isn't itself a parachain (Polkadot or Kusama).
	#[error("Host state machine should be a parachain")]
	HostStateMachineNotParachain,
	/// An MMR fraud proof failed to SCALE-decode.
	#[error("Cannot decode MMR proof: {0}")]
	DecodeMmrProof(String),
	/// The two fraud proofs aren't for the same block number.
	#[error("Fraud proofs must be for the same block number")]
	FraudProofsDifferentBlock,
	/// The two fraud proofs commit to identical payloads — no equivocation.
	#[error("Fraud proofs have identical commitments, no equivocation")]
	FraudProofsIdenticalCommitments,
	/// One of the fraud proof MMR commitments failed verification.
	#[error("Fraud proof verification failed: {0}")]
	FraudProofVerificationFailed(String),
	/// Asked to serve a state machine the client doesn't support.
	#[error("State machine not supported: {0:?}")]
	UnsupportedStateMachine(StateMachine),
	/// The configured `BeefyClientConfig` doesn't track the requested parachain id.
	#[error("Parachain with id {0} not registered")]
	UnregisteredParachain(u32),

	// -- forwarding for ismp client --
	/// An ISMP-level error surfaced from a nested call (kept as-is
	/// rather than stringified, so we can re-emit it unchanged).
	#[error("{0:?}")]
	Ismp(#[from] ismp::error::Error),
}

impl From<Error> for ismp::error::Error {
	fn from(value: Error) -> Self {
		match value {
			// Preserve the original ISMP error so callers can match on it.
			Error::Ismp(err) => err,
			// Carry the typed verifier error inside `AnyHow` so dispatchers can
			// still downcast it — `pallet-beefy-consensus-proofs` matches on
			// `StaleHeight` to re-route an SP1 uncle proof.
			other => ismp::error::Error::AnyHow(anyhow::Error::new(other).into()),
		}
	}
}
