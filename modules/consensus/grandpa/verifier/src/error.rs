// Copyright (c) 2025 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

//! Typed errors for the GRANDPA consensus support — both the verifier
//! (`verify_grandpa_finality_proof`,
//! `verify_parachain_headers_with_grandpa_finality_proof`) and the
//! `ismp-grandpa` client wrapper.
//!
//! The same enum spans both layers so the ismp client doesn't have to
//! redefine variants; downstream callers map to `ismp::error::Error`
//! via the `From` impl below.

use alloc::string::{String, ToString};
use ismp::host::StateMachine;

/// Failure modes for GRANDPA consensus proof verification and the ismp
/// client wrapper.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	// -- verifier-level --
	/// The submitted finality proof has no headers to chain against.
	#[error("Unknown headers can't be empty")]
	UnknownHeadersEmpty,
	/// The justification target hash does not match the highest header
	/// in `unknown_headers`.
	#[error("Latest finalized block should be highest block in unknown_headers")]
	LatestBlockMismatch,
	/// The justification block hash doesn't match the finality proof
	/// block hash.
	#[error("Justification target hash and finality proof block hash mismatch")]
	JustificationTargetMismatch,
	/// The submitted `unknown_headers` don't form a valid ancestor chain
	/// back to the trusted finalized block.
	#[error("Invalid ancestry")]
	InvalidAncestry,
	/// The justification bytes failed to SCALE-decode.
	#[error("Failed to decode justification: {0}")]
	DecodeJustification(String),
	/// The justification's BLS aggregate didn't verify under the trusted authorities.
	#[error("Justification verification failed: {0}")]
	JustificationVerify(String),
	/// A parachain header state proof failed verification against the
	/// relay-chain header's state root.
	#[error("Error verifying parachain header state proof: {0}")]
	StateProofVerification(String),
	/// The submitted state proof doesn't include the requested parachain
	/// header.
	#[error("Invalid proof, parachain header not found")]
	ParachainHeaderNotFound,
	/// A `parachain_headers` map entry references a relay-chain hash that
	/// is in the finalized ancestry route (`headers.ancestry`) but whose
	/// header is not present in `finality_proof.unknown_headers`. The
	/// trusted latest relay hash is the canonical instance of this:
	/// `AncestryChain::ancestry` includes the base hash even when the
	/// base header is not in the map. The verifier used to `.expect` the
	/// header here and panic; it now surfaces a typed error.
	#[error("Parachain header proof references a relay hash with no relay-chain header in unknown_headers")]
	RelayHeaderNotInUnknownHeaders,
	/// A parachain header in the state proof failed to SCALE-decode.
	#[error("Error decoding header: {0}")]
	DecodeHeader(String),

	// -- ismp-grandpa client wrapper --
	/// The submitted consensus proof failed to SCALE-decode into a `ConsensusMessage`.
	#[error("Cannot decode consensus message: {0}")]
	DecodeConsensusMessage(String),
	/// The trusted state failed to SCALE-decode into a `ConsensusState`.
	#[error("Cannot decode consensus state: {0}")]
	DecodeConsensusState(String),
	/// The submitted finality proof failed to SCALE-decode in fraud-proof handling.
	#[error("Cannot decode finality proof: {0}")]
	DecodeFinalityProof(String),
	/// Required coprocessor configuration is missing.
	#[error("Coprocessor not set; cannot determine para id state machine id")]
	CoprocessorNotSet,
	/// A finalized header carries no timestamp / ISMP root digest.
	#[error("Timestamp or ismp root not found")]
	MissingTimestampOrIsmpRoot,
	/// Asked to handle a state machine the client doesn't track a slot
	/// duration for.
	#[error("Slot duration not set for state machine: {0}")]
	SlotDurationNotSet(StateMachine),
	/// Both fraud proofs target the same block, so there's no
	/// equivocation to prove.
	#[error("Fraud proofs are for the same block")]
	FraudProofsSameBlock,
	/// The two fraud proofs aren't on the same chain.
	#[error("Fraud proofs are not for the same chain")]
	FraudProofsDifferentChain,
	/// The two fraud proofs don't share an ancestor.
	#[error("Fraud proofs are not for the same ancestor")]
	FraudProofsDifferentAncestor,
	/// The two finalized targets sit on the same canonical chain, so the proofs
	/// don't witness a fork.
	#[error("Fraud proofs are on the same branch")]
	FraudProofsSameBranch,
	/// A submitted justification doesn't match the trusted consensus
	/// latest hash.
	#[error("Justification does not match consensus latest hash")]
	JustificationConsensusMismatch,
	/// A submitted justification failed verification under the trusted
	/// authorities.
	#[error("Invalid justification")]
	InvalidJustification,
	/// Asked to serve a state machine the client doesn't support.
	#[error("Unsupported state machine: {0:?}")]
	UnsupportedStateMachine(StateMachine),

	// -- forwarding for ismp client --
	/// An ISMP-level error surfaced from a nested call (kept as-is
	/// rather than stringified, so we can re-emit it unchanged).
	#[error("{0:?}")]
	Ismp(#[from] ismp::error::Error),
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
