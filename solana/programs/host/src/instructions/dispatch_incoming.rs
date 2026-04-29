//! Init `RequestReceipt` (replay guard), accrue per-byte fee, CPI body
//! into the destination program.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke;

use crate::error::HyperbridgeError;
use crate::state::{HostConfig, RelayerFeeVault, RequestReceipt};

/// Tag prepended to dest-program ix data so non-Anchor destinations can
/// dispatch on it.
pub const ISMP_INBOUND_TAG: [u8; 8] = *b"ismp_msg";

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DispatchIncomingParams {
    pub commitment: [u8; 32],
    pub body: Vec<u8>,
}

#[derive(Accounts)]
#[instruction(params: DispatchIncomingParams)]
pub struct DispatchIncoming<'info> {
    /// CHECK: identity-checked against the configured handler PDA.
    pub handler_authority: Signer<'info>,

    #[account(mut)]
    pub relayer: Signer<'info>,

    #[account(
        seeds = [HostConfig::SEED],
        bump = host_config.bump,
        constraint = !host_config.frozen @ HyperbridgeError::HostFrozen,
        constraint = host_config.handler_program != Pubkey::default()
            @ HyperbridgeError::HandlerNotConfigured,
        constraint = handler_authority.key() ==
            Pubkey::find_program_address(
                &[HostConfig::HANDLER_AUTHORITY_SEED],
                &host_config.handler_program,
            ).0
            @ HyperbridgeError::UnauthorizedHandler,
    )]
    pub host_config: Account<'info, HostConfig>,

    #[account(
        init,
        payer = relayer,
        space = 8 + RequestReceipt::INIT_SPACE,
        seeds = [RequestReceipt::SEED_PREFIX, params.commitment.as_ref()],
        bump,
    )]
    pub request_receipt: Account<'info, RequestReceipt>,

    #[account(
        init_if_needed,
        payer = relayer,
        space = 8 + RelayerFeeVault::INIT_SPACE,
        seeds = [RelayerFeeVault::SEED_PREFIX, relayer.key().as_ref()],
        bump,
    )]
    pub fee_vault: Account<'info, RelayerFeeVault>,

    /// CHECK: handler-verified.
    pub dest_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<DispatchIncoming>, p: DispatchIncomingParams) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let receipt = &mut ctx.accounts.request_receipt;
    receipt.commitment = p.commitment;
    receipt.relayer = ctx.accounts.relayer.key();
    receipt.received_at = now;
    receipt.bump = ctx.bumps.request_receipt;

    let v = &mut ctx.accounts.fee_vault;
    if v.relayer == Pubkey::default() {
        v.relayer = ctx.accounts.relayer.key();
        v.bump = ctx.bumps.fee_vault;
    }
    let fee = ctx
        .accounts
        .host_config
        .per_byte_fee
        .saturating_mul(p.body.len() as u64);
    v.pending = v.pending.saturating_add(fee);
    v.last_accrual = now;

    let mut ix_data = Vec::with_capacity(8 + p.body.len());
    ix_data.extend_from_slice(&ISMP_INBOUND_TAG);
    ix_data.extend_from_slice(&p.body);
    let ix = Instruction {
        program_id: ctx.accounts.dest_program.key(),
        accounts: Vec::new(),
        data: ix_data,
    };
    invoke(&ix, &[ctx.accounts.dest_program.to_account_info()])?;

    msg!(
        "ismp post request delivered: commitment_len={}, body_len={}, fee={}",
        p.commitment.len(),
        p.body.len(),
        fee
    );
    Ok(())
}
