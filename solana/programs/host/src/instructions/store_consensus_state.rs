//! Handler-gated: overwrite `ConsensusState.state` with handler-verified
//! bytes.

use anchor_lang::prelude::*;

use crate::error::HyperbridgeError;
use crate::state::{ConsensusState, HostConfig};

#[derive(Accounts)]
pub struct StoreConsensusState<'info> {
    /// CHECK: identity-checked against the configured handler PDA.
    pub handler_authority: Signer<'info>,

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
        mut,
        seeds = [ConsensusState::SEED_PREFIX, consensus_state.id.as_ref()],
        bump = consensus_state.bump,
        constraint = !consensus_state.frozen @ HyperbridgeError::ConsensusClientFrozen,
    )]
    pub consensus_state: Account<'info, ConsensusState>,
}

pub fn handler(ctx: Context<StoreConsensusState>, new_state: Vec<u8>) -> Result<()> {
    require!(!new_state.is_empty(), HyperbridgeError::EmptyConsensusState);
    require!(
        new_state.len() <= ConsensusState::MAX_STATE_LEN,
        HyperbridgeError::ConsensusStateTooLarge
    );

    let cs = &mut ctx.accounts.consensus_state;
    cs.state = new_state;
    cs.last_updated = Clock::get()?.unix_timestamp;
    Ok(())
}
