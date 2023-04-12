use alloc::{format, string::ToString};
use codec::{Decode, Encode};
use core::time::Duration;
use ethabi::ethereum_types::H256;

use crate::consensus_clients::{
    beacon_consensus_client::{
        presets::UNBONDING_PERIOD_HOURS,
        state_machine_ids::EXECUTION_LAYER_ID,
        types::{BeaconClientUpdate, BeaconMessage, ConsensusState},
        utils::{
            construct_intermediate_state, decode_evm_state_proof, get_contract_storage_root,
            get_value_from_proof, req_res_to_key,
        },
    },
    consensus_client_ids::ETHEREUM_CONSENSUS_CLIENT_ID,
};
use ismp_rs::{
    consensus_client::{ConsensusClient, IntermediateState, StateCommitment},
    error::Error,
    host::ISMPHost,
    messaging::Proof,
    router::RequestResponse,
};

use crate::consensus_clients::beacon_consensus_client::{
    optimism::verify_optimism_payload, presets::ismp_contract_address,
};
use sp_std::prelude::*;

#[derive(Default, Clone)]
pub struct BeaconConsensusClient;

impl ConsensusClient for BeaconConsensusClient {
    fn verify_consensus(
        &self,
        _host: &dyn ISMPHost,
        trusted_consensus_state: Vec<u8>,
        consensus_proof: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<IntermediateState>), Error> {
        let beacon_message = BeaconMessage::decode(&mut &consensus_proof[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode beacon message".to_string())
        })?;

        match beacon_message {
            BeaconMessage::ConsensusUpdate(BeaconClientUpdate {
                optimism_payload,
                consensus_update,
            }) => {
                let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
                    .map_err(|_| {
                        Error::ImplementationSpecific(
                            "Cannot decode trusted consensus state".to_string(),
                        )
                    })?;

                let no_codec_light_client_state =
                    consensus_state.light_client_state.try_into().map_err(|_| {
                        Error::ImplementationSpecific(format!(
                            "Cannot convert light client state to no codec type",
                        ))
                    })?;

                let no_codec_light_client_update =
                    consensus_update.clone().try_into().map_err(|_| {
                        Error::ImplementationSpecific(format!(
                            "Cannot convert light client update to no codec type"
                        ))
                    })?;

                let new_light_client_state =
                    sync_committee_verifier::verify_sync_committee_attestation(
                        no_codec_light_client_state,
                        no_codec_light_client_update,
                    )
                    .map_err(|_| Error::ConsensusProofVerificationFailed {
                        id: ETHEREUM_CONSENSUS_CLIENT_ID,
                    })?;

                let mut intermediate_states = vec![];

                let state_root = consensus_update.execution_payload.state_root;
                let intermediate_state = construct_intermediate_state(
                    EXECUTION_LAYER_ID,
                    ETHEREUM_CONSENSUS_CLIENT_ID,
                    consensus_update.execution_payload.block_number,
                    consensus_update.execution_payload.timestamp,
                    &state_root,
                )?;

                intermediate_states.push(intermediate_state);

                if let Some(optimism_payload) = optimism_payload {
                    let state = verify_optimism_payload(optimism_payload, &state_root)?;
                    intermediate_states.push(state)
                }

                let new_consensus_state = ConsensusState {
                    frozen_height: None,
                    light_client_state: new_light_client_state.try_into().map_err(|_| {
                        Error::ImplementationSpecific(format!(
                            "Cannot convert light client state to codec type"
                        ))
                    })?,
                };

                Ok((new_consensus_state.encode(), intermediate_states))
            }
            _ => unimplemented!(),
        }
    }

    fn unbonding_period(&self) -> Duration {
        Duration::from_secs(UNBONDING_PERIOD_HOURS * 60 * 60)
    }

    fn verify_membership(
        &self,
        host: &dyn ISMPHost,
        item: RequestResponse,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<(), Error> {
        let evm_state_proof = decode_evm_state_proof(proof)?;
        let contract_address = ismp_contract_address(&item).ok_or_else(|| {
            Error::ImplementationSpecific("Ismp contract address not found".to_string())
        })?;
        let key = req_res_to_key(host, item);
        let root = H256::from_slice(&root.state_root[..]);
        let contract_root = get_contract_storage_root(
            evm_state_proof.contract_proof,
            &contract_address,
            root.clone(),
        )?;
        let _ = get_value_from_proof(key, contract_root, evm_state_proof.storage_proof)?
            .ok_or_else(|| {
                Error::MembershipProofVerificationFailed(format!("There is no DB value"))
            })?;

        Ok(())
    }

    fn verify_state_proof(
        &self,
        _host: &dyn ISMPHost,
        _key: Vec<u8>,
        _root: StateCommitment,
        _proof: &Proof,
    ) -> Result<Vec<u8>, Error> {
        unimplemented!()
    }

    fn verify_non_membership(
        &self,
        host: &dyn ISMPHost,
        item: RequestResponse,
        root: StateCommitment,
        proof: &Proof,
    ) -> Result<(), Error> {
        let evm_state_proof = decode_evm_state_proof(proof)?;
        let contract_address = ismp_contract_address(&item).ok_or_else(|| {
            Error::ImplementationSpecific("Ismp contract address not found".to_string())
        })?;
        let key = req_res_to_key(host, item);
        let root = H256::from_slice(&root.state_root[..]);
        let contract_root =
            get_contract_storage_root(evm_state_proof.contract_proof, &contract_address, root)?;

        let result = get_value_from_proof(key, contract_root, evm_state_proof.storage_proof)?;

        if result.is_some() {
            return Err(Error::NonMembershipProofVerificationFailed(
                "Invalid membership proof".to_string(),
            ))
        }

        Ok(())
    }

    fn is_frozen(&self, consensus_state: &[u8]) -> Result<(), Error> {
        let consensus_state = ConsensusState::decode(&mut &consensus_state[..]).map_err(|_| {
            Error::ImplementationSpecific("Cannot decode trusted consensus state".to_string())
        })?;
        if consensus_state.frozen_height.is_some() {
            Err(Error::FrozenConsensusClient { id: ETHEREUM_CONSENSUS_CLIENT_ID })
        } else {
            Ok(())
        }
    }
}
