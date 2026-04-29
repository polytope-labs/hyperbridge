use anchor_lang::prelude::*;

use crate::error::HyperbridgeError;
use crate::state::{ConsensusState, HostConfig};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SetConsensusStateParams {
    /// Consensus client identifier (`b"BEFY"` for BEEFY).
    pub id: [u8; 4],
    /// SCALE-encoded consensus payload (BEEFY trust anchor).
    pub state: Vec<u8>,
}

#[derive(Accounts)]
#[instruction(params: SetConsensusStateParams)]
pub struct SetConsensusState<'info> {
    #[account(
        mut,
        constraint = admin.key() == host_config.admin @ HyperbridgeError::Unauthorized,
    )]
    pub admin: Signer<'info>,

    #[account(
        seeds = [HostConfig::SEED],
        bump = host_config.bump,
        constraint = !host_config.frozen @ HyperbridgeError::HostFrozen,
    )]
    pub host_config: Account<'info, HostConfig>,

    #[account(
        init,
        payer = admin,
        space = ConsensusState::SPACE,
        seeds = [ConsensusState::SEED_PREFIX, params.id.as_ref()],
        bump,
    )]
    pub consensus_state: Account<'info, ConsensusState>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<SetConsensusState>, p: SetConsensusStateParams) -> Result<()> {
    require!(!p.state.is_empty(), HyperbridgeError::EmptyConsensusState);
    require!(
        p.state.len() <= ConsensusState::MAX_STATE_LEN,
        HyperbridgeError::ConsensusStateTooLarge
    );

    let cs = &mut ctx.accounts.consensus_state;
    cs.id = p.id;
    cs.last_updated = Clock::get()?.unix_timestamp;
    cs.frozen = false;
    cs.state = p.state;
    cs.bump = ctx.bumps.consensus_state;
    Ok(())
}
