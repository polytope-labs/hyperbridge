use anchor_lang::prelude::*;

use crate::state::HostConfig;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct InitializeHostParams {
    pub host_state_machine: u32,
    pub hyperbridge_id: u32,
    pub consensus_client_id: [u8; 4],
    pub challenge_period: u64,
    pub unbonding_period: u64,
    pub default_timeout: u64,
    pub fee_token_mint: Pubkey,
    pub per_byte_fee: u64,
}

#[derive(Accounts)]
pub struct InitializeHost<'info> {
    /// Recorded as `HostConfig.admin`.
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + HostConfig::INIT_SPACE,
        seeds = [HostConfig::SEED],
        bump,
    )]
    pub host_config: Account<'info, HostConfig>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<InitializeHost>, p: InitializeHostParams) -> Result<()> {
    let cfg = &mut ctx.accounts.host_config;
    cfg.admin = ctx.accounts.admin.key();
    cfg.host_state_machine = p.host_state_machine;
    cfg.hyperbridge_id = p.hyperbridge_id;
    cfg.consensus_client_id = p.consensus_client_id;
    cfg.challenge_period = p.challenge_period;
    cfg.unbonding_period = p.unbonding_period;
    cfg.default_timeout = p.default_timeout;
    cfg.fee_token_mint = p.fee_token_mint;
    cfg.per_byte_fee = p.per_byte_fee;
    cfg.frozen = false;
    // Set later via `set_handler` so the two programs can be deployed
    // in any order.
    cfg.handler_program = Pubkey::default();
    cfg.bump = ctx.bumps.host_config;
    Ok(())
}
