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

use core::marker::PhantomData;

use alloc::{collections::BTreeMap, format, string::ToString};
use arbitrum_verifier::{
	verify_arbitrum_bold, verify_arbitrum_payload, ArbitrumBoldProof, ArbitrumPayloadProof,
};
use codec::{Decode, Encode};
use evm_state_machine::construct_intermediate_state;
use polkadot_sdk::sp_core::H160;

use crate::{
	pallet::{self, SupportedStatemachines},
	types::{BeaconClientUpdate, ConsensusState, L2Consensus},
};
use evm_state_machine::EvmStateMachine;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, IntermediateState,
		StateMachineClient, VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};
use op_verifier::{
	verify_optimism_dispute_game_proof, verify_optimism_payload, OptimismDisputeGameProof,
	OptimismPayloadProof,
};
use sync_committee_primitives::constants::Config;

use crate::prelude::*;

pub use sync_committee_primitives::constants::{BEACON_CONSENSUS_ID, GNOSIS_CONSENSUS_ID};

// trait for L2-specific consensus clients
pub trait L2ConsensusClient {
	/// Verify the L2 state proof using the provided Ethereum state root
	fn verify_l2_state(
		&self,
		host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		ethereum_state_root: [u8; 32],
		proof: Vec<u8>,
	) -> Result<IntermediateState, Error>;

	/// Get the state machine ID this L2 client is responsible for
	fn state_machine_id(&self) -> StateMachine;
}

// Implement for Arbitrum Orbit
pub struct ArbitrumOrbitClient<H: IsmpHost + Send + Sync + Default + 'static> {
	state_machine_id: StateMachine,
	rollup_core_address: H160,
	_marker: PhantomData<H>,
}

impl<H: IsmpHost + Send + Sync + Default + 'static> ArbitrumOrbitClient<H> {
	pub fn new(state_machine_id: StateMachine, rollup_core_address: H160) -> Self {
		Self { state_machine_id, rollup_core_address, _marker: PhantomData }
	}
}

impl<H: IsmpHost + Send + Sync + Default + 'static> L2ConsensusClient for ArbitrumOrbitClient<H> {
	fn verify_l2_state(
		&self,
		_host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		ethereum_state_root: [u8; 32],
		proof: Vec<u8>,
	) -> Result<IntermediateState, Error> {
		verify_arbitrum_payload::<H>(
			ArbitrumPayloadProof::decode(&mut &proof[..]).unwrap(),
			ethereum_state_root.into(),
			self.rollup_core_address,
			consensus_state_id,
		)
	}

	fn state_machine_id(&self) -> StateMachine {
		self.state_machine_id
	}
}

// Implement for Arbitrum Bold
pub struct ArbitrumBoldClient<H: IsmpHost + Send + Sync + Default + 'static> {
	state_machine_id: StateMachine,
	rollup_core_address: H160,
	_marker: PhantomData<H>,
}

impl<H: IsmpHost + Send + Sync + Default + 'static> ArbitrumBoldClient<H> {
	pub fn new(state_machine_id: StateMachine, rollup_core_address: H160) -> Self {
		Self { state_machine_id, rollup_core_address, _marker: PhantomData }
	}
}

impl<H: IsmpHost + Send + Sync + Default + 'static> L2ConsensusClient for ArbitrumBoldClient<H> {
	fn verify_l2_state(
		&self,
		_host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		ethereum_state_root: [u8; 32],
		proof: Vec<u8>,
	) -> Result<IntermediateState, Error> {
		verify_arbitrum_bold::<H>(
			ArbitrumBoldProof::decode(&mut &proof[..]).unwrap(),
			ethereum_state_root.into(),
			self.rollup_core_address,
			consensus_state_id,
		)
		.map_err(|e| Error::Custom(e.to_string()))
	}

	fn state_machine_id(&self) -> StateMachine {
		self.state_machine_id
	}
}

// Implement for Optimism L2Oracle
pub struct OptimismL2OracleClient<H: IsmpHost + Send + Sync + Default + 'static> {
	state_machine_id: StateMachine,
	l2_oracle: H160,
	_marker: PhantomData<H>,
}

