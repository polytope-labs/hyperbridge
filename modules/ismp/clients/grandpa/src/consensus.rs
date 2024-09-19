// Copyright (c) 2024 Polytope Labs.
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
// See the License for the specific lang

use crate::{
	messages::{ConsensusMessage, SubstrateHeader},
	Parachains, SupportedStateMachines,
};
use alloc::{boxed::Box, collections::BTreeMap, format, vec::Vec};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use finality_grandpa::Chain;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};

use grandpa_verifier::{
	verify_grandpa_finality_proof, verify_parachain_headers_with_grandpa_finality_proof,
};
use grandpa_verifier_primitives::{
	justification::{AncestryChain, GrandpaJustification},
	ConsensusState, FinalityProof, ParachainHeadersWithFinalityProof,
};
use sp_core::Get;
use sp_runtime::traits::Header;
use substrate_state_machine::{fetch_overlay_root_and_timestamp, SubstrateStateMachine};

/// [`ConsensusStateId`] for the polkadot relay chain
pub const POLKADOT_CONSENSUS_STATE_ID: ConsensusStateId = *b"polk";

/// [`ConsensusStateId`] for the kusama relay chain
pub const KUSAMA_CONSENSUS_STATE_ID: ConsensusStateId = *b"ksma";

/// [`ConsensusClientId`] for GRANDPA consensus
pub const GRANDPA_CONSENSUS_ID: ConsensusClientId = *b"GRNP";

pub struct GrandpaConsensusClient<T>(PhantomData<T>);

