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

//! ISMP consensus client for the Tempo blockchain.
//!
//! Verifies commonware simplex finalization certificates (BLS12-381 threshold
//! signatures, MinSig variant) against Tempo's static network identity, and
//! delegates state proofs to the shared EVM state machine since Tempo is a
//! Reth-based EVM chain with standard MPT state commitments.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, format, vec, vec::Vec};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use evm_state_machine::EvmStateMachine;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId, VerifiedCommitments,
	},
	error::Error as IsmpError,
	host::{IsmpHost, StateMachine},
	messaging::{Keccak256, StateCommitmentHeight},
};
use tempo_verifier::{
	primitives::{ConsensusState, FraudProof, TempoClientUpdate},
	Error,
};

pub use tempo_verifier::primitives::{TEMPO_MAINNET_CHAIN_ID, TEMPO_TESTNET_CHAIN_ID};

/// Consensus state id for the Tempo consensus client.
pub const TEMPO_CONSENSUS_ID: ConsensusClientId = *b"TMPO";

/// The Tempo consensus client.
pub struct TempoClient<H: IsmpHost, T: pallet_ismp_host_executive::Config>(PhantomData<(H, T)>);

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Default for TempoClient<H, T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<H: IsmpHost, T: pallet_ismp_host_executive::Config> Clone for TempoClient<H, T> {
	fn clone(&self) -> Self {
		Self(PhantomData)
	}
}

impl<
		H: IsmpHost + Keccak256 + Send + Sync + Default + 'static,
		T: pallet_ismp_host_executive::Config,
	> ConsensusClient for TempoClient<H, T>
{
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), IsmpError> {
		let update =
			TempoClientUpdate::decode(&mut &proof[..]).map_err(|_| Error::DecodeClientUpdate)?;
		let mut consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::DecodeConsensusState)?;

		// `verify_and_apply` enforces height monotonicity and the epoch-boundary
		// rule, then folds the result (including any identity rotation) into the
		// state. Going through it keeps this client from drifting out of step
		// with the verifier's own notion of a valid transition.
		let result = tempo_verifier::verify_and_apply::<H>(&mut consensus_state, &update)?;

		let state_commitment = StateCommitmentHeight {
			commitment: StateCommitment {
				timestamp: result.timestamp,
				overlay_root: None,
				state_root: result.state_root,
			},
			height: result.height,
		};

		let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
			BTreeMap::new();
		state_machine_map.insert(
			StateMachineId {
				state_id: StateMachine::Evm(consensus_state.chain_id),
				consensus_state_id,
			},
			vec![state_commitment],
		);

		Ok((consensus_state.encode(), state_machine_map))
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		trusted_consensus_state: Vec<u8>,
		proof_1: Vec<u8>,
		proof_2: Vec<u8>,
	) -> Result<(), IsmpError> {
		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|_| Error::DecodeConsensusState)?;
		let proof_1 =
			FraudProof::decode(&mut &proof_1[..]).map_err(|_| Error::DecodeClientUpdate)?;
		let proof_2 =
			FraudProof::decode(&mut &proof_2[..]).map_err(|_| Error::DecodeClientUpdate)?;

		tempo_verifier::verify_fraud_proof::<H>(&consensus_state, &proof_1, &proof_2)?;
		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		TEMPO_CONSENSUS_ID
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, IsmpError> {
		match id {
			StateMachine::Evm(TEMPO_MAINNET_CHAIN_ID)
			| StateMachine::Evm(TEMPO_TESTNET_CHAIN_ID) => Ok(Box::new(EvmStateMachine::<H, T>::default())),
			_ => Err(IsmpError::Custom(format!("State machine not supported: {id:?}"))),
		}
	}
}
