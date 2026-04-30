//! Init `RequestReceipt` (replay guard), accrue per-byte fee, CPI the
//! incoming message into the destination program.
//!
//! Wire format on the destination CPI's instruction data mirrors EVM's
//! `IncomingPostRequest{request, relayer}`:
//!
//! ```text
//! [0..8]    ISMP_INBOUND_TAG = b"ismp_msg"
//! [8..40]   relayer pubkey (32B, raw)
//! [40..]    SCALE-encoded ismp::router::PostRequest
//! ```
//!
//! Carrying the full `PostRequest` (not just `body`) lets the dest
//! program authenticate `source` and `from`, read `nonce`/timeout,
//! etc. — without these, an app couldn't tell who sent the message or
//! whether it originated from a trusted gateway instance.
//!
//! Any `remaining_accounts` passed by the relayer on the outer handler
//! instruction are forwarded verbatim onto the destination CPI — this
//! is what lets the dest program mutate its own PDAs, move SPL tokens,
//! or chain further CPIs in response to an inbound message.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

use crate::error::HyperbridgeError;
use crate::state::{HostConfig, RelayerFeeVault, RequestReceipt};

/// Tag prepended to dest-program ix data so non-Anchor destinations can
/// dispatch on it.
pub const ISMP_INBOUND_TAG: [u8; 8] = *b"ismp_msg";

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DispatchIncomingParams {
    pub commitment: [u8; 32],
    /// SCALE-encoded `ismp::router::PostRequest` — the full inbound
    /// message, forwarded opaquely to the destination program.
    pub request: Vec<u8>,
    /// Length of just the application body inside `request`. Used for
    /// per-byte fee accounting so encoded routing metadata
    /// (source/dest/from/to/nonce/timeout) doesn't inflate the bill.
    /// The handler is the only authorized caller (gated on
    /// `handler_authority` PDA), so this is a trusted input.
    pub body_len: u32,
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

// Lifetimes are tied together so `dest_program`'s AccountInfo and the
// forwarded `remaining_accounts` share the same `'info` when assembled
// into the inner CPI's account-info slice.
pub fn handler<'info>(
    ctx: Context<'info, DispatchIncoming<'info>>,
    p: DispatchIncomingParams,
) -> Result<()> {
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
        .saturating_mul(p.body_len as u64);
    v.pending = v.pending.saturating_add(fee);
    v.last_accrual = now;

    // Wire layout: tag (8B) || relayer (32B) || encoded PostRequest.
    let relayer_bytes = ctx.accounts.relayer.key().to_bytes();
    let mut ix_data = Vec::with_capacity(8 + 32 + p.request.len());
    ix_data.extend_from_slice(&ISMP_INBOUND_TAG);
    ix_data.extend_from_slice(&relayer_bytes);
    ix_data.extend_from_slice(&p.request);

    // Mirror each forwarded account's signer/writable flags onto the
    // inner ix. Solana's CPI machinery enforces that flags can't
    // strengthen across the boundary (e.g. an account not signed in the
    // outer tx can't become a signer here unless it's a PDA we sign for
    // via invoke_signed), so propagating verbatim is safe.
    let metas: Vec<AccountMeta> = ctx
        .remaining_accounts
        .iter()
        .map(|ai| AccountMeta {
            pubkey: *ai.key,
            is_signer: ai.is_signer,
            is_writable: ai.is_writable,
        })
        .collect();

    let ix = Instruction {
        program_id: ctx.accounts.dest_program.key(),
        accounts: metas,
        data: ix_data,
    };

    let mut infos = Vec::with_capacity(ctx.remaining_accounts.len() + 1);
    infos.push(ctx.accounts.dest_program.to_account_info());
    infos.extend(ctx.remaining_accounts.iter().cloned());

    invoke(&ix, &infos)?;

    msg!(
        "ismp post request delivered: request_len={}, body_len={}, fee={}, forwarded_accounts={}",
        p.request.len(),
        p.body_len,
        fee,
        ctx.remaining_accounts.len()
    );
    Ok(())
}
