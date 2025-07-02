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
use evm_state_machine::construct_intermediate_state;

use crate::{
	pallet::{self, SupportedStatemachines},
	types::{BeaconClientUpdate, ConsensusState},
};
use evm_state_machine::EvmStateMachine;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateMachineClient, StateMachineId,
		VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};
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
		let BeaconClientUpdate { consensus_update } =
			BeaconClientUpdate::decode(&mut &consensus_proof[..])
				.map_err(|_| Error::Custom("Cannot decode beacon client update".to_string()))?;

		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::Custom("Cannot decode trusted consensus state".to_string()))?;

		let new_light_client_state =
			sync_committee_verifier::verify_sync_committee_attestation::<C>(
				consensus_state.light_client_state,
				consensus_update.clone(),
			)
			.map_err(|e| Error::Custom(format!("{:?}", e)))?;

		let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
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

		state_machine_map.insert(
			StateMachineId {
				state_id: StateMachine::Evm(consensus_state.chain_id),
				consensus_state_id,
			},
			state_commitment_vec,
		);

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
