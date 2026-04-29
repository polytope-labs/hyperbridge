use anchor_lang::prelude::*;

/// `[b"fee_vault", relayer]` — accrued-fee counter, denominated in
/// `HostConfig.fee_token_mint` smallest unit. The actual SPL transfer
/// happens at withdrawal time against a separately-funded vault; that
/// funding flow is on the Hyperbridge runtime side and out of scope
/// here.
#[account]
#[derive(InitSpace)]
pub struct RelayerFeeVault {
    pub relayer: Pubkey,
    pub pending: u64,
    pub last_accrual: i64,
    pub bump: u8,
}

impl RelayerFeeVault {
    pub const SEED_PREFIX: &'static [u8] = b"fee_vault";
}
