//! Admin-only swap of `HostConfig.handler_program`. State PDAs survive.

use anchor_lang::prelude::*;

use crate::error::HyperbridgeError;
use crate::state::HostConfig;

#[derive(Accounts)]
pub struct SetHandler<'info> {
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

pub(crate) fn handler(ctx: Context<SetHandler>, new_handler: Pubkey) -> Result<()> {
    let prev = ctx.accounts.host_config.handler_program;
    ctx.accounts.host_config.handler_program = new_handler;
    msg!("handler_program: {} -> {}", prev, new_handler);
    Ok(())
}
