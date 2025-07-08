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

//! ISMP Consensus Client for the Arbitrum Consensus Protocol.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_variables)]

extern crate alloc;

pub mod pallet;
use arbitrum_verifier::{
	ArbitrumBoldProof, ArbitrumPayloadProof, verify_arbitrum_bold, verify_arbitrum_payload,
};
use pallet::{Pallet, SupportedStateMachines};

use alloc::{boxed::Box, collections::BTreeMap, format, string::ToString, vec::Vec};
use codec::{Decode, Encode};
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

pub const ARBITRUM_CONSENSUS_CLIENT_ID: ConsensusClientId = *b"ARBC";

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct ConsensusState {
	pub finalized_height: u64,
	pub state_machine_id: StateMachineId,
	pub l1_state_machine_id: StateMachineId,
	pub arbitrum_consensus_type: ArbitrumConsensusType,
}

#[derive(Encode, Decode)]
pub struct ArbitrumUpdate {
	pub state_machine_id: StateMachineId,
	pub l1_height: u64,
	pub proof: ArbitrumConsensusProof,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
pub enum ArbitrumConsensusType {
	ArbitrumOrbit,
	ArbitrumBold,
}

/// Description of the various consensus mechanics supported for Arbitrum
#[derive(Encode, Decode, Debug)]
pub enum ArbitrumConsensusProof {
	ArbitrumOrbit(ArbitrumPayloadProof),
	ArbitrumBold(ArbitrumBoldProof),
}

pub struct ArbitrumConsensusClient<
	H: IsmpHost,
	T: pallet_ismp_host_executive::Config + crate::pallet::Config,
>(core::marker::PhantomData<(H, T)>);

impl<
	H: IsmpHost + Send + Sync + Default + 'static,
	T: pallet_ismp_host_executive::Config + pallet::Config + 'static,
> Default for ArbitrumConsensusClient<H, T>
{
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<
	H: IsmpHost + Send + Sync + Default + 'static,
	T: pallet_ismp_host_executive::Config + pallet::Config + 'static,
> Clone for ArbitrumConsensusClient<H, T>
{
	fn clone(&self) -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<
	H: IsmpHost + Send + Sync + Default + 'static,
	T: pallet_ismp_host_executive::Config + pallet::Config + 'static,
> ConsensusClient for ArbitrumConsensusClient<H, T>
{
	fn verify_consensus(
		&self,
		host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		consensus_proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		let ArbitrumUpdate { state_machine_id, l1_height, proof } =
			ArbitrumUpdate::decode(&mut &consensus_proof[..])
				.map_err(|_| Error::Custom("Cannot decode arbitrum update".to_string()))?;

		let mut consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::Custom("Cannot decode trusted consensus state".to_string()))?;

		let l1_state_machine_height =
			StateMachineHeight { id: consensus_state.l1_state_machine_id, height: l1_height };

		let l1_state_commitment = host.state_machine_commitment(l1_state_machine_height)?;
		let state_root = l1_state_commitment.state_root;

		let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
			BTreeMap::new();

		if let Some(rollup_core_address) =
			Pallet::<T>::state_machines_rollup_core_addresses(state_machine_id)
		{
			match proof {
				ArbitrumConsensusProof::ArbitrumOrbit(proof) => {
					let state = verify_arbitrum_payload::<H>(
						proof,
						state_root,
						rollup_core_address,
						consensus_state_id.clone(),
					)?;

					let state_commitment_height = StateCommitmentHeight {
						commitment: state.commitment,
						height: state.height.height,
					};

					let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
					state_commitment_vec.push(state_commitment_height);
					state_machine_map.insert(
						StateMachineId {
							state_id: consensus_state.state_machine_id.state_id,
							consensus_state_id: consensus_state
								.l1_state_machine_id
								.consensus_state_id,
						},
						state_commitment_vec,
					);

					consensus_state.finalized_height = state.height.height;
				},
				ArbitrumConsensusProof::ArbitrumBold(proof) => {
					let state = verify_arbitrum_bold::<H>(
						proof,
						state_root,
						rollup_core_address,
						consensus_state_id.clone(),
					)
					.map_err(|error| {
						Error::Custom(format!("Error verifying arbitrum payload {:?}", &error))
					})?;

					let state_commitment_height = StateCommitmentHeight {
						commitment: state.commitment,
						height: state.height.height,
					};

					let mut state_commitment_vec: Vec<StateCommitmentHeight> = Vec::new();
					state_commitment_vec.push(state_commitment_height);
					state_machine_map.insert(
						StateMachineId {
							state_id: consensus_state.state_machine_id.state_id,
							consensus_state_id: consensus_state
								.l1_state_machine_id
								.consensus_state_id,
						},
						state_commitment_vec,
					);

					consensus_state.finalized_height = state.height.height;
				},
			}
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
		ARBITRUM_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		if SupportedStateMachines::<T>::contains_key(id) {
			Ok(Box::new(<EvmStateMachine<H, T>>::default()))
		} else {
			Err(Error::Custom("State machine not supported".to_string()))
		}
	}
}
