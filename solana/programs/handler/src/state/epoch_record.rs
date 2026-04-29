use anchor_lang::prelude::*;

/// `[b"epoch", authority_set_id.to_le_bytes()]`. First relayer to advance
/// the epoch wins the attribution.
#[account]
#[derive(InitSpace)]
pub struct EpochRecord {
    pub authority_set_id: u64,
    pub relayer: Pubkey,
    pub recorded_at: i64,
    pub bump: u8,
}

impl EpochRecord {
    pub const SEED_PREFIX: &'static [u8] = b"epoch";
}
