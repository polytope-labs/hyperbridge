// Copyright (c) 2025 Polytope Labs.
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
use polkadot_sdk::*;

use crate::{
	messages::{ConsensusMessage, SubstrateHeader},
	SupportedStateMachines,
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
use ismp::consensus::StateMachineId;
use sp_core::Get;
use sp_runtime::traits::Header;
use substrate_state_machine::{fetch_overlay_root_and_timestamp, SubstrateStateMachine};

/// [`ConsensusStateId`] for the polkadot relay chain
pub const POLKADOT_CONSENSUS_STATE_ID: ConsensusStateId = *b"polk";

/// [`ConsensusStateId`] for the kusama relay chain
pub const KUSAMA_CONSENSUS_STATE_ID: ConsensusStateId = *b"ksma";

/// [`ConsensusClientId`] for GRANDPA consensus
pub const GRANDPA_CONSENSUS_ID: ConsensusClientId = *b"GRNP";

pub struct GrandpaConsensusClient<T, S = SubstrateStateMachine<T>>(PhantomData<(T, S)>);

impl<T, S> Default for GrandpaConsensusClient<T, S> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T, S> ConsensusClient for GrandpaConsensusClient<T, S>
where
	T: pallet_ismp::Config + super::Config,
	S: StateMachineClient + From<StateMachine> + 'static,
{
	fn verify_consensus(
		&self,
		_host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
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
			ConsensusMessage::Polkadot(relay_chain_message) => {
				let headers_with_finality_proof = ParachainHeadersWithFinalityProof {
					finality_proof: relay_chain_message.finality_proof,
					parachain_headers: relay_chain_message.parachain_headers,
				};

				let (consensus_state, parachain_headers) =
					verify_parachain_headers_with_grandpa_finality_proof(
						consensus_state,
						headers_with_finality_proof,
					)
					.map_err(|err| {
						Error::Custom(format!("Error verifying parachain headers: {err:#?}"))
					})?;

				let parachain_headers = parachain_headers
					.into_iter()
					// filter out unknown para ids
					.filter_map(|(para_id, header)| {
						if let Some(slot_duration) =
							SupportedStateMachines::<T>::get(StateMachine::Polkadot(para_id))
								.or(SupportedStateMachines::<T>::get(StateMachine::Kusama(para_id)))
						{
							Some((para_id, header, slot_duration))
						} else {
							None
						}
					})
					.collect::<Vec<_>>();

				for (para_id, header_vec, slot_duration) in parachain_headers {
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
						let digest_result =
							fetch_overlay_root_and_timestamp(header.digest(), slot_duration)?;

						if digest_result.timestamp == 0 {
							Err(Error::Custom("Timestamp or ismp root not found".into()))?
						}

						let height: u32 = (*header.number()).into();

						let intermediate = match T::Coprocessor::get() {
							Some(id) if id == state_id => StateCommitmentHeight {
								// for the coprocessor, we only care about the child root & mmr root
								commitment: StateCommitment {
									timestamp: digest_result.timestamp,
									overlay_root: Some(digest_result.ismp_digest.mmr_root),
									state_root: digest_result.ismp_digest.child_trie_root, /* child root */
								},
								height: height.into(),
							},
							_ => StateCommitmentHeight {
								commitment: StateCommitment {
									timestamp: digest_result.timestamp,
									overlay_root: Some(digest_result.ismp_digest.child_trie_root),
									state_root: header.state_root,
								},
								height: height.into(),
							},
						};

						state_commitments_vec.push(intermediate);
					}

					intermediates.insert(
						StateMachineId { state_id, consensus_state_id },
						state_commitments_vec,
					);
				}

				Ok((consensus_state.encode(), intermediates))
			},
			ConsensusMessage::StandaloneChain(standalone_chain_message) => {
				let (consensus_state, header, _, _) = verify_grandpa_finality_proof(
					consensus_state,
					standalone_chain_message.finality_proof,
				)
				.map_err(|err| {
					Error::Custom(format!("Error verifying grandpa header: {err:#?}"))
				})?;

				let slot_duration = SupportedStateMachines::<T>::get(consensus_state.state_machine)
					.ok_or_else(|| {
						Error::Custom(format!(
							"Error getting slot duration for state machine {}",
							consensus_state.state_machine
						))
					})?;
				let digest_result =
					fetch_overlay_root_and_timestamp(header.digest(), slot_duration)?;

				if digest_result.timestamp == 0 {
					Err(Error::Custom("Timestamp or ismp root not found".into()))?
				}

				let height: u32 = (*header.number()).into();

				let state_id = consensus_state.state_machine;

				let intermediate = StateCommitmentHeight {
					commitment: StateCommitment {
						timestamp: digest_result.timestamp,
						overlay_root: Some(digest_result.ismp_digest.child_trie_root),
						state_root: header.state_root,
					},
					height: height.into(),
				};

				let mut state_commitments_vec = Vec::new();
				state_commitments_vec.push(intermediate);

				intermediates
					.insert(StateMachineId { state_id, consensus_state_id }, state_commitments_vec);

				Ok((consensus_state.encode(), intermediates))
			},

			ConsensusMessage::Relaychain(relay_chain_message) => {
				let headers_with_finality_proof = ParachainHeadersWithFinalityProof {
					finality_proof: relay_chain_message.finality_proof,
					parachain_headers: relay_chain_message.parachain_headers,
				};

				let (consensus_state, parachain_headers) =
					verify_parachain_headers_with_grandpa_finality_proof(
						consensus_state,
						headers_with_finality_proof,
					)
					.map_err(|err| {
						Error::Custom(format!("Error verifying parachain headers: {err:#?}"))
					})?;

				let parachain_headers = parachain_headers
					.into_iter()
					// filter out unknown para ids
					.filter_map(|(para_id, header)| {
						if let Some(slot_duration) =
							SupportedStateMachines::<T>::get(StateMachine::Relay {
								relay: consensus_state_id,
								para_id,
							}) {
							Some((para_id, slot_duration, header))
						} else {
							None
						}
					})
					.collect::<Vec<_>>();

				for (para_id, slot_duration, header_vec) in parachain_headers {
					let mut state_commitments_vec = Vec::new();

					for header in header_vec {
						let digest_result =
							fetch_overlay_root_and_timestamp(header.digest(), slot_duration)?;

						if digest_result.timestamp == 0 {
							Err(Error::Custom("Timestamp or ismp root not found".into()))?
						}

						let height: u32 = (*header.number()).into();

						let intermediate = StateCommitmentHeight {
							commitment: StateCommitment {
								timestamp: digest_result.timestamp,
								overlay_root: Some(digest_result.ismp_digest.child_trie_root),
								state_root: header.state_root,
							},
							height: height.into(),
						};
						state_commitments_vec.push(intermediate);
					}

					intermediates.insert(
						StateMachineId {
							state_id: StateMachine::Relay { relay: consensus_state_id, para_id },
							consensus_state_id,
						},
						state_commitments_vec,
					);
				}

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
			return Err(Error::Custom(format!("Fraud proofs are for the same block",)));
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
			return Err(Error::Custom(format!("Fraud proofs are not for the same chain")));
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
			return Err(Error::Custom(format!("Fraud proofs are not for the same ancestor")));
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
		if SupportedStateMachines::<T>::contains_key(id) {
			Ok(Box::new(S::from(id)))
		} else {
			Err(Error::Custom(format!("Unsupported State Machine {id:?}")))
		}
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		GRANDPA_CONSENSUS_ID
	}
}
