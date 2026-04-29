//! Drains the relayer's pending counter. SPL transfer is deferred —
//! Hyperbridge-side fee funding into the host vault isn't defined yet.

use anchor_lang::prelude::*;

use crate::error::HyperbridgeError;
use crate::state::RelayerFeeVault;

#[derive(Accounts)]
pub struct WithdrawFees<'info> {
    pub relayer: Signer<'info>,

    #[account(
        mut,
        seeds = [RelayerFeeVault::SEED_PREFIX, relayer.key().as_ref()],
        bump = vault.bump,
        constraint = vault.relayer == relayer.key() @ HyperbridgeError::Unauthorized,
    )]
    pub vault: Account<'info, RelayerFeeVault>,
}

pub(crate) fn handler(ctx: Context<WithdrawFees>) -> Result<()> {
    let v = &mut ctx.accounts.vault;
    let amount = v.pending;
    require!(amount > 0, HyperbridgeError::NothingToWithdraw);

    v.pending = 0;
    v.last_accrual = Clock::get()?.unix_timestamp;

    msg!(
        "withdraw_fees: relayer={}, pending_drained={} (SPL transfer placeholder)",
        ctx.accounts.relayer.key(),
        amount
    );
    Ok(())
}
