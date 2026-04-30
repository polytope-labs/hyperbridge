//! Hyperbridge handler — inbound-only verification orchestrator,
//! gated to host state mutations via `[b"handler_authority"]` PDA CPIs.

use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod ismp;
pub mod state;
pub mod util;
pub mod verifier;

use crate::instructions::*;

// Placeholder. Run `anchor keys sync` before deploy.
declare_id!("Han1errrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrrr");

#[program]
pub mod handler {
    use super::*;

    pub fn initialize_handler(ctx: Context<InitializeHandler>) -> Result<()> {
        instructions::initialize_handler::handler(ctx)
    }

    pub fn handle_consensus(
        ctx: Context<HandleConsensus>,
        params: HandleConsensusParams,
    ) -> Result<()> {
        instructions::handle_consensus::handler(ctx, params)
    }

    pub fn handle_post_requests<'info>(
        ctx: Context<'info, HandlePostRequests<'info>>,
        params: HandlePostRequestsParams,
    ) -> Result<()> {
        instructions::handle_post_requests::handler(ctx, params)
    }
}
