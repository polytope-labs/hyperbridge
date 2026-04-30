//! `HandlerV2.handleConsensus(host, proof)` — drives ismp-core,
//! preserves V2 epoch attribution as a post-hook.

use anchor_lang::prelude::*;
use parity_scale_codec::Decode;

use sp1_beefy_verifier::{Sp1BeefyProof, PROOF_TYPE_SP1};

use host::program::Host;
use host::state::{ConsensusState, HostConfig};

use crate::error::HandlerError;
use crate::ismp::{SolanaHostFacade, SOLANA_STATE_MACHINE};
use crate::state::{EpochRecord, HandlerState};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct HandleConsensusParams {
    /// `[PROOF_TYPE_SP1=0x01] ++ SCALE(Sp1BeefyProof)`.
    pub message: Vec<u8>,
    pub sp1_vkey_hash: [u8; 32],
    /// Selects which parachain header gets a StateCommitment PDA in
    /// this tx.
    pub commit_header_index: u32,
    /// Pre-computed `proof.validator_set_id`; seeds the EpochRecord PDA.
    pub authority_set_id: u64,
}

#[derive(Accounts)]
#[instruction(params: HandleConsensusParams)]
pub struct HandleConsensus<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,

    #[account(
        mut,
        seeds = [HandlerState::SEED],
        bump = handler_state.bump,
    )]
    pub handler_state: Account<'info, HandlerState>,

    #[account(
        init_if_needed,
        payer = relayer,
        space = 8 + EpochRecord::INIT_SPACE,
        seeds = [EpochRecord::SEED_PREFIX, params.authority_set_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub epoch_record: Account<'info, EpochRecord>,

    /// CHECK: PDA address verified by Anchor via seeds.
    #[account(seeds = [b"handler_authority"], bump)]
    pub handler_authority: UncheckedAccount<'info>,

    pub host_program: Program<'info, Host>,

    #[account(mut)]
    pub host_config: Account<'info, HostConfig>,

    #[account(mut)]
    pub consensus_state: Account<'info, ConsensusState>,

    /// CHECK: validated by host's PDA-init constraint.
    #[account(mut)]
    pub state_commitment: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<HandleConsensus>, p: HandleConsensusParams) -> Result<()> {
    require!(
        !p.message.is_empty() && p.message[0] == PROOF_TYPE_SP1,
        HandlerError::WrongProofType
    );
    let mut input = &p.message[1..];
    let pre_proof = Sp1BeefyProof::decode(&mut input)
        .map_err(|_| error!(HandlerError::InvalidSp1BeefyProof))?;
    require!(!pre_proof.headers.is_empty(), HandlerError::NoHeadersInProof);
    let idx = p.commit_header_index as usize;
    let header = pre_proof
        .headers
        .get(idx)
        .ok_or(error!(HandlerError::CommitHeaderIndexOutOfRange))?;
    require!(
        pre_proof.validator_set_id == p.authority_set_id,
        HandlerError::UnknownAuthoritySet
    );
    let (header_number, _) = crate::verifier::beefy::extract_header_prefix(&header.header)?;
    let para_id = header.para_id;
    let height = header_number as u64;

    let host_cfg = &ctx.accounts.host_config;
    let consensus_state_payload = ctx.accounts.consensus_state.state.clone();
    let consensus_last_updated = ctx.accounts.consensus_state.last_updated;
    let now = Clock::get()?.unix_timestamp;

    // Empty `state_commitments` for this key — core's dup-check returns
    // Err and proceeds to call `store_state_machine_commitment`, which
    // CPIs the host to init the PDA.
    let state_commitments = std::collections::BTreeMap::new();
    let mut state_commitment_accts = std::collections::BTreeMap::new();
    state_commitment_accts.insert(
        (para_id, height),
        ctx.accounts.state_commitment.to_account_info(),
    );

    let facade = SolanaHostFacade {
        host_state_machine: SOLANA_STATE_MACHINE,
        consensus_client_id: host_cfg.consensus_client_id,
        frozen: host_cfg.frozen,
        challenge_period_secs: host_cfg.challenge_period,
        unbonding_period_secs: host_cfg.unbonding_period,
        consensus_state_payload: Some(consensus_state_payload),
        consensus_last_updated: Some(consensus_last_updated),
        state_commitments,
        request_receipts: std::collections::BTreeMap::new(),
        now_unix_secs: now,
        sp1_vkey_hash: p.sp1_vkey_hash,
        commit_header_index: Some(idx),
        host_program_id: ctx.accounts.host_program.key(),
        host_program: ctx.accounts.host_program.to_account_info(),
        host_config: ctx.accounts.host_config.to_account_info(),
        consensus_state_acct: Some(ctx.accounts.consensus_state.to_account_info()),
        state_commitment_accts,
        request_receipt_accts: std::collections::BTreeMap::new(),
        fee_vault: None,
        relayer: ctx.accounts.relayer.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        handler_authority: ctx.accounts.handler_authority.to_account_info(),
        handler_authority_bump: ctx.bumps.handler_authority,
        dest_program: None,
        // Consensus path never CPIs into a destination app.
        dest_remaining_accounts: Vec::new(),
    };

    let signer_bytes = ctx.accounts.relayer.key().to_bytes().to_vec();
    let consensus_msg = ismp::messaging::ConsensusMessage {
        consensus_proof: p.message.clone(),
        consensus_state_id: host_cfg.consensus_client_id,
        signer: signer_bytes,
    };
    ismp::handlers::handle_incoming_message(
        &facade,
        ismp::messaging::Message::Consensus(consensus_msg),
    )
    .map_err(|e| {
        msg!("ismp handle_incoming_message error: {:?}", e);
        error!(HandlerError::Sp1VerificationFailed)
    })?;

    // V2 epoch attribution — first relayer to introduce a new
    // `validator_set_id` wins. Not in ismp-core; bolt-on here.
    if pre_proof.validator_set_id > ctx.accounts.handler_state.current_epoch {
        ctx.accounts.handler_state.current_epoch = pre_proof.validator_set_id;
        let er = &mut ctx.accounts.epoch_record;
        if er.relayer == Pubkey::default() {
            er.authority_set_id = pre_proof.validator_set_id;
            er.relayer = ctx.accounts.relayer.key();
            er.recorded_at = now;
            er.bump = ctx.bumps.epoch_record;
            msg!(
                "NewEpoch: authority_set_id={}, relayer={}",
                pre_proof.validator_set_id,
                ctx.accounts.relayer.key()
            );
        }
    }

    Ok(())
}
