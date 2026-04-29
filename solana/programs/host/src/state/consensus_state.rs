use anchor_lang::prelude::*;

/// `[b"consensus", id]` — per-client trust anchor (SCALE-encoded
/// `beefy_verifier_primitives::ConsensusState` for BEEFY).
#[account]
pub struct ConsensusState {
    pub id: [u8; 4],
    pub last_updated: i64,
    pub frozen: bool,
    /// Opaque to the host; decoded by the client adapter.
    pub state: Vec<u8>,
    pub bump: u8,
}

impl ConsensusState {
    pub const SEED_PREFIX: &'static [u8] = b"consensus";

    /// Hard cap on the BEEFY consensus payload. Known fixture is 128B;
    /// generous headroom for authority-set growth.
    pub const MAX_STATE_LEN: usize = 4 * 1024;

    pub const SPACE: usize = 8       // anchor discriminator
        + 4                          // id
        + 8                          // last_updated
        + 1                          // frozen
        + 4                          // Vec<u8> length prefix
        + Self::MAX_STATE_LEN
        + 1; // bump
}
