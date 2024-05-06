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
#![cfg_attr(not(feature = "std"), no_std)]

//! The [`StateMachineClient`] for connected substrate-based chains which checks for protocol fees

extern crate alloc;

use core::marker::PhantomData;

use alloc::{collections::BTreeMap, format, vec::Vec};
use codec::Decode;
use sp_runtime::traits::{BlakeTwo256, Keccak256, Zero};
use sp_trie::{LayoutV0, StorageProof, Trie, TrieDBBuilder};

use ismp::{
	consensus::{StateCommitment, StateMachineClient},
	host::{IsmpHost, StateMachine},
	messaging::{hash_request, hash_response, Proof},
	router::RequestResponse,
	Error,
};
use pallet_hyperbridge::{
	child_trie::{RequestPayments, ResponsePayments},
	VersionedHostParams,
};
use pallet_ismp::child_trie::{RequestCommitments, ResponseCommitments};
use pallet_ismp_host_executive::HostParam;
use substrate_state_machine::{HashAlgorithm, SubstrateStateMachine, SubstrateStateProof};

/// The [`StateMachineClient`] implementation for substrate-based chains connected to hyperbridge.
///
/// This performs extra checks to ensure that protocol fees have been paid for each request or
/// response.
pub struct HyperbridgeClientMachine<T, H> {
	/// The [`StateMachine`] for whom we are to verify proofs for
	state_machine: StateMachine,
	/// The inner substrate state machine
	client: SubstrateStateMachine<T>,
	/// phantom type for pinning generics
	_phantom: PhantomData<H>,
}

impl<T, H> From<StateMachine> for HyperbridgeClientMachine<T, H> {
	fn from(state_machine: StateMachine) -> Self {
		Self { state_machine, client: Default::default(), _phantom: Default::default() }
	}
}

impl<T, H> StateMachineClient for HyperbridgeClientMachine<T, H>
where
	T: pallet_ismp_host_executive::Config,
	T::Balance: Into<u128>,
	H: IsmpHost,
{
	fn verify_membership(
		&self,
		_host: &dyn IsmpHost,
		item: RequestResponse,
		state: StateCommitment,
		proof: &Proof,
	) -> Result<(), Error> {
		let state_proof: SubstrateStateProof = codec::Decode::decode(&mut &*proof.proof)
			.map_err(|e| Error::Custom(format!("failed to decode proof: {e:?}")))?;

		if !matches!(state_proof, SubstrateStateProof::OverlayProof { .. }) {
			Err(Error::Custom("Expected Overlay Proof".into()))?
		}

		let root = state.overlay_root.ok_or_else(|| {
			Error::Custom("Child trie root is not available for provided state commitment".into())
		})?;

		let commitments = match item {
			RequestResponse::Request(requests) => requests
				.into_iter()
				.map(|request| {
					let commitment = hash_request::<H>(&request);
					(
						RequestCommitments::<T>::storage_key(commitment),
						RequestPayments::storage_key(commitment),
						request.data().unwrap_or_default().len() as u128,
					)
				})
				.collect::<Vec<_>>(),
			RequestResponse::Response(responses) => responses
				.into_iter()
				.map(|response| {
					let commitment = hash_response::<H>(&response);
					(
						ResponseCommitments::<T>::storage_key(commitment),
						ResponsePayments::storage_key(commitment),
						response.encode().len() as u128,
					)
				})
				.collect::<Vec<_>>(),
		};

		let Some(HostParam::SubstrateHostParam(VersionedHostParams::V1(per_byte_fee))) =
			pallet_ismp_host_executive::Pallet::<T>::host_params(&self.state_machine)
		else {
			Err(Error::Custom(format!(
				"State machine host params not found for {}",
				self.state_machine
			)))?
		};

		match state_proof.hasher() {
			HashAlgorithm::Keccak => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<Keccak256>();
				let trie = TrieDBBuilder::<LayoutV0<Keccak256>>::new(&db, &root).build();

				for (commitment_key, payment_key, size) in commitments {
					trie.get(&commitment_key)
						.map_err(|e| Error::Custom(format!("Error reading state proof: {e:?}")))?
						.ok_or_else(|| {
							Error::Custom(format!(
								"Request commitment not present in path: {commitment_key:?}",
							))
						})?;

					// only check for payments if a fee is configured
					if per_byte_fee > Zero::zero() {
						let paid = trie
							.get(&payment_key)
							.map_err(|e| {
								Error::Custom(format!("Error reading state proof: {e:?}"))
							})?
							.map(|value| u128::decode(&mut &value[..]))
							.transpose()
							.map_err(|err| {
								Error::Custom(format!("Failed to decode payment receipt: {err:?}",))
							})?
							.ok_or_else(|| {
								Error::Custom(format!(
									"Request payment not present in path: {payment_key:?}",
								))
							})?;

						let cost = size * per_byte_fee.into();

						if cost > paid {
							Err(Error::Custom(format!(
								"Insufficient payment for request. Expected: {cost}, got: {paid}"
							)))?
						}
					}
				}
			},
			HashAlgorithm::Blake2 => {
				let db =
					StorageProof::new(state_proof.storage_proof()).into_memory_db::<BlakeTwo256>();
				let trie = TrieDBBuilder::<LayoutV0<BlakeTwo256>>::new(&db, &root).build();

				for (commitment_key, payment_key, size) in commitments {
					trie.get(&commitment_key)
						.map_err(|e| Error::Custom(format!("Error reading state proof: {e:?}")))?
						.ok_or_else(|| {
							Error::Custom(format!(
								"Response commitment not present in path: {commitment_key:?}",
							))
						})?;

					// only check for payments if a fee is configured
					if per_byte_fee > Zero::zero() {
						let paid = trie
							.get(&payment_key)
							.map_err(|e| {
								Error::Custom(format!("Error reading state proof: {e:?}"))
							})?
							.map(|value| u128::decode(&mut &value[..]))
							.transpose()
							.map_err(|err| {
								Error::Custom(format!("Failed to decode payment receipt: {err:?}",))
							})?
							.ok_or_else(|| {
								Error::Custom(format!(
									"Request payment not present in path: {payment_key:?}",
								))
							})?;

						let cost = size * per_byte_fee.into();

						if cost > paid {
							Err(Error::Custom(format!(
								"Insufficient payment for request. Expected: {cost}, got: {paid}"
							)))?
						}
					}
				}
			},
		};

		Ok(())
	}

	fn state_trie_key(&self, items: RequestResponse) -> Vec<Vec<u8>> {
		self.client.state_trie_key(items)
	}

	fn verify_state_proof(
		&self,
		host: &dyn IsmpHost,
		keys: Vec<Vec<u8>>,
		root: StateCommitment,
		proof: &Proof,
	) -> Result<BTreeMap<Vec<u8>, Option<Vec<u8>>>, Error> {
		self.client.verify_state_proof(host, keys, root, proof)
	}
}
