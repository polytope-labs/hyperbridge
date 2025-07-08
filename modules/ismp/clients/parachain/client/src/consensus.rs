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

//! The parachain consensus client module
use polkadot_sdk::*;

use core::{marker::PhantomData, time::Duration};

use alloc::{boxed::Box, collections::BTreeMap, format, string::ToString, vec::Vec};
use codec::{Decode, Encode};
use core::fmt::Debug;
use cumulus_pallet_parachain_system::{RelaychainDataProvider, RelaychainStateProvider};
use frame_support::traits::Get;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId, VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};
use pallet_ismp::{ConsensusDigest, ISMP_ID};
use primitive_types::H256;
use sp_consensus_aura::{Slot, AURA_ENGINE_ID};
use sp_runtime::{
	app_crypto::sp_core::storage::StorageKey,
	generic::Header,
	traits::{BlakeTwo256, Header as _},
	DigestItem,
};
use sp_trie::StorageProof;
use substrate_state_machine::{read_proof_check_for_parachain, SubstrateStateMachine};

use crate::{Parachains, RelayChainOracle};

/// The parachain consensus client implementation for ISMP.
pub struct ParachainConsensusClient<T, R, S = SubstrateStateMachine<T>>(PhantomData<(T, R, S)>);

impl<T, R, S> Default for ParachainConsensusClient<T, R, S> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

/// Information necessary to prove the finality of a sibling parachain's state commitment to this
/// parachain.
#[derive(Debug, Encode, Decode)]
pub struct ParachainConsensusProof {
	/// Height of the relay chain for the given proof
	pub relay_height: u32,
	/// Storage proof for the parachain headers
	pub storage_proof: Vec<Vec<u8>>,
}

/// [`ConsensusClientId`] for [`ParachainConsensusClient`]
pub const PARACHAIN_CONSENSUS_ID: ConsensusClientId = *b"PARA";

/// [`ConsensusClientId`] for [`ParachainConsensusClient`] on Polkadot
pub const POLKADOT_CONSENSUS_ID: ConsensusStateId = *b"DOT0";

/// [`ConsensusClientId`] for [`ParachainConsensusClient`] on Paseo
pub const PASEO_CONSENSUS_ID: ConsensusStateId = *b"PAS0";

impl<T, R, S> ConsensusClient for ParachainConsensusClient<T, R, S>
where
	R: RelayChainOracle,
	T: pallet_ismp::Config + super::Config,
	S: StateMachineClient + From<StateMachine> + 'static,
{
	fn verify_consensus(
		&self,
		host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		let update: ParachainConsensusProof =
			codec::Decode::decode(&mut &proof[..]).map_err(|e| {
				Error::Custom(format!("Cannot decode parachain consensus proof: {e:?}"))
			})?;

		// first check our oracle's registry
		let root = R::state_root(update.relay_height)
			// not in our registry? ask parachain_system.
			.or_else(|| {
				let state = RelaychainDataProvider::<T>::current_relay_chain_state();

				if state.number == update.relay_height {
					Some(state.state_root)
				} else {
					None
				}
			})
			// well, we couldn't find it
			.ok_or_else(|| {
				Error::Custom(format!("Cannot find relay chain height: {}", update.relay_height))
			})?;

		let storage_proof = StorageProof::new(update.storage_proof);
		let mut intermediates = BTreeMap::new();

		let header_keys = Parachains::<T>::iter_keys().map(|id| parachain_header_storage_key(id).0);
		let headers =
			read_proof_check_for_parachain::<BlakeTwo256, _>(&root, storage_proof, header_keys)
				.map_err(|e| Error::Custom(format!("Error verifying parachain header {e:?}",)))?;

		for (key, header) in headers.into_iter() {
			let Some(header) = header else { continue };
			let mut state_commitments_vec = Vec::new();

			let id = codec::Decode::decode(&mut &key[(key.len() - 4)..])
				.map_err(|e| Error::Custom(format!("Error decoding parachain header: {e}")))?;

			let slot_duration = Parachains::<T>::get(id).expect("Parachain with ID exists; qed");

			// ideally all parachain headers are the same
			let header = Header::<u32, BlakeTwo256>::decode(&mut &*header)
				.map_err(|e| Error::Custom(format!("Error decoding parachain header: {e}")))?;

			let (mut timestamp, mut overlay_root, mut mmr_root) =
				(0, H256::default(), H256::default());
			for digest in header.digest().logs.iter() {
				match digest {
					DigestItem::PreRuntime(consensus_engine_id, value)
						if *consensus_engine_id == AURA_ENGINE_ID =>
					{
						let slot = Slot::decode(&mut &value[..])
							.map_err(|e| Error::Custom(format!("Cannot slot: {e:?}")))?;
						timestamp = Duration::from_millis(*slot * slot_duration).as_secs();
					},
					DigestItem::Consensus(consensus_engine_id, value)
						if *consensus_engine_id == ISMP_ID =>
					{
						let log = ConsensusDigest::decode(&mut &value[..]);
						if let Ok(log) = log {
							overlay_root = log.child_trie_root;
							mmr_root = log.mmr_root;
						} else {
							Err(Error::Custom(
								"Header contains an invalid ismp consensus log".into(),
							))?
						}
					},
					// don't really care about the rest
					_ => {},
				};
			}

			if timestamp == 0 {
				Err(Error::Custom("Timestamp not found".into()))?
			}

			let height: u32 = (*header.number()).into();

			let state_id = match host.host_state_machine() {
				StateMachine::Kusama(_) => StateMachine::Kusama(id),
				StateMachine::Polkadot(_) => StateMachine::Polkadot(id),
				_ => Err(Error::Custom("Host state machine should be a parachain".into()))?,
			};

			let intermediate = match T::Coprocessor::get() {
				Some(id) if id == state_id => StateCommitmentHeight {
					// for the coprocessor, we only care about the child root & mmr root
					commitment: StateCommitment {
						timestamp,
						overlay_root: Some(mmr_root),
						state_root: overlay_root, // child root
					},
					height: height.into(),
				},
				_ => StateCommitmentHeight {
					commitment: StateCommitment {
						timestamp,
						overlay_root: Some(overlay_root),
						state_root: header.state_root,
					},
					height: height.into(),
				},
			};

			state_commitments_vec.push(intermediate);
			intermediates
				.insert(StateMachineId { state_id, consensus_state_id }, state_commitments_vec);
		}

		Ok((state, intermediates))
	}

	fn verify_fraud_proof(
		&self,
		_host: &dyn IsmpHost,
		_trusted_consensus_state: Vec<u8>,
		_proof_1: Vec<u8>,
		_proof_2: Vec<u8>,
	) -> Result<(), Error> {
		// There are no fraud proofs for the parachain client
		Ok(())
	}

	fn consensus_client_id(&self) -> [u8; 4] {
		PARACHAIN_CONSENSUS_ID
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

/// This returns the storage key for a parachain header on the relay chain.
pub fn parachain_header_storage_key(para_id: u32) -> StorageKey {
	let mut storage_key = frame_support::storage::storage_prefix(b"Paras", b"Heads").to_vec();
	let encoded_para_id = para_id.encode();
	storage_key.extend_from_slice(sp_io::hashing::twox_64(&encoded_para_id).as_slice());
	storage_key.extend_from_slice(&encoded_para_id);
	StorageKey(storage_key)
}
