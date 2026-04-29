//! `ConsensusClient` impl wrapping the SP1 BEEFY verifier.

extern crate alloc;
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
        StateMachineId, VerifiedCommitments,
    },
    error::Error as IsmpError,
    host::{IsmpHost, StateMachine},
    messaging::StateCommitmentHeight,
};
use parity_scale_codec::{Decode, Encode};
use primitive_types::H256;

use sp1_beefy_verifier::{
    ConsensusState as BeefyConsensusState, Sp1BeefyProof, PROOF_TYPE_SP1,
};

use crate::verifier::beefy::{
    extract_header_prefix, maybe_rotate_authorities, verify_and_extract_update,
};

use super::state_machine_client::SubstrateStateMachineClient;

pub struct Sp1BeefyConsensusClient {
    pub sp1_vkey_hash: [u8; 32],
    pub consensus_client_id: ConsensusClientId,
    /// `Some(idx)` filters `verify_consensus` to one parachain header,
    /// preserving the one-StateCommitment-PDA-per-tx model.
    pub commit_header_index: Option<usize>,
}

impl ConsensusClient for Sp1BeefyConsensusClient {
    fn verify_consensus(
        &self,
        _host: &dyn IsmpHost,
        consensus_state_id: ConsensusStateId,
        trusted_consensus_state: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(Vec<u8>, VerifiedCommitments), IsmpError> {
        if proof.is_empty() || proof[0] != PROOF_TYPE_SP1 {
            return Err(IsmpError::Custom(
                "wrong proof-type prefix; expected PROOF_TYPE_SP1".to_string(),
            ));
        }
        let mut input = &proof[1..];
        let beefy_proof = Sp1BeefyProof::decode(&mut input)
            .map_err(|e| IsmpError::Custom(format!("decode Sp1BeefyProof: {e:?}")))?;

        let mut state_bytes = trusted_consensus_state.as_slice();
        let mut beefy_state = BeefyConsensusState::decode(&mut state_bytes)
            .map_err(|e| IsmpError::Custom(format!("decode trusted ConsensusState: {e:?}")))?;

        let update = verify_and_extract_update(&beefy_state, &beefy_proof, &self.sp1_vkey_hash)
            .map_err(|e| {
                IsmpError::ConsensusProofVerificationFailed { id: self.consensus_client_id }
                    .into_anyhow_or_self(format!("{e:?}"))
            })?;

        maybe_rotate_authorities(&mut beefy_state, &beefy_proof);
        beefy_state.latest_beefy_height = update.new_height;
        let new_state_bytes = beefy_state.encode();

        let mut commitments: VerifiedCommitments = BTreeMap::new();
        let header_iter: Box<dyn Iterator<Item = (usize, _)>> = match self.commit_header_index {
            Some(idx) => {
                let header = beefy_proof.headers.get(idx).ok_or_else(|| {
                    IsmpError::Custom(format!(
                        "commit_header_index {idx} out of range ({} headers)",
                        beefy_proof.headers.len()
                    ))
                })?;
                Box::new(core::iter::once((idx, header)))
            },
            None => Box::new(beefy_proof.headers.iter().enumerate()),
        };

        for (_idx, header) in header_iter {
            let (number, state_root) = extract_header_prefix(&header.header)
                .map_err(|e| IsmpError::Custom(format!("extract_header_prefix: {e:?}")))?;
            let id = StateMachineId {
                state_id: StateMachine::Polkadot(header.para_id),
                consensus_state_id,
            };
            let entry = commitments.entry(id).or_default();
            entry.push(StateCommitmentHeight {
                commitment: StateCommitment {
                    timestamp: 0,
                    overlay_root: None,
                    state_root: H256::from(state_root),
                },
                height: number as u64,
            });
        }

        Ok((new_state_bytes, commitments))
    }

    fn verify_fraud_proof(
        &self,
        _host: &dyn IsmpHost,
        _trusted_consensus_state: Vec<u8>,
        _proof_1: Vec<u8>,
        _proof_2: Vec<u8>,
    ) -> Result<(), IsmpError> {
        Err(IsmpError::Custom(
            "fraud_proof: unsupported on solana inbound-only host".to_string(),
        ))
    }

    fn consensus_client_id(&self) -> ConsensusClientId {
        self.consensus_client_id
    }

    fn state_machine(
        &self,
        _id: StateMachine,
    ) -> Result<Box<dyn StateMachineClient>, IsmpError> {
        Ok(Box::new(SubstrateStateMachineClient))
    }
}

trait IntoIsmpVerifyError {
    fn into_anyhow_or_self(self, ctx: String) -> IsmpError;
}

impl IntoIsmpVerifyError for IsmpError {
    fn into_anyhow_or_self(self, ctx: String) -> IsmpError {
        match self {
            IsmpError::ConsensusProofVerificationFailed { id } => IsmpError::Custom(format!(
                "ConsensusProofVerificationFailed(id={id:?}): {ctx}"
            )),
            other => other,
        }
    }
}
