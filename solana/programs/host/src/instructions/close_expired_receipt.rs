//! Permissionless rent reclaim once a receipt is older than
//! `default_timeout + SAFETY_MARGIN`. Refund goes to the receipt's
//! recorded relayer, not the caller.

use anchor_lang::prelude::*;

use crate::error::HyperbridgeError;
use crate::state::{HostConfig, RequestReceipt};

/// Buffer past `default_timeout` to absorb source-chain re-org +
/// finality skew before a receipt becomes closable.
const SAFETY_MARGIN: i64 = 60 * 60;

#[derive(Accounts)]
pub struct CloseExpiredReceipt<'info> {
    #[account(mut)]
    pub anyone: Signer<'info>,

    #[account(
        seeds = [HostConfig::SEED],
        bump = host_config.bump,
    )]
    pub host_config: Account<'info, HostConfig>,

    /// Refund target — must match `request_receipt.relayer`.
    #[account(mut)]
    pub relayer_refund: SystemAccount<'info>,

    #[account(
        mut,
        close = relayer_refund,
        constraint = request_receipt.relayer == relayer_refund.key()
            @ HyperbridgeError::ReceiptRelayerMismatch,
    )]
    pub request_receipt: Account<'info, RequestReceipt>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<CloseExpiredReceipt>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let min_age = ctx.accounts.host_config.default_timeout as i64 + SAFETY_MARGIN;

    require!(
        now.saturating_sub(ctx.accounts.request_receipt.received_at) >= min_age,
        HyperbridgeError::ReceiptNotYetExpired
    );

    Ok(())
}
