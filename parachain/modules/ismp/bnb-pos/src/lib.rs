#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
extern crate alloc;

use core::marker::PhantomData;

use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec, vec::Vec};
use bnb_pos_verifier::{
    primitives::BnbClientUpdate, verify_bnb_header, NextValidators, VerificationResult,
};
use codec::{Decode, Encode};
use ismp::{
    consensus::{ConsensusClient, ConsensusStateId, StateCommitment, StateMachineClient},
    error::Error,
    host::{IsmpHost, StateMachine},
    messaging::StateCommitmentHeight,
};
use ismp_sync_committee::EvmStateMachine;
use sp_core::H256;
use sync_committee_primitives::constants::BlsPublicKey;

pub const BNB_CONSENSUS_ID: ConsensusStateId = *b"BNBP";

#[derive(codec::Encode, codec::Decode, Debug)]
pub struct ConsensusState {
    pub current_validators: Vec<BlsPublicKey>,
    pub next_validators: Option<NextValidators>,
    pub finalized_height: u64,
    pub finalized_hash: H256,
}

pub struct BnbClient<H: IsmpHost>(PhantomData<H>);

impl<H: IsmpHost> Default for BnbClient<H> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<H: IsmpHost> Clone for BnbClient<H> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<H: IsmpHost + Send + Sync + Default + 'static> ConsensusClient for BnbClient<H> {
    fn verify_consensus(
        &self,
        _host: &dyn IsmpHost,
        _consensus_state_id: ismp::consensus::ConsensusStateId,
        trusted_consensus_state: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(Vec<u8>, ismp::consensus::VerifiedCommitments), ismp::error::Error> {
        let bnb_client_update = BnbClientUpdate::decode(&mut &proof[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode bnb client update".to_string())
        })?;

        let mut consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
            .map_err(|_| {
                Error::ImplementationSpecific("Cannot decode trusted consensus state".to_string())
            })?;

        if let Some(next_validators) = consensus_state.next_validators {
            let next_rotation_block_number = next_validators.rotation_block;

            if bnb_client_update.attested_header.number.low_u64() >= next_rotation_block_number {
                consensus_state.current_validators = next_validators.validators;
                consensus_state.next_validators = None;
            }
        }

        let VerificationResult { hash, finalized_header, next_validators } =
            verify_bnb_header::<H>(&consensus_state.current_validators, bnb_client_update)
                .map_err(|e| Error::ImplementationSpecific(e.to_string()))?;

        let mut state_machine_map: BTreeMap<StateMachine, Vec<StateCommitmentHeight>> =
            BTreeMap::new();

        let state_commitment = StateCommitmentHeight {
            commitment: StateCommitment {
                timestamp: finalized_header.timestamp,
                overlay_root: None,
                state_root: finalized_header.state_root,
            },
            height: finalized_header.number.low_u64(),
        };
        consensus_state.finalized_hash = hash;
        consensus_state.next_validators = next_validators;

        state_machine_map.insert(StateMachine::Bnb, vec![state_commitment]);

        Ok((consensus_state.encode(), state_machine_map))
    }

    fn verify_fraud_proof(
        &self,
        _host: &dyn IsmpHost,
        trusted_consensus_state: Vec<u8>,
        proof_1: Vec<u8>,
        proof_2: Vec<u8>,
    ) -> Result<(), ismp::error::Error> {
        let bnb_client_update_1 = BnbClientUpdate::decode(&mut &proof_1[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode bnb client update for proof 1".to_string())
        })?;

        let bnb_client_update_2 = BnbClientUpdate::decode(&mut &proof_2[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode bnb client update for proof 2".to_string())
        })?;

        let header_1 = bnb_client_update_1.attested_header.clone();
        let header_2 = bnb_client_update_2.attested_header.clone();

        if header_1.number != header_2.number {
            Err(Error::ImplementationSpecific("Invalid Fraud proof".to_string()))?
        }

        let consensus_state =
            ConsensusState::decode(&mut &trusted_consensus_state[..]).map_err(|_| {
                Error::ImplementationSpecific("Cannot decode trusted consensus state".to_string())
            })?;

        let res_1 =
            verify_bnb_header::<H>(&consensus_state.current_validators, bnb_client_update_1)
                .map_err(|_| {
                    Error::ImplementationSpecific("Failed to verify first header".to_string())
                })?;

        let res_2 =
            verify_bnb_header::<H>(&consensus_state.current_validators, bnb_client_update_2)
                .map_err(|_| {
                    Error::ImplementationSpecific("Failed to verify second header".to_string())
                })?;

        if res_1.hash != res_2.hash {
            return Ok(())
        }
        Ok(())
    }

    fn state_machine(
        &self,
        _id: ismp::host::StateMachine,
    ) -> Result<Box<dyn StateMachineClient>, ismp::error::Error> {
        Ok(Box::new(<EvmStateMachine<H>>::default()))
    }
}
