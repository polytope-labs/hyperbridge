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

//! ISMP Consensus Client for Pharos Network.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec, vec::Vec};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use evm_state_machine::EvmStateMachine;
use geth_primitives::Header;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};
pub use pharos_primitives::{Mainnet, Testnet};
use pharos_primitives::{PHAROS_ATLANTIC_CHAIN_ID, PHAROS_MAINNET_CHAIN_ID, ValidatorSet, VerifierState, VerifierStateUpdate};
use pharos_verifier::verify_pharos_block;
use polkadot_sdk::*;
use sp_core::H256;

/// Consensus state ID for Pharos
pub const PHAROS_CONSENSUS_CLIENT_ID: ConsensusStateId = *b"PHAR";

/// Consensus state for Pharos light client.
#[derive(codec::Encode, codec::Decode, Debug, Default, PartialEq, Eq, Clone)]
pub struct ConsensusState {
	pub current_validators: ValidatorSet,
	pub finalized_height: u64,
	pub finalized_hash: H256,
	pub current_epoch: u64,
	pub chain_id: u32,
}

impl From<ConsensusState> for VerifierState {
	fn from(state: ConsensusState) -> Self {
		VerifierState {
			current_validator_set: state.current_validators,
			finalized_block_number: state.finalized_height,
			finalized_hash: state.finalized_hash,
			current_epoch: state.current_epoch,
		}
	}
}

/// The Pharos consensus client.
pub struct PharosClient<
	H: IsmpHost,
	T: pallet_ismp_host_executive::Config,
	C: pharos_primitives::Config,
>(PhantomData<(H, T, C)>);

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config, C: pharos_primitives::Config> Default
	for PharosClient<H, T, C>
{
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config, C: pharos_primitives::Config> Clone
	for PharosClient<H, T, C>
{
	fn clone(&self) -> Self {
		Self(PhantomData)
	}
}

impl<
		H: IsmpHost + Send + Sync + Default + 'static,
		T: pallet_ismp_host_executive::Config,
		C: pharos_primitives::Config,
	> ConsensusClient for PharosClient<H, T, C>
{
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, ismp::consensus::VerifiedCommitments), Error> {
		let update = VerifierStateUpdate::decode(&mut &proof[..])
			.map_err(|_| Error::Custom("Cannot decode pharos client update".to_string()))?;

		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::Custom("Cannot decode trusted consensus state".to_string()))?;

		if consensus_state.finalized_height >= update.block_number() {
			return Err(Error::Custom("Expired update".to_string()));
		}

		let trusted_state = VerifierState {
			current_validator_set: consensus_state.current_validators.clone(),
			finalized_block_number: consensus_state.finalized_height,
			finalized_hash: consensus_state.finalized_hash,
			current_epoch: consensus_state.current_epoch,
		};

		let new_state = verify_pharos_block::<C, H>(trusted_state, update.clone())
			.map_err(|e| Error::Custom(alloc::format!("Verification failed: {:?}", e)))?;

		let state_commitment = StateCommitmentHeight {
			commitment: StateCommitment {
				timestamp: update.header.timestamp,
				overlay_root: None,
				state_root: update.header.state_root,
			},
			height: new_state.finalized_block_number,
		};

		let new_consensus_state = ConsensusState {
			current_validators: new_state.current_validator_set,
			finalized_height: new_state.finalized_block_number,
			finalized_hash: new_state.finalized_hash,
			current_epoch: new_state.current_epoch,
			chain_id: consensus_state.chain_id,
		};

		let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
			BTreeMap::new();
		state_machine_map.insert(
			StateMachineId {
				state_id: StateMachine::Evm(new_consensus_state.chain_id),
				consensus_state_id,
			},
			vec![state_commitment],
		);

		Ok((new_consensus_state.encode(), state_machine_map))
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		trusted_consensus_state: Vec<u8>,
		proof_1: Vec<u8>,
		proof_2: Vec<u8>,
	) -> Result<(), Error> {
		let update_1 = VerifierStateUpdate::decode(&mut &proof_1[..])
			.map_err(|_| Error::Custom("Cannot decode pharos update for proof 1".to_string()))?;

		let update_2 = VerifierStateUpdate::decode(&mut &proof_2[..])
			.map_err(|_| Error::Custom("Cannot decode pharos update for proof 2".to_string()))?;

		let header_1 = &update_1.header;
		let header_2 = &update_2.header;

		if header_1.number != header_2.number {
			return Err(Error::Custom("Invalid fraud proof: different block numbers".to_string()));
		}

		let header_1_hash = Header::from(header_1).hash::<H>();
		let header_2_hash = Header::from(header_2).hash::<H>();

		if header_1_hash == header_2_hash {
			return Err(Error::Custom("Invalid fraud proof: identical headers".to_string()));
		}

		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::Custom("Cannot decode trusted consensus state".to_string()))?;

		let trusted_state = VerifierState {
			current_validator_set: consensus_state.current_validators.clone(),
			finalized_block_number: consensus_state.finalized_height,
			finalized_hash: consensus_state.finalized_hash,
			current_epoch: consensus_state.current_epoch,
		};

		verify_pharos_block::<C, H>(trusted_state.clone(), update_1)
			.map_err(|_| Error::Custom("Failed to verify first header".to_string()))?;

		verify_pharos_block::<C, H>(trusted_state, update_2)
			.map_err(|_| Error::Custom("Failed to verify second header".to_string()))?;

		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		PHAROS_CONSENSUS_CLIENT_ID
	}

	fn state_machine(
		&self,
		id: StateMachine,
	) -> Result<Box<dyn StateMachineClient>, Error> {
		match id {
			StateMachine::Evm(chain_id)
				if chain_id == PHAROS_MAINNET_CHAIN_ID ||
					chain_id == PHAROS_ATLANTIC_CHAIN_ID =>
				Ok(Box::new(<EvmStateMachine<H, T>>::default())),
			state_machine =>
				Err(Error::Custom(alloc::format!("Unsupported state machine: {state_machine:?}"))),
		}
	}
}
