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
    #[msg("handler_authority signer does not match the PDA derived from HostConfig.handler_program")]
    UnauthorizedHandler,
    #[msg("relayer_refund target does not match the receipt's recorded relayer")]
    ReceiptRelayerMismatch,
    #[msg("Receipt is not yet old enough to be closed (default_timeout + safety hasn't elapsed)")]
    ReceiptNotYetExpired,
    #[msg("StateCommitment has been vetoed and cannot be used to verify proofs")]
    StateCommitmentVetoed,
    #[msg("StateCommitment is already vetoed — no-op")]
    AlreadyVetoed,
    #[msg("Fee vault has zero pending fees — nothing to withdraw")]
    NothingToWithdraw,
}
