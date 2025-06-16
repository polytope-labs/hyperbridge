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

//! ISMP Consensus Client for the Optimism Consensus Protocol.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

pub mod pallet;
use pallet::Pallet;
use pallet::SupportedStateMachines;

use crate::pallet::Config;
use alloc::{collections::BTreeMap, string::ToString};
use codec::Decode;
use codec::Encode;
use evm_state_machine::EvmStateMachine;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateMachineClient,
		StateMachineHeight, StateMachineId, VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};
use op_verifier::OptimismDisputeGameProof;
use op_verifier::OptimismPayloadProof;
use op_verifier::{verify_optimism_dispute_game_proof, verify_optimism_payload};
use primitive_types::H256;

pub const OPTIMISM_CONSENSUS_ID: ConsensusClientId = *b"OPTC";

#[derive(codec::Encode, codec::Decode, Debug, PartialEq, Eq, Clone)]
pub struct ConsensusState {
	pub finalized_height: u64,
	pub state_machine_id: StateMachineId,
	pub l1_state_machine_id: StateMachineId,
	pub state_root: H256,
}

#[derive(Encode, Decode)]
pub struct OptimismUpdate {
	pub state_machine_id: StateMachineId,
	pub l1_height: u64,
	pub proof: OptimismConsensusProof,
}

/// Description of the various consensus mechanics supported for Optimism
#[derive(Encode, Decode, Debug)]
pub enum OptimismConsensusProof {
	OpL2Oracle(OptimismPayloadProof),
	OpFaultProofGames(OptimismDisputeGameProof),
}

pub struct OptimismConsensusClient<
	H: IsmpHost,
	C: Config,
	T: pallet_ismp_host_executive::Config + crate::pallet::Config,
>(core::marker::PhantomData<(H, C, T)>);

impl<
	H: IsmpHost + Send + Sync + Default + 'static,
	C: Config + Send + Sync + Default + 'static,
	T: pallet_ismp_host_executive::Config + pallet::Config + 'static,
> Default for OptimismConsensusClient<H, C, T>
{
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<
	H: IsmpHost + Send + Sync + Default + 'static,
	C: Config + Send + Sync + Default + 'static,
	T: pallet_ismp_host_executive::Config + pallet::Config + 'static,
> Clone for OptimismConsensusClient<H, C, T>
{
	fn clone(&self) -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<
	H: IsmpHost + Send + Sync + Default + 'static,
	C: Config + Send + Sync + Default + 'static,
	T: pallet_ismp_host_executive::Config + pallet::Config + 'static,
> ConsensusClient for OptimismConsensusClient<H, C, T>
{
	fn verify_consensus(
		&self,
		host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		consensus_proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		let OptimismUpdate { state_machine_id, l1_height, proof } =
			OptimismUpdate::decode(&mut &consensus_proof[..])
				.map_err(|_| Error::Custom("Cannot decode optimism update".to_string()))?;

		let mut consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::Custom("Cannot decode trusted consensus state".to_string()))?;

		let l1_state_machine_height =
			StateMachineHeight { id: consensus_state.l1_state_machine_id, height: l1_height };

		let l1_state_commitment = host.state_machine_commitment(l1_state_machine_height)?;
		let state_root = l1_state_commitment.state_root;

		let mut state_machine_map: BTreeMap<StateMachine, Vec<StateCommitmentHeight>> =
			BTreeMap::new();

		match proof {
			OptimismConsensusProof::OpL2Oracle(payload_proof) => {
				if let Some(oracle_address) =
					Pallet::<T>::state_machines_oracle_addresses(state_machine_id)
				{
					let state = verify_optimism_payload::<H>(
						payload_proof,
						state_root,
						oracle_address,
						consensus_state_id.clone(),
					)?;

					let state_commitment_height = StateCommitmentHeight {
						commitment: state.commitment,
						height: state.height.height,
					};

					let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
					state_commitment_vec.push(state_commitment_height);
					state_machine_map
						.insert(consensus_state.state_machine_id.state_id, state_commitment_vec);

					consensus_state.finalized_height = state.height.height;
					consensus_state.state_root = state.commitment.state_root;
				}
			},
			OptimismConsensusProof::OpFaultProofGames(dispute_proof) => {
				if let Some((dispute_game_factory, respected_game_types)) =
					Pallet::<T>::state_machines_dispute_game_factories_types(state_machine_id)
				{
					let state = verify_optimism_dispute_game_proof::<H>(
						dispute_proof,
						state_root,
						dispute_game_factory,
						respected_game_types,
						consensus_state_id.clone(),
					)?;

					let state_commitment_height = StateCommitmentHeight {
						commitment: state.commitment,
						height: state.height.height,
					};

					let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
					state_commitment_vec.push(state_commitment_height);
					state_machine_map
						.insert(consensus_state.state_machine_id.state_id, state_commitment_vec);

					consensus_state.finalized_height = state.height.height;
					consensus_state.state_root = state.commitment.state_root;
				}
			},
		}

		Ok((consensus_state.encode(), state_machine_map))
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
		OPTIMISM_CONSENSUS_ID
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		if SupportedStateMachines::<T>::contains_key(id) {
			Ok(Box::new(<EvmStateMachine<H, T>>::default()))
		} else {
			Err(Error::Custom("State machine not supported".to_string()))
		}
	}
}
