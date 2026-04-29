//! `IsmpHost` impl for a Solana handler-program tx.

extern crate alloc;
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::ToString,
    vec,
    vec::Vec,
};
use core::time::Duration;

// Anchor's prelude re-exports `Result<T>` as a 1-arg alias, which would
// shadow the 2-arg shape `IsmpHost` returns.
use anchor_lang::context::CpiContext;
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::pubkey::Pubkey;
use primitive_types::H256;
use sha3::{Digest, Keccak256 as Sha3Keccak};

use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId,
        StateCommitment as IsmpStateCommitment, StateMachineHeight, StateMachineId,
    },
    error::Error as IsmpError,
    host::{IsmpHost, StateMachine},
    messaging::{hash_request, Keccak256},
    router::{IsmpRouter, PostResponse, Request, Response},
};

use super::{consensus_client::Sp1BeefyConsensusClient, router::SolanaRouter};

/// Placeholder until upstream `ismp-core` adds a dedicated
/// `StateMachine::Solana` variant.
pub const SOLANA_STATE_MACHINE: StateMachine = StateMachine::Substrate(*b"sola");

#[derive(Clone, Copy)]
pub struct CommitmentSnapshot {
    pub state_root: [u8; 32],
    pub timestamp_secs: u64,
    pub updated_at: i64,
    pub vetoed: bool,
}

pub struct SolanaHostFacade<'info> {
    pub host_state_machine: StateMachine,
    pub consensus_client_id: ConsensusClientId,
    pub frozen: bool,
    pub challenge_period_secs: u64,
    pub unbonding_period_secs: u64,

    pub consensus_state_payload: Option<Vec<u8>>,
    pub consensus_last_updated: Option<i64>,

    pub state_commitments: BTreeMap<(u32, u64), CommitmentSnapshot>,
    pub request_receipts: BTreeMap<H256, bool>,

    pub now_unix_secs: i64,
    pub sp1_vkey_hash: [u8; 32],
    /// One-PDA-per-tx model: filter `verify_consensus` to a single header.
    pub commit_header_index: Option<usize>,

    pub host_program_id: Pubkey,
    pub host_program: AccountInfo<'info>,
    pub host_config: AccountInfo<'info>,
    pub consensus_state_acct: Option<AccountInfo<'info>>,
    pub state_commitment_accts: BTreeMap<(u32, u64), AccountInfo<'info>>,
    pub request_receipt_accts: BTreeMap<H256, AccountInfo<'info>>,
    pub fee_vault: Option<AccountInfo<'info>>,
    pub relayer: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub handler_authority: AccountInfo<'info>,
    pub handler_authority_bump: u8,
    pub dest_program: Option<AccountInfo<'info>>,
}

impl<'info> Keccak256 for SolanaHostFacade<'info> {
    fn keccak256(bytes: &[u8]) -> H256 {
        let mut hasher = Sha3Keccak::new();
        hasher.update(bytes);
        H256::from_slice(hasher.finalize().as_slice())
    }
}

fn outbound<T>(method: &'static str) -> Result<T, IsmpError> {
    Err(IsmpError::Custom(format!(
        "{method}: outbound surface unsupported on solana host"
    )))
}

impl<'info> IsmpHost for SolanaHostFacade<'info> {
    fn host_state_machine(&self) -> StateMachine {
        self.host_state_machine
    }

    fn latest_commitment_height(&self, _id: StateMachineId) -> Result<u64, IsmpError> {
        Ok(0)
    }

    fn state_machine_commitment(
        &self,
        height: StateMachineHeight,
    ) -> Result<IsmpStateCommitment, IsmpError> {
        let key = (state_machine_to_u32(height.id), height.height);
        let snap = self
            .state_commitments
            .get(&key)
            .ok_or(IsmpError::StateCommitmentNotFound { height })?;
        if snap.vetoed {
            return Err(IsmpError::StateCommitmentNotFound { height });
        }
        Ok(IsmpStateCommitment {
            timestamp: snap.timestamp_secs,
            overlay_root: None,
            state_root: H256::from(snap.state_root),
        })
    }

