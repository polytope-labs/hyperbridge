use anchor_lang::prelude::*;

/// `[b"commit", state_machine.to_le_bytes(), height.to_le_bytes()]` —
/// trusted state root for a finalized source-chain block.
#[account]
#[derive(InitSpace)]
pub struct StateCommitment {
    pub state_machine: u32,
    pub height: u64,
    pub state_root: [u8; 32],
    /// MMR root from the source chain's parachain header digest
    /// (engine_id `b"ISMP"`). What `verify_membership` checks request
    /// commitments against.
    pub overlay_root: [u8; 32],
    /// Source-chain block timestamp; 0 if not extracted.
    pub timestamp: u64,
    /// Solana clock at PDA creation — feeds the challenge-period gate.
    pub updated_at: i64,
    /// `veto_state_commitment` flips this; vetoed commitments can't be
    /// used as proof anchors.
    pub vetoed: bool,
    pub vetoed_by: Pubkey,
    pub bump: u8,
}

impl StateCommitment {
    pub const SEED_PREFIX: &'static [u8] = b"commit";
}
