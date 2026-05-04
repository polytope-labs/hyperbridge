use anchor_lang::prelude::*;

/// `[b"req", commitment]` — replay guard for inbound POST requests.
/// Anchor's `init` constraint catches duplicate `commitment` values.
/// Permanent: never closed. Closing the PDA would let an attacker re-init
/// it with the same commitment and re-dispatch the original message
/// (the source-chain storage proof and consensus state stay valid
/// indefinitely). EVM mirrors this — `_requestReceipts[commitment]` is
/// only deleted on dispatch *failure* so a different relayer can retry.
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
