//! Handler-gated: init a `StateCommitment` PDA at `(state_machine, height)`.

use anchor_lang::prelude::*;

use crate::error::HyperbridgeError;
use crate::state::{HostConfig, StateCommitment};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct StoreStateCommitmentParams {
    pub state_machine: u32,
    pub height: u64,
    pub state_root: [u8; 32],
    /// Source-chain block timestamp; 0 if not extracted.
    pub timestamp: u64,
}

#[derive(Accounts)]
#[instruction(params: StoreStateCommitmentParams)]
pub struct StoreStateCommitment<'info> {
    /// CHECK: identity-checked against the configured handler PDA.
    pub handler_authority: Signer<'info>,

    #[account(mut)]
    pub rent_payer: Signer<'info>,

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
        payer = rent_payer,
        space = 8 + StateCommitment::INIT_SPACE,
        seeds = [
            StateCommitment::SEED_PREFIX,
            params.state_machine.to_le_bytes().as_ref(),
            params.height.to_le_bytes().as_ref(),
        ],
        bump,
    )]
    pub state_commitment: Account<'info, StateCommitment>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<StoreStateCommitment>, p: StoreStateCommitmentParams) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let sc = &mut ctx.accounts.state_commitment;
    sc.state_machine = p.state_machine;
    sc.height = p.height;
    sc.state_root = p.state_root;
    sc.timestamp = p.timestamp;
    sc.updated_at = now;
    sc.vetoed = false;
    sc.vetoed_by = Pubkey::default();
    sc.bump = ctx.bumps.state_commitment;
    Ok(())
}
