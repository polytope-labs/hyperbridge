use anchor_lang::prelude::*;

#[error_code]
pub enum HandlerError {
    #[msg("Wrong proof-type prefix; expected PROOF_TYPE_SP1=0x01")]
    WrongProofType,
    #[msg("Failed to SCALE-decode the inner Sp1BeefyProof")]
    InvalidSp1BeefyProof,
    #[msg("Failed to SCALE-decode the stored ConsensusState payload")]
    InvalidStoredConsensusState,
    #[msg("Proof block_number is not strictly greater than latest_beefy_height")]
    NonMonotonicHeight,
    #[msg("Proof's validator_set_id matches neither current nor next authorities")]
    UnknownAuthoritySet,
    #[msg("SP1 v6 Groth16 verification rejected the proof")]
    Sp1VerificationFailed,
    #[msg("Substrate header is shorter than the minimum prefix")]
    HeaderTooShort,
    #[msg("Failed to SCALE-decode a field in the Substrate header prefix")]
    HeaderDecodeFailed,
    #[msg("Parachain header is missing the pallet-ismp ConsensusDigest (engine_id=ISMP)")]
    IsmpDigestMissing,
    #[msg("Parachain header is missing the pallet-ismp TimestampDigest (engine_id=ISTM)")]
    TimestampDigestMissing,
    #[msg("This proof contains no parachain headers — nothing to commit")]
    NoHeadersInProof,
    #[msg("commit_header_index is out of range for this proof's headers")]
    CommitHeaderIndexOutOfRange,
    #[msg("Substrate storage proof is malformed or doesn't hash to the trusted state_root")]
    InvalidStorageProof,
    #[msg("Storage proof is valid but the key has no value at the trusted state_root")]
    StorageKeyAbsent,
    #[msg("commitment param does not match hash_request(req)")]
    CommitmentMismatch,
    #[msg("Failed to SCALE-decode the PostRequest from post_request_body")]
    InvalidPostRequestFormat,
    #[msg("Supplied dest_program does not match the `to` field decoded from the PostRequest")]
    DestProgramMismatch,
    #[msg("StateCommitment seeds don't match the (state_machine, height) supplied by the relayer")]
    StateCommitmentSeedsMismatch,
    #[msg("StateCommitment account is vetoed and cannot be used")]
    StateCommitmentVetoed,
}
