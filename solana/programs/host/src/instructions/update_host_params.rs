//! Admin-only updates to the runtime-tunable subset of `HostConfig`.
//! `admin`, `host_state_machine`, `hyperbridge_id`, `consensus_client_id`
//! are immutable post-init; `frozen` lives in `set_frozen_state`.

use anchor_lang::prelude::*;

use crate::error::HyperbridgeError;
use crate::state::HostConfig;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateHostParamsParams {
    pub challenge_period: u64,
    pub unbonding_period: u64,
    pub default_timeout: u64,
    pub fee_token_mint: Pubkey,
    pub per_byte_fee: u64,
}

#[derive(Accounts)]
pub struct UpdateHostParams<'info> {
    #[account(
        constraint = admin.key() == host_config.admin @ HyperbridgeError::Unauthorized,
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [HostConfig::SEED],
        bump = host_config.bump,
    )]
    pub host_config: Account<'info, HostConfig>,
}

pub(crate) fn handler(ctx: Context<UpdateHostParams>, p: UpdateHostParamsParams) -> Result<()> {
    let cfg = &mut ctx.accounts.host_config;
    cfg.challenge_period = p.challenge_period;
    cfg.unbonding_period = p.unbonding_period;
    cfg.default_timeout = p.default_timeout;
    cfg.fee_token_mint = p.fee_token_mint;
    cfg.per_byte_fee = p.per_byte_fee;
    msg!(
        "host_params updated: challenge_period={}, default_timeout={}, per_byte_fee={}",
        p.challenge_period,
        p.default_timeout,
        p.per_byte_fee
    );
    Ok(())
}