impl<H: IsmpHost + Send + Sync + Default + 'static> OptimismL2OracleClient<H> {
	pub fn new(state_machine_id: StateMachine, l2_oracle: H160) -> Self {
		Self { state_machine_id, l2_oracle, _marker: PhantomData }
	}
}

impl<H: IsmpHost + Send + Sync + Default + 'static> L2ConsensusClient
	for OptimismL2OracleClient<H>
{
	fn verify_l2_state(
		&self,
		_host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		ethereum_state_root: [u8; 32],
		proof: Vec<u8>,
	) -> Result<IntermediateState, Error> {
		verify_optimism_payload::<H>(
			OptimismPayloadProof::decode(&mut &proof[..]).unwrap(),
			ethereum_state_root.into(),
			self.l2_oracle,
			consensus_state_id,
		)
	}

	fn state_machine_id(&self) -> StateMachine {
		self.state_machine_id
	}
}

// Implement for Optimism Fault Proofs
pub struct OptimismFaultProofClient<H: IsmpHost + Send + Sync + Default + 'static> {
	state_machine_id: StateMachine,
	dispute_game_factory: H160,
	respected_game_types: Vec<u32>,
	_marker: PhantomData<H>,
}

impl<H: IsmpHost + Send + Sync + Default + 'static> OptimismFaultProofClient<H> {
	pub fn new(
		state_machine_id: StateMachine,
		dispute_game_factory: H160,
		respected_game_types: Vec<u32>,
	) -> Self {
		Self { state_machine_id, dispute_game_factory, respected_game_types, _marker: PhantomData }
	}
}

impl<H: IsmpHost + Send + Sync + Default + 'static> L2ConsensusClient
	for OptimismFaultProofClient<H>
{
	fn verify_l2_state(
		&self,
		_host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		ethereum_state_root: [u8; 32],
		proof: Vec<u8>,
	) -> Result<IntermediateState, Error> {
		verify_optimism_dispute_game_proof::<H>(
			OptimismDisputeGameProof::decode(&mut &proof[..]).unwrap(),
			ethereum_state_root.into(),
			self.dispute_game_factory,
			self.respected_game_types.clone(),
			consensus_state_id,
		)
	}

	fn state_machine_id(&self) -> StateMachine {
		self.state_machine_id
	}
}

// Now we refactor the main SyncCommitteeConsensusClient to use these L2 clients
pub struct SyncCommitteeConsensusClient<
	H: IsmpHost + Send + Sync + Default + 'static,
	C: Config + Send + Sync + Default + 'static,
	T: pallet_ismp_host_executive::Config + pallet::Config<I> + 'static,
	I: 'static,
> {
	l2_clients: Vec<Box<dyn L2ConsensusClient>>,
	_marker: PhantomData<(H, C, T, I)>,
}

impl<
		H: IsmpHost + Send + Sync + Default + 'static,
		C: Config + Send + Sync + Default + 'static,
		T: pallet_ismp_host_executive::Config + pallet::Config<I> + 'static,
		I: 'static,
	> SyncCommitteeConsensusClient<H, C, T, I>
{
	pub fn new() -> Self {
		Self { l2_clients: Vec::new(), _marker: PhantomData }
	}

	pub fn register_l2_client(&mut self, client: Box<dyn L2ConsensusClient>) {
		self.l2_clients.push(client);
	}

	pub fn from_consensus_state(consensus_state: &ConsensusState) -> Result<Self, Error> {
		let mut client = Self::new();

		for (state_machine, consensus_mechanic) in &consensus_state.l2_consensus {
			match consensus_mechanic {
				L2Consensus::ArbitrumOrbit(rollup_core_address) => {
					let l2_client =
						ArbitrumOrbitClient::<H>::new(state_machine.clone(), *rollup_core_address);
					client.register_l2_client(Box::new(l2_client));
				},
				L2Consensus::ArbitrumBold(rollup_core_address) => {
					let l2_client =
						ArbitrumBoldClient::<H>::new(state_machine.clone(), *rollup_core_address);
					client.register_l2_client(Box::new(l2_client));
				},
				L2Consensus::OpL2Oracle(l2_oracle) => {
					let l2_client =
						OptimismL2OracleClient::<H>::new(state_machine.clone(), *l2_oracle);
					client.register_l2_client(Box::new(l2_client));
				},
				L2Consensus::OpFaultProofs((dispute_game_factory, respected_game_type)) => {
					let l2_client = OptimismFaultProofClient::<H>::new(
						state_machine.clone(),
						*dispute_game_factory,
						vec![*respected_game_type],
					);
					client.register_l2_client(Box::new(l2_client));
				},
				L2Consensus::OpFaultProofGames((dispute_game_factory, respected_game_types)) => {
					let l2_client = OptimismFaultProofClient::<H>::new(
						state_machine.clone(),
						*dispute_game_factory,
						respected_game_types.clone(),
					);
					client.register_l2_client(Box::new(l2_client));
				},
			}
		}

		Ok(client)
	}
}

