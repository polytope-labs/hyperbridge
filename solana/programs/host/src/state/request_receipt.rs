use anchor_lang::prelude::*;

/// `[b"req", commitment]` — replay guard for inbound POST requests.
/// Anchor's `init` constraint catches duplicate `commitment` values.
/// Closable via `close_expired_receipt` to reclaim rent after timeout.
#[account]
#[derive(InitSpace)]
pub struct RequestReceipt {
    pub commitment: [u8; 32],
    pub relayer: Pubkey,
    pub received_at: i64,
    pub bump: u8,
}

impl RequestReceipt {
    pub const SEED_PREFIX: &'static [u8] = b"req";
}
