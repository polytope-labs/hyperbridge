//! Hyperbridge host — pure state owner. Mutating primitives are gated
//! on the configured handler's `[b"handler_authority"]` PDA signer so a
//! verification-logic upgrade is a `set_handler` flip, not a state
//! migration. Inbound-only.
//!
//! The `IsmpHost` trait impl lives in the **handler** crate, not here —
//! see `handler::ismp::host_facade::SolanaHostFacade`. A Solana
//! `#[program]` module isn't a Rust value with `self`, so the trait can't
//! be implemented on it directly. The facade is built per-tx from the
//! handler's Anchor `Context`: reads hit the PDAs directly, writes
//! (`store_consensus_state`, `store_state_machine_commitment`,
//! `store_request_receipt`, …) become CPIs back into this program,
//! authenticated by the `[b"handler_authority"]` signer PDA.

use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

use crate::instructions::*;

// Anchor's `cpi` feature expects param types reachable at the crate root.
pub use instructions::dispatch_incoming::DispatchIncomingParams;
pub use instructions::store_state_commitment::StoreStateCommitmentParams;

// Placeholder. Run `anchor keys sync` before deploy.
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod host {
    use super::*;

    pub fn initialize_host(ctx: Context<InitializeHost>, params: InitializeHostParams) -> Result<()> {
        instructions::initialize_host::handler(ctx, params)
    }

    pub fn set_consensus_state(
        ctx: Context<SetConsensusState>,
        params: SetConsensusStateParams,
    ) -> Result<()> {
        instructions::set_consensus_state::handler(ctx, params)
    }

    pub fn set_handler(ctx: Context<SetHandler>, new_handler: Pubkey) -> Result<()> {
        instructions::set_handler::handler(ctx, new_handler)
    }

    pub fn set_frozen_state(ctx: Context<SetFrozenState>, frozen: bool) -> Result<()> {
        instructions::set_frozen_state::handler(ctx, frozen)
    }

    pub fn update_host_params(
        ctx: Context<UpdateHostParams>,
        params: UpdateHostParamsParams,
    ) -> Result<()> {
        instructions::update_host_params::handler(ctx, params)
    }

    pub fn veto_state_commitment(ctx: Context<VetoStateCommitment>) -> Result<()> {
        instructions::veto_state_commitment::handler(ctx)
    }

    pub fn store_consensus_state(
        ctx: Context<StoreConsensusState>,
        new_state: Vec<u8>,
    ) -> Result<()> {
        instructions::store_consensus_state::handler(ctx, new_state)
    }

    pub fn store_state_commitment(
        ctx: Context<StoreStateCommitment>,
        params: StoreStateCommitmentParams,
    ) -> Result<()> {
        instructions::store_state_commitment::handler(ctx, params)
    }

    pub fn dispatch_incoming<'info>(
        ctx: Context<'info, DispatchIncoming<'info>>,
        params: DispatchIncomingParams,
    ) -> Result<()> {
        instructions::dispatch_incoming::handler(ctx, params)
    }

    pub fn withdraw_fees(ctx: Context<WithdrawFees>) -> Result<()> {
        instructions::withdraw_fees::handler(ctx)
    }
}
