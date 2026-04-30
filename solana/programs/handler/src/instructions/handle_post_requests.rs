//! `HandlerV2.handlePostRequests(host, message)` — drives ismp-core.
//!
//! Wire format on `Proof.proof`: `SCALE((storage_key, proof_nodes))`.

use anchor_lang::prelude::*;
use ismp::messaging::hash_request;
use ismp::router::{PostRequest, Request};
use parity_scale_codec::{Decode, Encode};
use primitive_types::H256;

use host::program::Host;
use host::state::{HostConfig, RelayerFeeVault, RequestReceipt, StateCommitment};

use crate::error::HandlerError;
use crate::ismp::{CommitmentSnapshot, SolanaHostFacade, SOLANA_STATE_MACHINE};

/// Solana program ids are exactly 32 bytes; PostRequest.to is variable.
const SOLANA_ADDRESS_LEN: usize = 32;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct HandlePostRequestsParams {
    pub state_machine: u32,
    pub height: u64,
    pub storage_key: Vec<u8>,
    pub storage_proof: Vec<Vec<u8>>,
    /// SCALE-encoded `ismp::router::PostRequest`.
    pub post_request_body: Vec<u8>,
    /// `hash_request(Request::Post(post))` — the canonical ismp-core
    /// commitment that also seeds the receipt PDA.
    pub commitment: [u8; 32],
}

