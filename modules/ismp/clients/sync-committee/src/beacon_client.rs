// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use alloc::{collections::BTreeMap, format, string::ToString};
use codec::{Decode, Encode};
use ethabi::ethereum_types::{H160, H256};

use crate::{
    types::{BeaconClientUpdate, ConsensusState},
    utils::{
        construct_intermediate_state, decode_evm_state_proof, get_contract_storage_root,
        req_res_to_key,
    },
};
use ismp::{
    consensus::{
        ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
        VerifiedCommitments,
    },
    error::Error,
    host::{Ethereum, IsmpHost, StateMachine},
    messaging::{Proof, StateCommitmentHeight},
    router::RequestResponse,
};
use sync_committee_primitives::constants::Config;

use crate::{
    arbitrum::verify_arbitrum_payload,
    optimism::verify_optimism_payload,
    prelude::*,
    utils::{get_value_from_proof, get_values_from_proof, req_res_receipt_keys},
};

pub const BEACON_CONSENSUS_ID: ConsensusClientId = *b"BEAC";

#[derive(Default, Clone)]
pub struct SyncCommitteeConsensusClient<H: IsmpHost, C: Config>(core::marker::PhantomData<(H, C)>);

impl<
        H: IsmpHost + Send + Sync + Default + 'static,
        C: Config + Send + Sync + Default + 'static,
    > ConsensusClient for SyncCommitteeConsensusClient<H, C>
{
    fn verify_consensus(
        &self,
        _host: &dyn IsmpHost,
        consensus_state_id: ConsensusStateId,
        trusted_consensus_state: Vec<u8>,
        consensus_proof: Vec<u8>,
    ) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
        let BeaconClientUpdate { mut op_stack_payload, consensus_update, arbitrum_payload } =
            BeaconClientUpdate::decode(&mut &consensus_proof[..]).map_err(|_| {
                Error::ImplementationSpecific("Cannot decode beacon client update".to_string())
            })?;

        let consensus_state =
            ConsensusState::decode(&mut &trusted_consensus_state[..]).map_err(|_| {
                Error::ImplementationSpecific("Cannot decode trusted consensus state".to_string())
            })?;

        let new_light_client_state =
            sync_committee_verifier::verify_sync_committee_attestation::<C>(
                consensus_state.light_client_state,
                consensus_update.clone(),
            )
            .map_err(|e| Error::ImplementationSpecific(format!("{:?}", e)))?;

        let mut state_machine_map: BTreeMap<StateMachine, Vec<StateCommitmentHeight>> =
            BTreeMap::new();

        let state_root = consensus_update.execution_payload.state_root;
        let intermediate_state = construct_intermediate_state(
            StateMachine::Ethereum(Ethereum::ExecutionLayer),
            consensus_state_id.clone(),
            consensus_update.execution_payload.block_number,
            consensus_update.execution_payload.timestamp,
            &state_root[..],
        )?;

        let ethereum_state_commitment_height = StateCommitmentHeight {
            commitment: intermediate_state.commitment,
            height: intermediate_state.height.height,
        };

        let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
        state_commitment_vec.push(ethereum_state_commitment_height);

        state_machine_map
            .insert(StateMachine::Ethereum(Ethereum::ExecutionLayer), state_commitment_vec);

        let op_stack =
            [StateMachine::Ethereum(Ethereum::Base), StateMachine::Ethereum(Ethereum::Optimism)];

        for state_machine_id in op_stack {
            if let Some(payload) = op_stack_payload.remove(&state_machine_id) {
                let state = verify_optimism_payload::<H>(
                    payload,
                    &state_root[..],
                    *consensus_state.l2_oracle_address.get(&state_machine_id).ok_or_else(|| {
                        Error::ImplementationSpecific("l2 oracle address was not set".into())
                    })?,
                    consensus_state_id.clone(),
                )?;

                let state_commitment_height = StateCommitmentHeight {
                    commitment: state.commitment,
                    height: state.height.height,
                };

                let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
                state_commitment_vec.push(state_commitment_height);
                state_machine_map.insert(state_machine_id, state_commitment_vec);
            }
        }

        if let Some(arbitrum_payload) = arbitrum_payload {
            let state = verify_arbitrum_payload::<H>(
                arbitrum_payload,
                &state_root[..],
                consensus_state.rollup_core_address,
                consensus_state_id.clone(),
            )?;

            let arbitrum_state_commitment_height =
                StateCommitmentHeight { commitment: state.commitment, height: state.height.height };

            let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
            state_commitment_vec.push(arbitrum_state_commitment_height);
            state_machine_map
                .insert(StateMachine::Ethereum(Ethereum::Arbitrum), state_commitment_vec);
        }

        let new_consensus_state = ConsensusState {
            frozen_height: None,
            light_client_state: new_light_client_state.try_into().map_err(|_| {
                Error::ImplementationSpecific(format!(
                    "Cannot convert light client state to codec type"
                ))
            })?,
            ismp_contract_addresses: consensus_state.ismp_contract_addresses,
            l2_oracle_address: consensus_state.l2_oracle_address,
            rollup_core_address: consensus_state.rollup_core_address,
        };

        Ok((new_consensus_state.encode(), state_machine_map))
    }

    fn verify_fraud_proof(
        &self,
        _host: &dyn IsmpHost,
        _trusted_consensus_state: Vec<u8>,
        _proof_1: Vec<u8>,
        _proof_2: Vec<u8>,
    ) -> Result<(), Error> {
        Err(Error::ImplementationSpecific("fraud proof verification unimplemented".to_string()))
    }

    fn consensus_client_id(&self) -> ConsensusClientId {
        BEACON_CONSENSUS_ID
    }

    fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
        match id {
            StateMachine::Ethereum(_) => Ok(Box::new(<EvmStateMachine<H>>::default())),
            _ =>
                return Err(Error::ImplementationSpecific("State machine not supported".to_string())),
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

        let contract_address = consensus_state
            .ismp_contract_addresses
            .get(&proof.height.id.state_id)
            .cloned()
            .ok_or_else(|| {
                Error::ImplementationSpecific("Ismp contract address not found".to_string())
            })?;
        verify_membership::<H>(item, root, proof, contract_address)
    }

    fn state_trie_key(&self, items: RequestResponse) -> Vec<Vec<u8>> {
        // State trie keys are used to process timeouts from EVM chains
        // We return the trie keys for request or response receipts
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
        let ismp_address = consensus_state
            .ismp_contract_addresses
            .get(&proof.height.id.state_id)
            .cloned()
            .ok_or_else(|| {
                Error::ImplementationSpecific("Ismp contract address not found".to_string())
            })?;

        verify_state_proof::<H>(keys, root, proof, ismp_address)
    }
}

