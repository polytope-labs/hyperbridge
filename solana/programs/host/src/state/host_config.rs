use anchor_lang::prelude::*;

/// Singleton at `[b"host_config"]` — protocol-wide parameters.
#[account]
#[derive(InitSpace)]
pub struct HostConfig {
    pub admin: Pubkey,
    /// `StateMachine` discriminant placeholder; switch when ismp-core
    /// adds a `Solana` variant.
    pub host_state_machine: u32,
    pub hyperbridge_id: u32,
    /// `b"BEFY"` for the SP1 BEEFY client.
    pub consensus_client_id: [u8; 4],
    pub challenge_period: u64,
    pub unbonding_period: u64,
    pub default_timeout: u64,
    pub fee_token_mint: Pubkey,
    pub per_byte_fee: u64,
    pub frozen: bool,
    /// Hot-swappable handler program id. State-mutating primitives
    /// require a CPI signed by this program's `[b"handler_authority"]`
    /// PDA.
    pub handler_program: Pubkey,
    pub bump: u8,
}

impl HostConfig {
    pub const SEED: &'static [u8] = b"host_config";
    pub const HANDLER_AUTHORITY_SEED: &'static [u8] = b"handler_authority";
}
