use anchor_lang::prelude::*;

/// Singleton at `[b"handler_state"]`. Tracks the highest
/// `validator_set_id` for which a consensus proof has been processed —
/// per-id relayer attribution lives in [`crate::state::EpochRecord`].
#[account]
#[derive(InitSpace)]
pub struct HandlerState {
    pub current_epoch: u64,
    pub bump: u8,
}

impl HandlerState {
    pub const SEED: &'static [u8] = b"handler_state";
}