impl<T> Default for GrandpaConsensusClient<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T> ConsensusClient for GrandpaConsensusClient<T>
where
	T: pallet_ismp::Config + super::Config,
{
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		_consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		// decode the proof into consensus message
		let consensus_message: ConsensusMessage =
			codec::Decode::decode(&mut &proof[..]).map_err(|e| {
				Error::Custom(format!("Cannot decode consensus message from proof: {e:?}",))
			})?;

		// decode the consensus state
		let consensus_state: ConsensusState =
			codec::Decode::decode(&mut &trusted_consensus_state[..]).map_err(|e| {
				Error::Custom(format!(
					"Cannot decode consensus state from trusted consensus state bytes: {e:?}",
				))
			})?;

		let mut intermediates = BTreeMap::new();

		// match over the message
		match consensus_message {
			ConsensusMessage::RelayChainMessage(relay_chain_message) => {
				let headers_with_finality_proof = ParachainHeadersWithFinalityProof {
					finality_proof: relay_chain_message.finality_proof,
					parachain_headers: relay_chain_message.parachain_headers,
				};

				let (consensus_state, parachain_headers) =
					verify_parachain_headers_with_grandpa_finality_proof(
						consensus_state,
						headers_with_finality_proof,
					)
					.map_err(|_| Error::Custom(format!("Error verifying parachain headers")))?;

				// filter out unknown para ids
				let parachain_headers = parachain_headers
					.into_iter()
					.filter(|(para_id, _)| Parachains::<T>::contains_key(para_id))
					.collect::<Vec<_>>();

				for (para_id, header_vec) in parachain_headers {
					let slot_duration =
						Parachains::<T>::get(para_id).expect("ParaId has been checked; qed");

					let mut state_commitments_vec = Vec::new();

					let state_id: StateMachine = match T::Coprocessor::get() {
						Some(StateMachine::Polkadot(_)) => StateMachine::Polkadot(para_id),
						Some(StateMachine::Kusama(_)) => StateMachine::Kusama(para_id),
						_ => Err(Error::Custom(
							"Coprocessor was not set, cannot determine para id state machine id"
								.into(),
						))?,
					};

					for header in header_vec {
						let (timestamp, overlay_root) =
							fetch_overlay_root_and_timestamp(header.digest(), slot_duration)?;

						if timestamp == 0 {
							Err(Error::Custom("Timestamp or ismp root not found".into()))?
						}

						let height: u32 = (*header.number()).into();

						let intermediate = StateCommitmentHeight {
							commitment: StateCommitment {
								timestamp,
								overlay_root: Some(overlay_root),
								state_root: header.state_root,
							},
							height: height.into(),
						};

						state_commitments_vec.push(intermediate);
					}

					intermediates.insert(state_id, state_commitments_vec);
				}

				Ok((consensus_state.encode(), intermediates))
			},

			ConsensusMessage::StandaloneChainMessage(standalone_chain_message) => {
				let (consensus_state, header, _, _) = verify_grandpa_finality_proof(
					consensus_state,
					standalone_chain_message.finality_proof,
				)
				.map_err(|_| Error::Custom("Error verifying parachain headers".parse().unwrap()))?;
				let (timestamp, overlay_root) = fetch_overlay_root_and_timestamp(
					header.digest(),
					consensus_state.slot_duration,
				)?;

				if timestamp == 0 {
					Err(Error::Custom("Timestamp or ismp root not found".into()))?
				}

				let height: u32 = (*header.number()).into();

				let state_id = consensus_state.state_machine.ok_or_else(|| {
					Error::Custom("State Machine Id was not set on consensus state".into())
				})?;

				let intermediate = StateCommitmentHeight {
					commitment: StateCommitment {
						timestamp,
						overlay_root: Some(overlay_root),
						state_root: header.state_root,
					},
					height: height.into(),
				};

				let mut state_commitments_vec = Vec::new();
				state_commitments_vec.push(intermediate);

				intermediates.insert(state_id, state_commitments_vec);

				Ok((consensus_state.encode(), intermediates))
			},
		}
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		trusted_consensus_state: Vec<u8>,
		proof_1: Vec<u8>,
		proof_2: Vec<u8>,
	) -> Result<(), Error> {
		// decode the consensus state
		let consensus_state: ConsensusState =
			codec::Decode::decode(&mut &trusted_consensus_state[..]).map_err(|e| {
				Error::Custom(format!(
					"Cannot decode consensus state from trusted consensus state bytes: {e:?}",
				))
			})?;

		let first_proof: FinalityProof<SubstrateHeader> = codec::Decode::decode(&mut &proof_1[..])
			.map_err(|e| {
				Error::Custom(format!(
					"Cannot decode first finality proof from proof_1 bytes: {e:?}",
				))
			})?;

		let second_proof: FinalityProof<SubstrateHeader> = codec::Decode::decode(&mut &proof_2[..])
			.map_err(|e| {
				Error::Custom(format!(
					"Cannot decode second finality proof from proof_2 bytes: {e:?}",
				))
			})?;

		if first_proof.block == second_proof.block {
			return Err(Error::Custom(format!("Fraud proofs are for the same block",)))
		}

		let first_headers = AncestryChain::<SubstrateHeader>::new(&first_proof.unknown_headers);
		let first_target = first_proof
			.unknown_headers
			.iter()
			.max_by_key(|h| *h.number())
			.ok_or_else(|| Error::Custom(format!("Unknown headers can't be empty!")))?;

		let second_headers = AncestryChain::<SubstrateHeader>::new(&second_proof.unknown_headers);
		let second_target = second_proof
			.unknown_headers
			.iter()
			.max_by_key(|h| *h.number())
			.ok_or_else(|| Error::Custom(format!("Unknown headers can't be empty!")))?;

		if first_target.hash() != first_proof.block || second_target.hash() != second_proof.block {
			return Err(Error::Custom(format!("Fraud proofs are not for the same chain")))
		}

		let first_base = first_proof
			.unknown_headers
			.iter()
			.min_by_key(|h| *h.number())
			.ok_or_else(|| Error::Custom(format!("Unknown headers can't be empty!")))?;
		first_headers
			.ancestry(first_base.hash(), first_target.hash())
			.map_err(|_| Error::Custom(format!("Invalid ancestry!")))?;

		let second_base = second_proof
			.unknown_headers
			.iter()
			.min_by_key(|h| *h.number())
			.ok_or_else(|| Error::Custom(format!("Unknown headers can't be empty!")))?;
		second_headers
			.ancestry(second_base.hash(), second_target.hash())
			.map_err(|_| Error::Custom(format!("Invalid ancestry!")))?;

		let first_parent = first_base.parent_hash();
		let second_parent = second_base.parent_hash();

		if first_parent != second_parent {
			return Err(Error::Custom(format!("Fraud proofs are not for the same ancestor")))
		}

		let first_justification =
			GrandpaJustification::<SubstrateHeader>::decode(&mut &first_proof.justification[..])
				.map_err(|_| Error::Custom(format!("Could not decode first justification")))?;

		let second_justification =
			GrandpaJustification::<SubstrateHeader>::decode(&mut &second_proof.justification[..])
				.map_err(|_| Error::Custom(format!("Could not decode second justification")))?;

		if first_proof.block != first_justification.commit.target_hash ||
			second_proof.block != second_justification.commit.target_hash
		{
			Err(Error::Custom(
                format!("First or second finality proof block hash does not match justification target hash")
            ))?
		}

		if first_justification.commit.target_hash != consensus_state.latest_hash &&
			second_justification.commit.target_hash != consensus_state.latest_hash
		{
			Err(Error::Custom(format!(
				"First or second justification does not match consensus latest hash"
			)))?
		}

		let first_valid = first_justification
			.verify(consensus_state.current_set_id, &consensus_state.current_authorities)
			.is_ok();
		let second_valid = second_justification
			.verify(consensus_state.current_set_id, &consensus_state.current_authorities)
			.is_ok();

		if !first_valid || !second_valid {
			Err(Error::Custom(format!("Invalid justification")))?
		}

		Ok(())
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		let is_supported_para = match &id {
			StateMachine::Polkadot(id) | StateMachine::Kusama(id) =>
				Parachains::<T>::contains_key(*id),
			_ => false,
		};
		if SupportedStateMachines::<T>::contains_key(id) || is_supported_para {
			Ok(Box::new(SubstrateStateMachine::<T>::default()))
		} else {
			Err(Error::Custom(format!("Unsupported State Machine {id:?}")))
		}
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		GRANDPA_CONSENSUS_ID
	}
}