    fn consensus_update_time(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Result<Duration, IsmpError> {
        let ts = self
            .consensus_last_updated
            .ok_or(IsmpError::ConsensusStateNotFound { consensus_state_id })?;
        Ok(Duration::from_secs(ts.max(0) as u64))
    }

    fn state_machine_update_time(
        &self,
        state_machine_height: StateMachineHeight,
    ) -> Result<Duration, IsmpError> {
        let key = (
            state_machine_to_u32(state_machine_height.id),
            state_machine_height.height,
        );
        let snap = self.state_commitments.get(&key).ok_or(
            IsmpError::StateCommitmentNotFound { height: state_machine_height },
        )?;
        Ok(Duration::from_secs(snap.updated_at.max(0) as u64))
    }

    fn consensus_client_id(
        &self,
        _consensus_state_id: ConsensusStateId,
    ) -> Option<ConsensusClientId> {
        Some(self.consensus_client_id)
    }

    fn consensus_state(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Result<Vec<u8>, IsmpError> {
        self.consensus_state_payload
            .clone()
            .ok_or(IsmpError::ConsensusStateNotFound { consensus_state_id })
    }

    fn timestamp(&self) -> Duration {
        Duration::from_secs(self.now_unix_secs.max(0) as u64)
    }

    fn is_consensus_client_frozen(
        &self,
        consensus_state_id: ConsensusStateId,
    ) -> Result<(), IsmpError> {
        if self.frozen {
            Err(IsmpError::FrozenConsensusClient { consensus_state_id })
        } else {
            Ok(())
        }
    }

    fn request_commitment(&self, _req: H256) -> Result<(), IsmpError> {
        outbound("request_commitment")
    }

    fn response_commitment(&self, _req: H256) -> Result<(), IsmpError> {
        outbound("response_commitment")
    }

    fn next_nonce(&self) -> u64 {
        0
    }

    fn request_receipt(&self, req: &Request) -> Option<()> {
        let commitment = hash_request::<Self>(req);
        match self.request_receipts.get(&commitment).copied() {
            Some(true) => Some(()),
            _ => None,
        }
    }

    fn response_receipt(&self, _res: &Response) -> Option<()> {
        None
    }

    fn store_consensus_state_id(
        &self,
        _consensus_state_id: ConsensusStateId,
        _client_id: ConsensusClientId,
    ) -> Result<(), IsmpError> {
        outbound("store_consensus_state_id")
    }

    fn store_consensus_state(
        &self,
        _consensus_state_id: ConsensusStateId,
        consensus_state: Vec<u8>,
    ) -> Result<(), IsmpError> {
        let consensus_state_acct = self
            .consensus_state_acct
            .clone()
            .ok_or_else(|| IsmpError::Custom("consensus_state_acct missing".to_string()))?;
        let bump = self.handler_authority_bump;
        let signer_seeds: &[&[&[u8]]] =
            &[&[b"handler_authority", core::slice::from_ref(&bump)]];
        let cpi_ctx = CpiContext::new_with_signer(
            self.host_program_id,
            host::cpi::accounts::StoreConsensusState {
                handler_authority: self.handler_authority.clone(),
                host_config: self.host_config.clone(),
                consensus_state: consensus_state_acct,
            },
            signer_seeds,
        );
        host::cpi::store_consensus_state(cpi_ctx, consensus_state)
            .map_err(|e| IsmpError::Custom(format!("cpi store_consensus_state: {e:?}")))
    }

    fn store_unbonding_period(
        &self,
        _consensus_state_id: ConsensusStateId,
        _period: u64,
    ) -> Result<(), IsmpError> {
        outbound("store_unbonding_period")
    }

    fn store_consensus_update_time(
        &self,
        _consensus_state_id: ConsensusStateId,
        _timestamp: Duration,
    ) -> Result<(), IsmpError> {
        // Host's `store_consensus_state` CPI stamps `last_updated` inline.
        Ok(())
    }

    fn store_state_machine_update_time(
        &self,
        _state_machine_height: StateMachineHeight,
        _timestamp: Duration,
    ) -> Result<(), IsmpError> {
        // Host's `store_state_commitment` CPI stamps `updated_at` inline.
        Ok(())
    }

    fn store_state_machine_commitment(
        &self,
        height: StateMachineHeight,
        state: IsmpStateCommitment,
    ) -> Result<(), IsmpError> {
        let key = (state_machine_to_u32(height.id), height.height);
        let acct = self
            .state_commitment_accts
            .get(&key)
            .cloned()
            .ok_or_else(|| {
                IsmpError::Custom(format!(
                    "state_commitment account missing for ({:?}, {})",
                    key.0, key.1
                ))
            })?;
        let bump = self.handler_authority_bump;
        let signer_seeds: &[&[&[u8]]] =
            &[&[b"handler_authority", core::slice::from_ref(&bump)]];
        let cpi_ctx = CpiContext::new_with_signer(
            self.host_program_id,
            host::cpi::accounts::StoreStateCommitment {
                handler_authority: self.handler_authority.clone(),
                rent_payer: self.relayer.clone(),
                host_config: self.host_config.clone(),
                state_commitment: acct,
                system_program: self.system_program.clone(),
            },
            signer_seeds,
        );
        host::cpi::store_state_commitment(
            cpi_ctx,
            host::StoreStateCommitmentParams {
                state_machine: key.0,
                height: key.1,
                state_root: state.state_root.0,
                timestamp: state.timestamp,
            },
        )
        .map_err(|e| IsmpError::Custom(format!("cpi store_state_commitment: {e:?}")))
    }

    fn delete_state_commitment(
        &self,
        _height: StateMachineHeight,
    ) -> Result<(), IsmpError> {
        // Vetoes go through the host's `veto_state_commitment` admin path.
        Err(IsmpError::Custom(
            "delete_state_commitment: handled out-of-band on solana".to_string(),
        ))
    }

    fn freeze_consensus_client(
        &self,
        _consensus_state_id: ConsensusStateId,
    ) -> Result<(), IsmpError> {
        Err(IsmpError::Custom(
            "freeze_consensus_client: handled out-of-band on solana".to_string(),
        ))
    }

    fn store_latest_commitment_height(
        &self,
        _height: StateMachineHeight,
    ) -> Result<(), IsmpError> {
        Ok(())
    }

    fn delete_request_commitment(&self, _req: &Request) -> Result<Vec<u8>, IsmpError> {
        outbound("delete_request_commitment")
    }

    fn delete_response_commitment(
        &self,
        _res: &PostResponse,
    ) -> Result<Vec<u8>, IsmpError> {
        outbound("delete_response_commitment")
    }

    fn delete_request_receipt(&self, _req: &Request) -> Result<Vec<u8>, IsmpError> {
        // CPI failure aborts the tx atomically — Anchor rolls back the
        // receipt write. No undo needed.
        Ok(vec![])
    }

    fn delete_response_receipt(&self, _res: &Response) -> Result<Vec<u8>, IsmpError> {
        outbound("delete_response_receipt")
    }

    fn store_request_receipt(
        &self,
        req: &Request,
        signer: &Vec<u8>,
    ) -> Result<Vec<u8>, IsmpError> {
        let commitment = hash_request::<Self>(req);
        let acct = self
            .request_receipt_accts
            .get(&commitment)
            .cloned()
            .ok_or_else(|| {
                IsmpError::Custom(format!(
                    "request_receipt account missing for commitment {:?}",
                    commitment
                ))
            })?;
        let fee_vault = self
            .fee_vault
            .clone()
            .ok_or_else(|| IsmpError::Custom("fee_vault account missing".to_string()))?;
        let dest_program = self
            .dest_program
            .clone()
            .ok_or_else(|| IsmpError::Custom("dest_program account missing".to_string()))?;
        let body = req.body().unwrap_or_default();
        let bump = self.handler_authority_bump;
        let signer_seeds: &[&[&[u8]]] =
            &[&[b"handler_authority", core::slice::from_ref(&bump)]];
        let cpi_ctx = CpiContext::new_with_signer(
            self.host_program_id,
            host::cpi::accounts::DispatchIncoming {
                handler_authority: self.handler_authority.clone(),
                relayer: self.relayer.clone(),
                host_config: self.host_config.clone(),
                request_receipt: acct,
                fee_vault,
                dest_program,
                system_program: self.system_program.clone(),
            },
            signer_seeds,
        );
        host::cpi::dispatch_incoming(
            cpi_ctx,
            host::DispatchIncomingParams {
                commitment: commitment.0,
                body,
            },
        )
        .map_err(|e| IsmpError::Custom(format!("cpi dispatch_incoming: {e:?}")))?;

        Ok(signer.clone())
    }

    fn store_response_receipt(
        &self,
        _req: &Response,
        _signer: &Vec<u8>,
    ) -> Result<Vec<u8>, IsmpError> {
        outbound("store_response_receipt")
    }

    fn store_request_commitment(
        &self,
        _req: &Request,
        _meta: Vec<u8>,
    ) -> Result<(), IsmpError> {
        outbound("store_request_commitment")
    }

    fn store_response_commitment(
        &self,
        _res: &PostResponse,
        _meta: Vec<u8>,
    ) -> Result<(), IsmpError> {
        outbound("store_response_commitment")
    }

    fn consensus_clients(&self) -> Vec<Box<dyn ConsensusClient>> {
        vec![Box::new(Sp1BeefyConsensusClient {
            sp1_vkey_hash: self.sp1_vkey_hash,
            consensus_client_id: self.consensus_client_id,
            commit_header_index: self.commit_header_index,
        })]
    }

    fn challenge_period(&self, _state_machine: StateMachineId) -> Option<Duration> {
        Some(Duration::from_secs(self.challenge_period_secs))
    }

    fn store_challenge_period(
        &self,
        _state_machine: StateMachineId,
        _period: u64,
    ) -> Result<(), IsmpError> {
        outbound("store_challenge_period")
    }

    fn allowed_proxy(&self) -> Option<StateMachine> {
        None
    }

    fn unbonding_period(&self, _consensus_state_id: ConsensusStateId) -> Option<Duration> {
        Some(Duration::from_secs(self.unbonding_period_secs))
    }

    fn ismp_router(&self) -> Box<dyn IsmpRouter> {
        Box::new(SolanaRouter)
    }

    fn previous_commitment_height(&self, _id: StateMachineId) -> Option<u64> {
        None
    }
}

pub fn state_machine_to_u32(id: StateMachineId) -> u32 {
    match id.state_id {
        StateMachine::Evm(n) => n,
        StateMachine::Polkadot(n) => n,
        StateMachine::Kusama(n) => n,
        StateMachine::Relay { para_id, .. } => para_id,
        StateMachine::Substrate(b) => u32::from_be_bytes(b),
        StateMachine::Tendermint(b) => u32::from_be_bytes(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(state: StateMachine) -> StateMachineId {
        StateMachineId { state_id: state, consensus_state_id: *b"BEFY" }
    }

    #[test]
    fn keccak256_matches_known_vector() {
        // keccak256("") — well-known Keccak-256 constant.
        let empty = SolanaHostFacade::keccak256(b"");
        let expected: [u8; 32] = [
            0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7,
            0x03, 0xc0, 0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04,
            0x5d, 0x85, 0xa4, 0x70,
        ];
        assert_eq!(empty.0, expected);
    }

    #[test]
    fn state_machine_u32_round_trips_polkadot_para_ids() {
        assert_eq!(state_machine_to_u32(id(StateMachine::Polkadot(2042))), 2042);
        assert_eq!(state_machine_to_u32(id(StateMachine::Kusama(2007))), 2007);
        assert_eq!(state_machine_to_u32(id(StateMachine::Evm(1))), 1);
        assert_eq!(
            state_machine_to_u32(id(StateMachine::Relay { relay: *b"hybr", para_id: 4009 })),
            4009
        );
    }

    #[test]
    fn state_machine_u32_packs_4byte_tags_big_endian() {
        // Substrate/Tendermint tags are 4-byte arrays we fold into u32.
        // Encoding stays consistent across calls so PDA seeds line up.
        let s = state_machine_to_u32(id(StateMachine::Substrate(*b"sola")));
        let t = state_machine_to_u32(id(StateMachine::Tendermint(*b"sola")));
        assert_eq!(s, t);
        assert_eq!(s, u32::from_be_bytes(*b"sola"));
    }

    #[test]
    fn outbound_helper_carries_method_name() {
        let err: Result<(), IsmpError> = outbound("frob");
        match err.unwrap_err() {
            IsmpError::Custom(msg) => assert!(msg.contains("frob"), "got: {msg}"),
            other => panic!("expected Custom, got {other:?}"),
        }
    }
}
