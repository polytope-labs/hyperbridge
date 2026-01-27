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

use alloc::collections::BTreeMap;
use beefy_verifier::{MerkleKeccak256, verify_consensus};
use beefy_verifier_primitives::{
	BeefyConsensusProof, ConsensusState, ParachainProof, RelaychainProof,
};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use ismp::{
	Error,
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId, VerifiedCommitments,
	},
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};
use ismp_parachain::Parachains;
use pallet_ismp::{ConsensusDigest, ISMP_ID, ISMP_TIMESTAMP_ID, TimestampDigest};
use polkadot_sdk::*;
use primitive_types::H256;
use rs_merkle::MerkleProof;
use sp_runtime::{
	DigestItem,
	generic::Header,
	traits::{BlakeTwo256, Header as _},
};
use substrate_state_machine::SubstrateStateMachine;
pub const BEEFY_CONSENSUS_ID: ConsensusClientId = *b"BEEF";

/// Beefy consensus client implementation
pub struct BeefyConsensusClient<H, T, S = SubstrateStateMachine<H>>(PhantomData<(H, T, S)>);

impl<
	H: IsmpHost + Send + Sync + Default + 'static,
	T: ismp_parachain::Config,
	S: StateMachineClient + From<StateMachine> + 'static,
> Default for BeefyConsensusClient<H, T, S>
{
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<H, T, S> ConsensusClient for BeefyConsensusClient<H, T, S>
where
	H: IsmpHost + Send + Sync + Default + 'static,
	T: ismp_parachain::Config,
	S: StateMachineClient + From<StateMachine> + 'static,
{
	fn verify_consensus(
		&self,
		host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		let consensus_proof: BeefyConsensusProof =
			codec::Decode::decode(&mut &proof[..]).map_err(|e| {
				Error::Custom(format!("Cannot decode consensus message from proof: {e:?}"))
			})?;

		let consensus_state: ConsensusState =
			codec::Decode::decode(&mut &trusted_consensus_state[..]).map_err(|e| {
				Error::Custom(format!(
					"Cannot decode consensus state from trusted consensus state: {e:?}"
				))
			})?;

		let verified_updates = verify_consensus::<H>(consensus_state, consensus_proof.clone())
			.map_err(|e| Error::Custom(format!("Error verifying Beefy consensus update: {e:?}")))?;

		let parachain_proof = consensus_proof.parachain;
		let parachains = parachain_proof.parachains;
		let heads_root: H256 = verified_updates.1;

		if parachains.is_empty() {
			return Ok((verified_updates.0, BTreeMap::new()));
		}

		let mut indexed_leaf_hashes = Vec::with_capacity(parachains.len());

		for para_header in &parachains {
			let leaf = (para_header.para_id, para_header.header.clone());
			let hash = H::keccak256(&leaf.encode()).0;
			indexed_leaf_hashes.push((para_header.index as usize, hash));
		}

		indexed_leaf_hashes.sort_by_key(|(index, _)| *index);

		let (leaf_indices, leaf_hashes): (Vec<usize>, Vec<[u8; 32]>) =
			indexed_leaf_hashes.into_iter().unzip();
		let proof_hashes: Vec<[u8; 32]> =
			parachain_proof.proof.iter().map(|node| (*node).into()).collect();
		let merkle_proof = MerkleProof::<MerkleKeccak256>::new(proof_hashes);
		let valid = merkle_proof.verify(
			heads_root.0,
			&leaf_indices,
			&leaf_hashes,
			parachain_proof.total_leaves as usize,
		);
		if !valid {
			return Err(Error::Custom("Error verifying Beefy consensus update".to_string()))
		}

		let mut intermediates = BTreeMap::new();
		for para_header in parachains {
			let mut state_commitments_vec = Vec::new();
			let header = Header::<u32, BlakeTwo256>::decode(&mut &*para_header.header)
				.map_err(|e| Error::Custom(format!("Error decoding parachain header: {e}")))?;

			if !Parachains::<T>::contains_key(para_header.para_id) {
				Err(Error::Custom(format!(
					"Parachain with id {} not registered",
					para_header.para_id
				)))?
			}

			let (mut timestamp, mut overlay_root) = (0, H256::default());

			for digest in header.digest().logs.iter() {
				match digest {
					DigestItem::Consensus(consensus_engine_id, value)
						if *consensus_engine_id == ISMP_TIMESTAMP_ID =>
					{
						let timestamp_digest =
							TimestampDigest::decode(&mut &value[..]).map_err(|e| {
								Error::Custom(format!("Failed to decode timestamp digest: {e:?}"))
							})?;
						timestamp = timestamp_digest.timestamp;
					},
					DigestItem::Consensus(consensus_engine_id, value)
						if *consensus_engine_id == ISMP_ID =>
					{
						let log = ConsensusDigest::decode(&mut &value[..]);
						if let Ok(log) = log {
							overlay_root = log.child_trie_root;
						} else {
							Err(Error::Custom(
								"Header contains an invalid ismp consensus log".into(),
							))?
						}
					},
					_ => {},
				};
			}
			if timestamp == 0 {
				Err(Error::Custom("Timestamp not found".into()))?
			}

			let state_id = match host.host_state_machine() {
				StateMachine::Kusama(_) => StateMachine::Kusama(para_header.para_id),
				StateMachine::Polkadot(_) => StateMachine::Polkadot(para_header.para_id),
				_ => Err(Error::Custom("Host state machine should be a parachain".into()))?,
			};

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
			intermediates
				.insert(StateMachineId { state_id, consensus_state_id }, state_commitments_vec);
		}

		Ok((verified_updates.0, intermediates))
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		trusted_consensus_state: Vec<u8>,
		proof_1: Vec<u8>,
		proof_2: Vec<u8>,
	) -> Result<(), Error> {
		let consensus_state: ConsensusState =
			codec::Decode::decode(&mut &trusted_consensus_state[..]).map_err(|e| {
				Error::Custom(format!(
					"Cannot decode consensus state from trusted consensus state: {e:?}"
				))
			})?;

		let first_proof: RelaychainProof =
			codec::Decode::decode(&mut &proof_1[..]).map_err(|e| {
				Error::Custom(format!(
					"Cannot decode first relay chain proof from proof_1 bytes: {e:?}"
				))
			})?;

		let second_proof: RelaychainProof =
			codec::Decode::decode(&mut &proof_2[..]).map_err(|e| {
				Error::Custom(format!(
					"Cannot decode second relay chain proof from proof_2 bytes: {e:?}"
				))
			})?;

		let first_commitment = &first_proof.signed_commitment.commitment;
		let second_commitment = &second_proof.signed_commitment.commitment;

		if first_commitment.block_number != second_commitment.block_number {
			return Err(Error::Custom("Fraud proofs must be for the same block number".to_string()))
		}

		if first_commitment.encode() == second_commitment.encode() {
			return Err(Error::Custom(
				"Fraud proofs have identical commitments, no equivocation".to_string(),
			))
		}

		let empty_parachain_proof =
			ParachainProof { parachains: vec![], proof: vec![], total_leaves: 0 };

		verify_consensus::<H>(
			consensus_state.clone(),
			BeefyConsensusProof { relay: first_proof, parachain: empty_parachain_proof.clone() },
		)
		.map_err(|e| Error::Custom(format!("First proof verification failed: {e:?}")))?;

		verify_consensus::<H>(
			consensus_state,
			BeefyConsensusProof { relay: second_proof, parachain: empty_parachain_proof },
		)
		.map_err(|e| Error::Custom(format!("Second proof verification failed: {e:?}")))?;

		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		BEEFY_CONSENSUS_ID
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		let para_id = match id {
			StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id,
			_ => Err(Error::Custom(
				"State Machine is not supported by this consensus client".to_string(),
			))?,
		};

		if !Parachains::<T>::contains_key(&para_id) {
			Err(Error::Custom(format!("Parachain with id {para_id} not registered")))?
		}

		Ok(Box::new(S::from(id)))
	}
}