// Updated implementation of the ConsensusClient trait
impl<
		H: IsmpHost + Send + Sync + Default + 'static,
		C: Config + Send + Sync + Default + 'static,
		T: pallet_ismp_host_executive::Config + pallet::Config<I> + 'static,
		I: 'static,
	> ConsensusClient for SyncCommitteeConsensusClient<H, C, T, I>
{
	fn verify_consensus(
		&self,
		host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		consensus_proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::Custom("Cannot decode trusted consensus state".to_string()))?;

		// Parse update based on the type
		let update = match BeaconClientUpdate::decode(&mut &consensus_proof[..]) {
			Ok(update) => update,
			Err(_) => {
				// Try to decode as a standalone L2 update
				match L2StateUpdate::decode(&mut &consensus_proof[..]) {
					Ok(l2_update) => {
						// Handle standalone L2 update without changing beacon state
						return self.process_l2_update(
							host,
							consensus_state_id,
							consensus_state,
							l2_update,
						);
					},
					Err(_) => {
						return Err(Error::Custom("Cannot decode update".to_string()));
					},
				}
			},
		};

		// Process full beacon update with optional L2 updates
		self.process_beacon_update(host, consensus_state_id, consensus_state, update)
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

// Add helper methods for the consensus client
impl<
		H: IsmpHost + Send + Sync + Default + 'static,
		C: Config + Send + Sync + Default + 'static,
		T: pallet_ismp_host_executive::Config + pallet::Config<I> + 'static,
		I: 'static,
	> SyncCommitteeConsensusClient<H, C, T, I>
{
	fn process_beacon_update(
		&self,
		host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		consensus_state: ConsensusState,
		update: BeaconClientUpdate,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		let BeaconClientUpdate {
			l2_oracle_payload,
			dispute_game_payload,
			consensus_update,
			arbitrum_payload,
			arbitrum_bold,
		} = update;

		// Verify sync committee update
		let new_light_client_state =
			sync_committee_verifier::verify_sync_committee_attestation::<C>(
				consensus_state.light_client_state,
				consensus_update.clone(),
			)
			.map_err(|e| Error::Custom(format!("{:?}", e)))?;

		let state_root = consensus_update.execution_payload.state_root;

		// Process Ethereum state update
		let mut state_machine_map = BTreeMap::new();
		let ethereum_state = construct_intermediate_state(
			StateMachine::Evm(consensus_state.chain_id),
			consensus_state_id.clone(),
			consensus_update.execution_payload.block_number,
			consensus_update.execution_payload.timestamp,
			&state_root[..],
		)?;

		let ethereum_commitment = StateCommitmentHeight {
			commitment: ethereum_state.commitment,
			height: ethereum_state.height.height,
		};

		state_machine_map
			.insert(StateMachine::Evm(consensus_state.chain_id), vec![ethereum_commitment]);

		// Process L2 updates if any
		self.process_l2_updates(
			host,
			&mut state_machine_map,
			consensus_state_id,
			state_root.into(),
			l2_oracle_payload,
			dispute_game_payload,
			arbitrum_payload,
			arbitrum_bold,
		)?;

		// Create new consensus state
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

	fn process_l2_update(
		&self,
		host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		consensus_state: ConsensusState,
		l2_update: L2StateUpdate,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		let L2StateUpdate { state_machine_id, ethereum_block_hash, ethereum_state_root, proof } =
			l2_update;

		// Find the L2 client for this state machine
		let l2_client = self
			.l2_clients
			.iter()
			.find(|client| client.state_machine_id() == state_machine_id)
			.ok_or_else(|| {
				Error::Custom(format!(
					"No L2 client registered for state machine {:?}",
					state_machine_id
				))
			})?;

		// Verify the L2 state proof
		let intermediate_state =
			l2_client.verify_l2_state(host, consensus_state_id, ethereum_state_root, proof)?;

		// Create verified commitment
		let state_commitment = StateCommitmentHeight {
			commitment: intermediate_state.commitment,
			height: intermediate_state.height.height,
		};

		let mut state_machine_map = BTreeMap::new();
		state_machine_map.insert(state_machine_id, vec![state_commitment]);

		// Return unchanged consensus state with the verified L2 state
		Ok((consensus_state.encode(), state_machine_map))
	}

	fn process_l2_updates(
		&self,
		host: &dyn IsmpHost,
		state_machine_map: &mut BTreeMap<StateMachine, Vec<StateCommitmentHeight>>,
		consensus_state_id: ConsensusStateId,
		ethereum_state_root: [u8; 32],
		l2_oracle_payload: BTreeMap<StateMachine, OptimismPayloadProof>,
		dispute_game_payload: BTreeMap<StateMachine, OptimismDisputeGameProof>,
		arbitrum_payload: BTreeMap<StateMachine, ArbitrumPayloadProof>,
		arbitrum_bold: BTreeMap<StateMachine, ArbitrumBoldProof>,
	) -> Result<(), Error> {
		// Process all L2 updates using the registered L2 clients
		for l2_client in &self.l2_clients {
			let state_machine = l2_client.state_machine_id();

			// Check if we have a payload for this state machine
			let maybe_proof = match &state_machine {
				sm if arbitrum_payload.contains_key(sm) => {
					Some(arbitrum_payload.get(sm).unwrap().encode())
				},
				sm if arbitrum_bold.contains_key(sm) => {
					Some(arbitrum_bold.get(sm).unwrap().encode())
				},
				sm if l2_oracle_payload.contains_key(sm) => {
					Some(l2_oracle_payload.get(sm).unwrap().encode())
				},
				sm if dispute_game_payload.contains_key(sm) => {
					Some(dispute_game_payload.get(sm).unwrap().encode())
				},
				_ => None,
			};

			if let Some(proof) = maybe_proof {
				let state = l2_client.verify_l2_state(
					host,
					consensus_state_id.clone(),
					ethereum_state_root,
					proof,
				)?;

				let state_commitment = StateCommitmentHeight {
					commitment: state.commitment,
					height: state.height.height,
				};

				state_machine_map.insert(state_machine, vec![state_commitment]);
			}
		}

		Ok(())
	}
}

// Define a standalone L2 update format for independent L2 updates
#[derive(Encode, Decode, Debug)]
pub struct L2StateUpdate {
	pub state_machine_id: StateMachine,
	pub ethereum_block_hash: [u8; 32],
	pub ethereum_state_root: [u8; 32],
	pub proof: Vec<u8>,
}

// Creates an L2 consensus client based on the consensus mechanic
pub fn create_l2_client<H: IsmpHost + Send + Sync + Default + 'static>(
	state_machine: StateMachine,
	consensus_mechanic: &L2Consensus,
) -> Box<dyn L2ConsensusClient> {
	match consensus_mechanic {
		L2Consensus::ArbitrumOrbit(rollup_core_address) => {
			Box::new(ArbitrumOrbitClient::<H>::new(state_machine, *rollup_core_address))
		},
		L2Consensus::ArbitrumBold(rollup_core_address) => {
			Box::new(ArbitrumBoldClient::<H>::new(state_machine, *rollup_core_address))
		},
		L2Consensus::OpL2Oracle(l2_oracle) => {
			Box::new(OptimismL2OracleClient::<H>::new(state_machine, *l2_oracle))
		},
		L2Consensus::OpFaultProofs((dispute_game_factory, respected_game_type)) => {
			Box::new(OptimismFaultProofClient::<H>::new(
				state_machine,
				*dispute_game_factory,
				vec![*respected_game_type],
			))
		},
		L2Consensus::OpFaultProofGames((dispute_game_factory, respected_game_types)) => {
			Box::new(OptimismFaultProofClient::<H>::new(
				state_machine,
				*dispute_game_factory,
				respected_game_types.clone(),
			))
		},
	}
}