pub fn verify_membership<H: IsmpHost + Send + Sync>(
    item: RequestResponse,
    root: StateCommitment,
    proof: &Proof,
    contract_address: H160,
) -> Result<(), Error> {
    let evm_state_proof = decode_evm_state_proof(proof)?;

    let keys = req_res_to_key::<H>(item);
    let root = H256::from_slice(&root.state_root[..]);
    let contract_root = get_contract_storage_root::<H>(
        evm_state_proof.contract_proof,
        contract_address,
        root.clone(),
    )?;
    let values = get_values_from_proof::<H>(keys, contract_root, evm_state_proof.storage_proof)?;

    if values.into_iter().any(|val| val.is_none()) {
        Err(Error::ImplementationSpecific("Missing values for some keys in the proof".to_string()))?
    }

    Ok(())
}

pub fn verify_state_proof<H: IsmpHost + Send + Sync>(
    keys: Vec<Vec<u8>>,
    root: StateCommitment,
    proof: &Proof,
    ismp_address: H160,
) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
    let mut evm_state_proof = decode_evm_state_proof(proof)?;
    let mut map = BTreeMap::new();
    for key in keys {
        // For keys less than 52 bytes we default to the ismp contract address as the contract
        // key
        let contract_address =
            if key.len() == 52 { H160::from_slice(&key[..20]) } else { ismp_address };
        let slot_hash = if key.len() == 52 {
            H::keccak256(&key[20..]).0.to_vec()
        } else {
            H::keccak256(&key).0.to_vec()
        };

        let contract_root = get_contract_storage_root::<H>(
            evm_state_proof.contract_proof.clone(),
            contract_address,
            root.state_root,
        )?;

        let storage_proof = evm_state_proof
            .storage_proof
            .remove(&key)
            .ok_or_else(|| Error::ImplementationSpecific("Missing proof for key".to_string()))?;

        let value = get_value_from_proof::<H>(slot_hash, contract_root, storage_proof)?;
        map.insert(key, value);
    }

    Ok(map)
}
