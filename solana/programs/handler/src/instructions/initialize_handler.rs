//! One-time setup of the `HandlerState` singleton.

use anchor_lang::prelude::*;

use crate::state::HandlerState;

#[derive(Accounts)]
pub struct InitializeHandler<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + HandlerState::INIT_SPACE,
        seeds = [HandlerState::SEED],
        bump,
    )]
    pub handler_state: Account<'info, HandlerState>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<InitializeHandler>) -> Result<()> {
    let s = &mut ctx.accounts.handler_state;
    s.current_epoch = 0;
    s.bump = ctx.bumps.handler_state;
    Ok(())
}
