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
use arbitrum_verifier::verify_arbitrum_payload;
use codec::{Decode, Encode};
use evm_common::{
    construct_intermediate_state, req_res_receipt_keys, verify_membership, verify_state_proof,
};

use crate::types::{BeaconClientUpdate, ConsensusState, L2Consensus};
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
use op_verifier::{verify_optimism_dispute_game_proof, verify_optimism_payload};
use sync_committee_primitives::constants::Config;

use crate::prelude::*;

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
        let BeaconClientUpdate {
            l2_oracle_payload: mut op_stack_payload,
            mut dispute_game_payload,
            consensus_update,
            mut arbitrum_payload,
        } = BeaconClientUpdate::decode(&mut &consensus_proof[..]).map_err(|_| {
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

        let l2_consensus = consensus_state.l2_consensus.clone();

        for (state_machine, consensus_mechanic) in l2_consensus {
            match consensus_mechanic {
                L2Consensus::ArbitrumOrbit(rollup_core_address) => {
                    if let Some(arbitrum_payload) = arbitrum_payload.remove(&state_machine) {
                        let state = verify_arbitrum_payload::<H>(
                            arbitrum_payload,
                            state_root,
                            rollup_core_address,
                            consensus_state_id.clone(),
                        )?;

                        let arbitrum_state_commitment_height = StateCommitmentHeight {
                            commitment: state.commitment,
                            height: state.height.height,
                        };

                        let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
                        state_commitment_vec.push(arbitrum_state_commitment_height);
                        state_machine_map.insert(state_machine, state_commitment_vec);
                    }
                },
                L2Consensus::OpL2Oracle(l2_oracle) => {
                    if let Some(payload) = op_stack_payload.remove(&state_machine) {
                        let state = verify_optimism_payload::<H>(
                            payload,
                            state_root,
                            l2_oracle,
                            consensus_state_id.clone(),
                        )?;

                        let state_commitment_height = StateCommitmentHeight {
                            commitment: state.commitment,
                            height: state.height.height,
                        };

                        let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
                        state_commitment_vec.push(state_commitment_height);
                        state_machine_map.insert(state_machine, state_commitment_vec);
                    }
                },
                L2Consensus::OpFaultProofs(dispute_game_factory) => {
                    if let Some(payload) = dispute_game_payload.remove(&state_machine) {
                        let state = verify_optimism_dispute_game_proof::<H>(
                            payload,
                            state_root,
                            dispute_game_factory,
                            consensus_state_id.clone(),
                        )?;

                        let state_commitment_height = StateCommitmentHeight {
                            commitment: state.commitment,
                            height: state.height.height,
                        };

                        let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
                        state_commitment_vec.push(state_commitment_height);
                        state_machine_map.insert(state_machine, state_commitment_vec);
                    }
                },
            }
        }

        let new_consensus_state = ConsensusState {
            frozen_height: None,
            light_client_state: new_light_client_state.try_into().map_err(|_| {
                Error::ImplementationSpecific(format!(
                    "Cannot convert light client state to codec type"
                ))
            })?,
            ismp_contract_addresses: consensus_state.ismp_contract_addresses,
            l2_consensus: consensus_state.l2_consensus,
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
            _ => Err(Error::ImplementationSpecific("State machine not supported".to_string())),
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
