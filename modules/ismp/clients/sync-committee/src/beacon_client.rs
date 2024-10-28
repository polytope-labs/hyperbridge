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
use evm_common::construct_intermediate_state;

use crate::{
	pallet::{self, SupportedStatemachines},
	types::{BeaconClientUpdate, ConsensusState, L2Consensus},
};
use evm_common::EvmStateMachine;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateMachineClient,
		VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};
use op_verifier::{verify_optimism_dispute_game_proof, verify_optimism_payload};
use sync_committee_primitives::constants::Config;

use crate::prelude::*;

pub use sync_committee_primitives::constants::{BEACON_CONSENSUS_ID, GNOSIS_CONSENSUS_ID};

pub struct SyncCommitteeConsensusClient<
	H: IsmpHost,
	C: Config,
	T: pallet_ismp_host_executive::Config + crate::pallet::Config<I>,
	I: 'static,
>(core::marker::PhantomData<(H, C, T, I)>);

impl<
		H: IsmpHost + Send + Sync + Default + 'static,
		C: Config + Send + Sync + Default + 'static,
		I: 'static,
		T: pallet_ismp_host_executive::Config + pallet::Config<I> + 'static,
	> Default for SyncCommitteeConsensusClient<H, C, T, I>
{
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<
		H: IsmpHost + Send + Sync + Default + 'static,
		C: Config + Send + Sync + Default + 'static,
		I: 'static,
		T: pallet_ismp_host_executive::Config + pallet::Config<I> + 'static,
	> Clone for SyncCommitteeConsensusClient<H, C, T, I>
{
	fn clone(&self) -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<
		H: IsmpHost + Send + Sync + Default + 'static,
		C: Config + Send + Sync + Default + 'static,
		I: 'static,
		T: pallet_ismp_host_executive::Config + pallet::Config<I> + 'static,
	> ConsensusClient for SyncCommitteeConsensusClient<H, C, T, I>
{
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		consensus_proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		let BeaconClientUpdate {
			mut l2_oracle_payload,
			mut dispute_game_payload,
			consensus_update,
			mut arbitrum_payload,
		} = BeaconClientUpdate::decode(&mut &consensus_proof[..])
			.map_err(|_| Error::Custom("Cannot decode beacon client update".to_string()))?;

		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::Custom("Cannot decode trusted consensus state".to_string()))?;

		let new_light_client_state =
			sync_committee_verifier::verify_sync_committee_attestation::<C>(
				consensus_state.light_client_state,
				consensus_update.clone(),
			)
			.map_err(|e| Error::Custom(format!("{:?}", e)))?;

		let mut state_machine_map: BTreeMap<StateMachine, Vec<StateCommitmentHeight>> =
			BTreeMap::new();

		let state_root = consensus_update.execution_payload.state_root;
		let intermediate_state = construct_intermediate_state(
			StateMachine::Evm(consensus_state.chain_id),
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

		state_machine_map.insert(StateMachine::Evm(consensus_state.chain_id), state_commitment_vec);

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
					if let Some(payload) = l2_oracle_payload.remove(&state_machine) {
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
				L2Consensus::OpFaultProofs((dispute_game_factory, respected_game_type)) =>
					if let Some(payload) = dispute_game_payload.remove(&state_machine) {
						let state = verify_optimism_dispute_game_proof::<H>(
							payload,
							state_root,
							dispute_game_factory,
							respected_game_type,
							consensus_state_id.clone(),
						)?;

						let state_commitment_height = StateCommitmentHeight {
							commitment: state.commitment,
							height: state.height.height,
						};

						let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
						state_commitment_vec.push(state_commitment_height);
						state_machine_map.insert(state_machine, state_commitment_vec);
					},
			}
		}

		let new_consensus_state = ConsensusState {
			frozen_height: None,
			light_client_state: new_light_client_state.try_into().map_err(|_| {
				Error::Custom(format!("Cannot convert light client state to codec type"))
			})?,
			l2_consensus: consensus_state.l2_consensus,
			chain_id: consensus_state.chain_id,
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
		Err(Error::Custom("fraud proof verification unimplemented".to_string()))
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		C::ID
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		if SupportedStatemachines::<T, I>::contains_key(id) {
			Ok(Box::new(<EvmStateMachine<H, T>>::default()))
		} else {
			Err(Error::Custom("State machine not supported".to_string()))
		}
	}
}
