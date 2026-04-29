//! Admin-only kill switch. While `HostConfig.frozen`, handler-gated
//! instructions return `HostFrozen`.

use anchor_lang::prelude::*;

use crate::error::HyperbridgeError;
use crate::state::HostConfig;

#[derive(Accounts)]
pub struct SetFrozenState<'info> {
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

pub(crate) fn handler(ctx: Context<SetFrozenState>, frozen: bool) -> Result<()> {
    ctx.accounts.host_config.frozen = frozen;
    msg!("host frozen={}", frozen);
    Ok(())
}
