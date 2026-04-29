//! Admin-only veto. Sets `StateCommitment.vetoed = true` so future
//! `handle_post_requests` against it fail closed. One-way — no unveto.

use anchor_lang::prelude::*;

use crate::error::HyperbridgeError;
use crate::state::{HostConfig, StateCommitment};

#[derive(Accounts)]
pub struct VetoStateCommitment<'info> {
    #[account(
        constraint = admin.key() == host_config.admin @ HyperbridgeError::Unauthorized,
    )]
    pub admin: Signer<'info>,

    #[account(
        seeds = [HostConfig::SEED],
        bump = host_config.bump,
    )]
    pub host_config: Account<'info, HostConfig>,

    #[account(mut)]
    pub state_commitment: Account<'info, StateCommitment>,
}

pub(crate) fn handler(ctx: Context<VetoStateCommitment>) -> Result<()> {
    let sc = &mut ctx.accounts.state_commitment;
    require!(!sc.vetoed, HyperbridgeError::AlreadyVetoed);
    sc.vetoed = true;
    sc.vetoed_by = ctx.accounts.admin.key();

    msg!(
        "state_commitment vetoed: state_machine={}, height={}, by={}",
        sc.state_machine,
        sc.height,
        ctx.accounts.admin.key()
    );
    Ok(())
}
