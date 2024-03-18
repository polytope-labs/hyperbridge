#![cfg_attr(not(feature = "std"), no_std)]
#[warn(unused_imports)]
#[warn(unused_variables)]
extern crate alloc;

use core::marker::PhantomData;

use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec, vec::Vec};
use bsc_pos_verifier::{
    primitives::{compute_epoch, BscClientUpdate, EPOCH_LENGTH},
    verify_bsc_header, NextValidators, VerificationResult,
};
use codec::{Decode, Encode};
use geth_primitives::Header;
use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
    },
    error::Error,
    host::{IsmpHost, StateMachine},
    messaging::{Proof, StateCommitmentHeight},
    router::RequestResponse,
};
use ismp_sync_committee::{utils::req_res_receipt_keys, verify_membership, verify_state_proof};
use sp_core::{H160, H256};
use sync_committee_primitives::constants::BlsPublicKey;

pub const BSC_CONSENSUS_ID: ConsensusStateId = *b"BSCP";

#[derive(codec::Encode, codec::Decode, Debug, Default, PartialEq, Eq, Clone)]
pub struct ConsensusState {
    pub current_validators: Vec<BlsPublicKey>,
    pub next_validators: Option<NextValidators>,
    pub finalized_height: u64,
    pub finalized_hash: H256,
    pub current_epoch: u64,
    pub ismp_contract_address: H160,
}

pub struct BscClient<H: IsmpHost>(PhantomData<H>);

impl<H: IsmpHost> Default for BscClient<H> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<H: IsmpHost> Clone for BscClient<H> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<H: IsmpHost + Send + Sync + Default + 'static> ConsensusClient for BscClient<H> {
    fn verify_consensus(
        &self,
        _host: &dyn IsmpHost,
        _consensus_state_id: ConsensusStateId,
        trusted_consensus_state: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(Vec<u8>, ismp::consensus::VerifiedCommitments), ismp::error::Error> {
        let bsc_client_update = BscClientUpdate::decode(&mut &proof[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode bsc client update".to_string())
        })?;

        let mut consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
            .map_err(|_| {
                Error::ImplementationSpecific("Cannot decode trusted consensus state".to_string())
            })?;

        if consensus_state.finalized_height >= bsc_client_update.source_header.number.low_u64() {
            Err(Error::ImplementationSpecific("Expired Update".to_string()))?
        }

        if let Some(next_validators) = consensus_state.next_validators.clone() {
            if bsc_client_update.attested_header.number.low_u64() % EPOCH_LENGTH >=
                (consensus_state.current_validators.len() as u64 / 2)
            {
                // Sanity check
                // During authority set rotation, the source header must be from the same epoch as
                // the attested header
                let epoch = compute_epoch(bsc_client_update.attested_header.number.low_u64());
                let source_header_epoch =
                    compute_epoch(bsc_client_update.source_header.number.low_u64());
                if source_header_epoch != epoch {
                    Err(Error::ImplementationSpecific("The Source Header must be from the same epoch with the attested epoch during an authority set rotation".to_string()))?
                }
                consensus_state.current_validators = next_validators.validators;
                consensus_state.next_validators = None;
                consensus_state.current_epoch = epoch;
            }
        }

        let VerificationResult { hash, finalized_header, next_validators } =
            verify_bsc_header::<H>(&consensus_state.current_validators, bsc_client_update)
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

        if let Some(next_validators) = next_validators {
            consensus_state.next_validators = Some(next_validators);
        }
        consensus_state.finalized_height = finalized_header.number.low_u64();
        state_machine_map.insert(StateMachine::Bsc, vec![state_commitment]);

        Ok((consensus_state.encode(), state_machine_map))
    }

    fn verify_fraud_proof(
        &self,
        _host: &dyn IsmpHost,
        trusted_consensus_state: Vec<u8>,
        proof_1: Vec<u8>,
        proof_2: Vec<u8>,
    ) -> Result<(), ismp::error::Error> {
        let bsc_client_update_1 = BscClientUpdate::decode(&mut &proof_1[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode bsc client update for proof 1".to_string())
        })?;

        let bsc_client_update_2 = BscClientUpdate::decode(&mut &proof_2[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode bsc client update for proof 2".to_string())
        })?;

        let header_1 = bsc_client_update_1.attested_header.clone();
        let header_2 = bsc_client_update_2.attested_header.clone();

        if header_1.number != header_2.number {
            Err(Error::ImplementationSpecific("Invalid Fraud proof".to_string()))?
        }

        let header_1_hash = Header::from(&header_1).hash::<H>();
        let header_2_hash = Header::from(&header_2).hash::<H>();

        if header_1_hash == header_2_hash {
            return Err(Error::ImplementationSpecific("Invalid Fraud proof".to_string()))
        }

        let consensus_state =
            ConsensusState::decode(&mut &trusted_consensus_state[..]).map_err(|_| {
                Error::ImplementationSpecific("Cannot decode trusted consensus state".to_string())
            })?;

        let _ = verify_bsc_header::<H>(&consensus_state.current_validators, bsc_client_update_1)
            .map_err(|_| {
                Error::ImplementationSpecific("Failed to verify first header".to_string())
            })?;

        let _ = verify_bsc_header::<H>(&consensus_state.current_validators, bsc_client_update_2)
            .map_err(|_| {
                Error::ImplementationSpecific("Failed to verify second header".to_string())
            })?;

        Ok(())
    }

    fn consensus_client_id(&self) -> ConsensusClientId {
        BSC_CONSENSUS_ID
    }

    fn state_machine(
        &self,
        id: ismp::host::StateMachine,
    ) -> Result<Box<dyn StateMachineClient>, ismp::error::Error> {
        match id {
            StateMachine::Bsc => Ok(Box::new(<EvmStateMachine<H>>::default())),
            state_machine =>
                return Err(Error::ImplementationSpecific(alloc::format!(
                    "Unsupported state machine: {state_machine:?}"
                ))),
        }
    }
}

#[derive(Default, Clone)]
pub struct EvmStateMachine<H: IsmpHost>(core::marker::PhantomData<H>);

impl<H: IsmpHost + Send + Sync> StateMachineClient for EvmStateMachine<H> {
    fn verify_membership(
        &self,
        host: &dyn IsmpHost,
        item: RequestResponse,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<(), Error> {
        let consensus_state = host.consensus_state(proof.height.id.consensus_state_id)?;
        let consensus_state = ConsensusState::decode(&mut &consensus_state[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode consensus state".to_string())
        })?;

        verify_membership::<H>(item, root, proof, consensus_state.ismp_contract_address)
    }

    fn state_trie_key(&self, items: RequestResponse) -> Vec<Vec<u8>> {
        req_res_receipt_keys::<H>(items)
    }

    fn verify_state_proof(
        &self,
        host: &dyn IsmpHost,
        keys: Vec<Vec<u8>>,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
        let consensus_state = host.consensus_state(proof.height.id.consensus_state_id)?;
        let consensus_state = ConsensusState::decode(&mut &consensus_state[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode consensus state".to_string())
        })?;

        verify_state_proof::<H>(keys, root, proof, consensus_state.ismp_contract_address)
    }
}
