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

pub mod migrations;
pub mod pallet;
use pallet::{Pallet, SupportedStateMachines};

/// Current storage version of `pallet-ismp-optimism`. Bumped to `1` alongside the
/// [`migrations::SeedDisputeGameConfigs`] migration that translates
/// `StateMachinesDisputeGameFactoriesTypes` from `(H160, Vec<u32>)` to the richer
/// `(H160, Vec<GameTypeConfig>)` layout and seeds the per-game-type verification configs.
pub const STORAGE_VERSION: polkadot_sdk::frame_support::traits::StorageVersion =
	polkadot_sdk::frame_support::traits::StorageVersion::new(1);

use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec::Vec};
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
use op_verifier::{
	GameTypeConfig, OptimismDisputeGameProof, OptimismPayloadProof,
	verify_optimism_dispute_game_proof, verify_optimism_payload,
};

pub const OPTIMISM_CONSENSUS_CLIENT_ID: ConsensusClientId = *b"OPTC";

#[derive(codec::Encode, codec::Decode, Debug, PartialEq, Eq, Clone)]
pub struct ConsensusState {
	pub finalized_height: u64,
	pub state_machine_id: StateMachineId,
	pub l1_state_machine_id: StateMachineId,
	pub optimism_consensus_type: Option<OptimismConsensusType>,
	/// Per-game-type verification configuration for `OpFaultProofGames`. See
	/// [`op_verifier::GameTypeConfig`]. The op-host reads the authoritative configuration from
	/// the `IsmpOptimism` pallet on Hyperbridge; this field is retained as informational
	/// metadata on the consensus state.
	pub game_type_configs: Option<Vec<GameTypeConfig>>,
}

impl ConsensusState {
	/// SCALE-decode the consensus state while tolerating schema drift in the
	/// trailing `game_type_configs` field.
	///
	/// `GameTypeConfig` gained the `expected_impl: H160` field after consensus
	/// states had already been seeded. Stored entries encoded before that
	/// change have 20 fewer bytes per config than the current struct expects,
	/// so a strict decode fails with "Not enough data to fill buffer" partway
	/// through the last field. Since the authoritative game-type configuration
	/// for verification is read from pallet storage (see
	/// `Pallet::state_machines_dispute_game_factories_types` in
	/// [`verify_consensus`]) rather than from this blob, we can safely treat
	/// the last field as `None` on old entries and let the consensus loop
	/// reseed the state over time.
	pub fn decode_tolerant(bytes: &[u8]) -> Result<Self, codec::Error> {
		if let Ok(state) = Self::decode(&mut &*bytes) {
			return Ok(state);
		}

		#[derive(codec::Decode)]
		struct PrefixOnly {
			finalized_height: u64,
			state_machine_id: StateMachineId,
			l1_state_machine_id: StateMachineId,
			optimism_consensus_type: Option<OptimismConsensusType>,
		}

		let prefix = PrefixOnly::decode(&mut &*bytes)?;
		Ok(Self {
			finalized_height: prefix.finalized_height,
			state_machine_id: prefix.state_machine_id,
			l1_state_machine_id: prefix.l1_state_machine_id,
			optimism_consensus_type: prefix.optimism_consensus_type,
			game_type_configs: None,
		})
	}
}

#[derive(Encode, Decode)]
pub struct OptimismUpdate {
	pub state_machine_id: StateMachineId,
	pub l1_height: u64,
	pub proof: OptimismConsensusProof,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
pub enum OptimismConsensusType {
	OpL2Oracle,
	OpFaultProofGames,
}

/// Description of the various consensus mechanics supported for Optimism
#[derive(Encode, Decode, Debug)]
pub enum OptimismConsensusProof {
	OpL2Oracle(OptimismPayloadProof),
	OpFaultProofGames(OptimismDisputeGameProof),
}

pub struct OptimismConsensusClient<
	H: IsmpHost,
	T: pallet_ismp_host_executive::Config + crate::pallet::Config,
>(core::marker::PhantomData<(H, T)>);

impl<
	H: IsmpHost + Send + Sync + Default + 'static,
	T: pallet_ismp_host_executive::Config + pallet::Config + 'static,
> Default for OptimismConsensusClient<H, T>
{
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<
	H: IsmpHost + Send + Sync + Default + 'static,
	T: pallet_ismp_host_executive::Config + pallet::Config + 'static,
> Clone for OptimismConsensusClient<H, T>
{
	fn clone(&self) -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<
	H: IsmpHost + Send + Sync + Default + 'static,
	T: pallet_ismp_host_executive::Config + pallet::Config + 'static,
> ConsensusClient for OptimismConsensusClient<H, T>
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

		let mut consensus_state = ConsensusState::decode_tolerant(&trusted_consensus_state)
			.map_err(|_| Error::Custom("Cannot decode trusted consensus state".to_string()))?;

		let l1_state_machine_height =
			StateMachineHeight { id: consensus_state.l1_state_machine_id, height: l1_height };

		let l1_state_commitment = host.state_machine_commitment(l1_state_machine_height)?;
		let state_root = l1_state_commitment.state_root;

		let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
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
				}
			},
			OptimismConsensusProof::OpFaultProofGames(dispute_proof) => {
				if let Some((dispute_game_factory, game_type_configs)) =
					Pallet::<T>::state_machines_dispute_game_factories_types(state_machine_id)
				{
					let state = verify_optimism_dispute_game_proof::<H>(
						dispute_proof,
						state_root,
						dispute_game_factory,
						game_type_configs,
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
		OPTIMISM_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		if SupportedStateMachines::<T>::contains_key(id) {
			Ok(Box::new(<EvmStateMachine<H, T>>::default()))
		} else {
			Err(Error::Custom("State machine not supported".to_string()))
		}
	}
}
