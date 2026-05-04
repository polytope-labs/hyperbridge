use anchor_lang::prelude::*;

#[error_code]
pub enum HyperbridgeError {
    #[msg("Caller is not the admin recorded in HostConfig")]
    Unauthorized,
    #[msg("Host is currently frozen")]
    HostFrozen,
    #[msg("Consensus client is currently frozen")]
    ConsensusClientFrozen,
    #[msg("Initial consensus state must be non-empty")]
    EmptyConsensusState,
    #[msg("Consensus state payload exceeds the configured max size")]
    ConsensusStateTooLarge,
    #[msg("HostConfig.handler_program is unset; call set_handler first")]
    HandlerNotConfigured,
    #[msg("set_consensus_state.id must equal HostConfig.consensus_client_id")]
    ConsensusClientIdMismatch,
    #[msg("handler_authority signer does not match the PDA derived from HostConfig.handler_program")]
    UnauthorizedHandler,
    #[msg("StateCommitment has been vetoed and cannot be used to verify proofs")]
    StateCommitmentVetoed,
    #[msg("StateCommitment is already vetoed — no-op")]
    AlreadyVetoed,
    #[msg("Fee vault has zero pending fees — nothing to withdraw")]
    NothingToWithdraw,
}
