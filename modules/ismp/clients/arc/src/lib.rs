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

//! ISMP Consensus Client for the Arc Network.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, format, string::ToString, vec, vec::Vec};
use arc_primitives::{ValidatorSet, VerifierState, VerifierStateUpdate, ARC_TESTNET_CHAIN_ID};
use arc_verifier::{verify_arc_update, verify_certificate};
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
use polkadot_sdk::*;
use sp_core::H256;

/// Consensus client id for Arc.
pub const ARC_CONSENSUS_CLIENT_ID: ConsensusClientId = arc_primitives::ARC_CONSENSUS_ID;

/// Consensus state for the Arc light client.
#[derive(codec::Encode, codec::Decode, Debug, Default, PartialEq, Eq, Clone)]
pub struct ConsensusState {
	/// The active validator set as of `finalized_height`
	pub current_validators: ValidatorSet,
	/// The latest finalized block number
	pub finalized_height: u64,
	/// The hash of the latest finalized header
	pub finalized_hash: H256,
	/// The Arc EVM chain id
	pub chain_id: u32,
}

impl From<ConsensusState> for VerifierState {
	fn from(state: ConsensusState) -> Self {
		VerifierState {
			current_validators: state.current_validators,
			finalized_height: state.finalized_height,
			finalized_hash: state.finalized_hash,
		}
	}
}

/// The Arc consensus client.
pub struct ArcClient<H: IsmpHost, T: pallet_ismp_host_executive::Config>(PhantomData<(H, T)>);

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Default for ArcClient<H, T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Clone for ArcClient<H, T> {
	fn clone(&self) -> Self {
		Self(PhantomData)
	}
}

impl<H: IsmpHost + Send + Sync + Default + 'static, T: pallet_ismp_host_executive::Config>
	ConsensusClient for ArcClient<H, T>
{
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, ismp::consensus::VerifiedCommitments), Error> {
		let update = VerifierStateUpdate::decode(&mut &proof[..])
			.map_err(|e| Error::AnyHow(anyhow::anyhow!("{:?}", e).into()))?;

		let consensus_state =
			ConsensusState::decode(&mut &trusted_consensus_state[..]).map_err(|e| {
				Error::AnyHow(
					anyhow::anyhow!("Cannot decode trusted consensus state: {:?}", e).into(),
				)
			})?;

		let trusted_state: VerifierState = consensus_state.clone().into();

		let new_state = verify_arc_update::<H>(trusted_state, update.clone())
			.map_err(|e| Error::AnyHow(anyhow::Error::from(e).into()))?;

		let state_commitment = StateCommitmentHeight {
			commitment: StateCommitment {
				timestamp: update.header.timestamp,
				overlay_root: None,
				state_root: update.header.state_root,
			},
			height: new_state.finalized_height,
		};

		let new_consensus_state = ConsensusState {
			current_validators: new_state.current_validators,
			finalized_height: new_state.finalized_height,
			finalized_hash: new_state.finalized_hash,
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
		let update_1 = VerifierStateUpdate::decode(&mut &proof_1[..]).map_err(|e| {
			Error::AnyHow(anyhow::anyhow!("Cannot decode arc update for proof 1: {:?}", e).into())
		})?;

		let update_2 = VerifierStateUpdate::decode(&mut &proof_2[..]).map_err(|e| {
			Error::AnyHow(anyhow::anyhow!("Cannot decode arc update for proof 2: {:?}", e).into())
		})?;

		if update_1.certificate.height != update_2.certificate.height {
			return Err(Error::Custom("Invalid fraud proof: different block heights".to_string()));
		}

		if update_1.certificate.block_hash == update_2.certificate.block_hash {
			return Err(Error::Custom("Invalid fraud proof: identical block hashes".to_string()));
		}

		let consensus_state =
			ConsensusState::decode(&mut &trusted_consensus_state[..]).map_err(|e| {
				Error::AnyHow(
					anyhow::anyhow!("Cannot decode trusted consensus state: {:?}", e).into(),
				)
			})?;

		// Two conflicting certificates for the same height, each signed by a
		// quorum of the trusted validator set, prove equivocation. The headers
		// bind each certificate's block hash to real header contents.
		for update in [&update_1, &update_2] {
			let computed_hash = Header::from(&update.header).hash::<H>();
			if computed_hash != update.certificate.block_hash {
				return Err(Error::Custom(format!(
					"Invalid fraud proof: header hash {computed_hash} does not match certificate {}",
					update.certificate.block_hash
				)));
			}
			verify_certificate(&consensus_state.current_validators, &update.certificate)
				.map_err(|e| Error::AnyHow(anyhow::Error::from(e).into()))?;
		}

		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		ARC_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		match id {
			StateMachine::Evm(chain_id) if chain_id == ARC_TESTNET_CHAIN_ID =>
				Ok(Box::new(<EvmStateMachine<H, T>>::default())),
			state_machine =>
				Err(Error::Custom(format!("Unsupported state machine: {state_machine:?}"))),
		}
	}
}