#[derive(Accounts)]
#[instruction(params: HandlePostRequestsParams)]
pub struct HandlePostRequests<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,

    /// CHECK: PDA address verified by Anchor via seeds.
    #[account(seeds = [b"handler_authority"], bump)]
    pub handler_authority: UncheckedAccount<'info>,

    pub state_commitment: Account<'info, StateCommitment>,

    pub host_program: Program<'info, Host>,

    #[account(mut)]
    pub host_config: Account<'info, HostConfig>,

    /// CHECK: validated by host's `init` constraint.
    #[account(mut)]
    pub request_receipt: UncheckedAccount<'info>,

    /// CHECK: validated by host's `init_if_needed` constraint.
    #[account(mut)]
    pub fee_vault: UncheckedAccount<'info>,

    /// Destination program — handler-verified against decoded `post.to`.
    /// CHECK: handler-verified.
    pub dest_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler<'info>(
    ctx: Context<'info, HandlePostRequests<'info>>,
    p: HandlePostRequestsParams,
) -> Result<()> {
    let post = PostRequest::decode(&mut p.post_request_body.as_slice())
        .map_err(|_| error!(HandlerError::InvalidPostRequestFormat))?;
    let req = Request::Post(post.clone());
    let computed = hash_request::<SolanaHostFacade>(&req);
    require!(computed.0 == p.commitment, HandlerError::CommitmentMismatch);

    let sm_bytes = p.state_machine.to_le_bytes();
    let height_bytes = p.height.to_le_bytes();
    let (expected_pda, _bump) = Pubkey::find_program_address(
        &[StateCommitment::SEED_PREFIX, sm_bytes.as_slice(), height_bytes.as_slice()],
        &ctx.accounts.host_program.key(),
    );
    require_keys_eq!(
        ctx.accounts.state_commitment.key(),
        expected_pda,
        HandlerError::StateCommitmentSeedsMismatch
    );
    require!(
        !ctx.accounts.state_commitment.vetoed,
        HandlerError::StateCommitmentVetoed
    );

    require!(
        post.to.len() == SOLANA_ADDRESS_LEN,
        HandlerError::InvalidPostRequestFormat
    );
    let mut to_bytes = [0u8; SOLANA_ADDRESS_LEN];
    to_bytes.copy_from_slice(&post.to);
    require_keys_eq!(
        ctx.accounts.dest_program.key(),
        Pubkey::new_from_array(to_bytes),
        HandlerError::DestProgramMismatch
    );

    let (expected_receipt, _) = Pubkey::find_program_address(
        &[RequestReceipt::SEED_PREFIX, p.commitment.as_ref()],
        &ctx.accounts.host_program.key(),
    );
    require_keys_eq!(
        ctx.accounts.request_receipt.key(),
        expected_receipt,
        HandlerError::StateCommitmentSeedsMismatch
    );
    let (expected_vault, _) = Pubkey::find_program_address(
        &[RelayerFeeVault::SEED_PREFIX, ctx.accounts.relayer.key().as_ref()],
        &ctx.accounts.host_program.key(),
    );
    require_keys_eq!(
        ctx.accounts.fee_vault.key(),
        expected_vault,
        HandlerError::StateCommitmentSeedsMismatch
    );

    let host_cfg = &ctx.accounts.host_config;
    let now = Clock::get()?.unix_timestamp;
    let sc = &ctx.accounts.state_commitment;

    let mut state_commitments = std::collections::BTreeMap::new();
    let mut state_commitment_accts = std::collections::BTreeMap::new();
    let key = (p.state_machine, p.height);
    state_commitments.insert(
        key,
        CommitmentSnapshot {
            state_root: sc.state_root,
            timestamp_secs: sc.timestamp,
            updated_at: sc.updated_at,
            vetoed: sc.vetoed,
        },
    );
    state_commitment_accts.insert(key, ctx.accounts.state_commitment.to_account_info());

    let mut request_receipt_accts = std::collections::BTreeMap::new();
    let mut request_receipts = std::collections::BTreeMap::new();
    let commitment_h256 = H256::from(p.commitment);
    request_receipt_accts.insert(commitment_h256, ctx.accounts.request_receipt.to_account_info());
    request_receipts.insert(commitment_h256, false);

    let facade = SolanaHostFacade {
        host_state_machine: SOLANA_STATE_MACHINE,
        consensus_client_id: host_cfg.consensus_client_id,
        frozen: host_cfg.frozen,
        challenge_period_secs: host_cfg.challenge_period,
        unbonding_period_secs: host_cfg.unbonding_period,
        consensus_state_payload: None,
        consensus_last_updated: None,
        state_commitments,
        request_receipts,
        now_unix_secs: now,
        sp1_vkey_hash: [0u8; 32],
        commit_header_index: None,
        host_program_id: ctx.accounts.host_program.key(),
        host_program: ctx.accounts.host_program.to_account_info(),
        host_config: ctx.accounts.host_config.to_account_info(),
        consensus_state_acct: None,
        state_commitment_accts,
        request_receipt_accts,
        fee_vault: Some(ctx.accounts.fee_vault.to_account_info()),
        relayer: ctx.accounts.relayer.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        handler_authority: ctx.accounts.handler_authority.to_account_info(),
        handler_authority_bump: ctx.bumps.handler_authority,
        dest_program: Some(ctx.accounts.dest_program.to_account_info()),
        dest_remaining_accounts: ctx.remaining_accounts.to_vec(),
    };

    let wire_proof: (Vec<u8>, Vec<Vec<u8>>) = (p.storage_key.clone(), p.storage_proof.clone());
    let proof_bytes = wire_proof.encode();

    let signer_bytes = ctx.accounts.relayer.key().to_bytes().to_vec();
    let request_message = ismp::messaging::RequestMessage {
        requests: vec![post.clone()],
        proof: ismp::messaging::Proof {
            height: ismp::consensus::StateMachineHeight {
                id: ismp::consensus::StateMachineId {
                    state_id: ismp::host::StateMachine::Polkadot(p.state_machine),
                    consensus_state_id: host_cfg.consensus_client_id,
                },
                height: p.height,
            },
            proof: proof_bytes,
        },
        signer: signer_bytes,
    };
    ismp::handlers::handle_incoming_message(
        &facade,
        ismp::messaging::Message::Request(request_message),
    )
    .map_err(|e| {
        msg!("ismp handle_incoming_message error: {:?}", e);
        error!(HandlerError::InvalidStorageProof)
    })?;

    msg!(
        "post_request verified + dispatched: commitment=0x{}",
        crate::util::hex32(&p.commitment)
    );
    Ok(())
}
