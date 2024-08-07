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

//! Pallet method definitions

use super::{Config, Pallet};
use evm_common::{derive_unhashed_map_key, presets::REQUEST_COMMITMENTS_SLOT};
use ismp::{
	handlers::validate_state_machine,
	host::{IsmpHost, StateMachine},
	messaging::{hash_request, hash_response, Proof},
	router::{GetRequest, GetResponse, Request, Response, StorageValue},
	Error,
};
use mmr_primitives::MerkleMountainRangeTree;
use pallet_ismp::{
	child_trie::RequestCommitments,
	dispatcher::{FeeMetadata, RequestMetadata},
	mmr::{Leaf, LeafIndexAndPos},
};
use sp_core::U256;

/// Message for processing state queries
pub struct GetRequestsWithProof {
	/// The associated Get requests
	pub requests: Vec<GetRequest>,
	/// Proof of these requests on the source chain
	pub source: Proof,
	/// State proof of the requested values in the Get requests.
	pub response: Proof,
	/// Address that should be credited with fees
	pub address: Vec<u8>,
}

impl<T: Config> Pallet<T>
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	<T as pallet_ismp::Config>::Balance: Into<u128>,
{
	pub fn handle_get_requests(
		GetRequestsWithProof { requests, source, response, address }: GetRequestsWithProof,
	) -> Result<(), Error> {
		// 1. Verify source proofs
		// 2. Extract fees
		// 3. Verify response proof
		// 4. insert GetResponse into mmr and request receipts
		// 5. emit Response events
		let mut checked = vec![];
		let host = <<T as Config>::IsmpHost>::default();
		for req in requests.iter() {
			let full = Request::Get(req.clone());

			// Get requests time out are relative to Hyperbridge
			if full.timed_out(host.timestamp()) {
				Err(Error::RequestTimeout { meta: full.clone().into() })?
			}

			// Source of the request must match the proof
			if full.source_chain() != source.height.id.state_id {
				Err(Error::RequestProofMetadataNotValid { meta: full.clone().into() })?
			}

			// This request has already been previously processed
			if host.request_receipt(&full).is_some() {
				Err(Error::DuplicateResponse { meta: full.into() })?
			}

			checked.push(req.clone());
		}

		// Ensure the proof height is equal to each retrieval height specified in the Get
		// requests
		if !checked.iter().all(|get| get.height == source.height.height) {
			Err(Error::InsufficientProofHeight)?
		}

		// Verify source proof
		let source_state_machine = validate_state_machine(&host, source.height)?;
		let state_root = host.state_machine_commitment(source.height)?;

		let source_storage_keys = get_request_keys::<T>(&requests, source.height.id.state_id);
		let result = source_state_machine.verify_state_proof(
			&host,
			source_storage_keys,
			state_root,
			&source,
		)?;

		let mut total_fee = Default::default();

		for (.., value) in result {
			if let Some(value) = value {
				let fee = {
					match source.height.id.state_id {
						StateMachine::Evm(_) => {
							use alloy_rlp::Decodable;
							let fee = alloy_primitives::U256::decode(&mut &*value)
								.map_err(|_| Error::Custom("Failed to decode fee".to_string()))?;
							U256::from_big_endian(&fee.to_be_bytes::<32>())
						},
						StateMachine::Beefy(_) |
						StateMachine::Grandpa(_) |
						StateMachine::Kusama(_) |
						StateMachine::Polkadot(_) => {
							use codec::Decode;
							let fee: u128 =
								pallet_ismp::dispatcher::RequestMetadata::<T>::decode(&mut &*value)
									.map_err(|_| Error::Custom("Failed to decode fee".to_string()))?
									.fee
									.fee
									.into();
							U256::from(fee)
						},
						// unsupported
						StateMachine::Tendermint(_) =>
							Err(Error::Custom("Unsupported State Machine".to_string()))?,
					}
				};

				total_fee += fee;
			} else {
				Err(Error::MembershipProofVerificationFailed(
					"Message contains a request that was not found in the proof".to_string(),
				))?
			}
		}

		pallet_ismp_relayer::Pallet::<T>::accumulate_fee(
			source.height.id.state_id,
			address.clone(),
			total_fee,
		);

		// Verify response proof
		let dest_state_machine = validate_state_machine(&host, response.height)?;
		let state_root = host.state_machine_commitment(response.height)?;

		// Insert GetResponses into mmr
		let mut get_responses = vec![];
		for req in requests {
			let values = dest_state_machine
				.verify_state_proof(&host, req.keys.clone(), state_root, &response)?
				.into_iter()
				.map(|(key, value)| StorageValue { key, value })
				.collect();

			let get_response = GetResponse { get: req, values };

			get_responses.push(get_response);
		}

		for get_response in get_responses {
			let full = Request::Get(get_response.get.clone());
			host.store_request_receipt(&full, &address)?;
			let meta = FeeMetadata::<T> { payer: [0u8; 32].into(), fee: Default::default() };
			Self::dispatch_get_response(get_response, meta)
				.map_err(|e| Error::Custom("Failed to dispatch get response".to_string()))?
		}

		Ok(())
	}

	/// Insert a get response into the MMR and dispatch an event
	pub fn dispatch_get_response(
		get_response: GetResponse,
		meta: FeeMetadata<T>,
	) -> Result<(), ismp::Error> {
		let full = Response::Get(get_response.clone());
		let commitment = hash_response::<<T as Config>::IsmpHost>(&full);
		let req_commitment =
			hash_request::<<T as Config>::IsmpHost>(&Request::Get(get_response.get.clone()));
		let event = pallet_ismp::Event::Response {
			request_nonce: full.nonce(),
			dest_chain: full.source_chain(),
			source_chain: full.dest_chain(),
			commitment,
		};
		let leaf_index_and_pos =
			<T as pallet_ismp::Config>::Mmr::push(Leaf::Response(Response::Get(get_response)));

		pallet_ismp::child_trie::ResponseCommitments::<T>::insert(
			commitment,
			RequestMetadata {
				mmr: LeafIndexAndPos {
					leaf_index: leaf_index_and_pos.index,
					pos: leaf_index_and_pos.position,
				},
				fee: meta,
				claimed: false,
			},
		);
		pallet_ismp::Responded::<T>::insert(req_commitment, true);
		pallet_ismp::Pallet::<T>::deposit_pallet_event(event);

		Ok(())
	}
}

fn get_request_keys<T: Config>(requests: &[GetRequest], source: StateMachine) -> Vec<Vec<u8>> {
	let mut keys = vec![];
	for req in requests {
		let full = Request::Get(req.clone());
		let commitment = hash_request::<<T as Config>::IsmpHost>(&full);

		match source {
			StateMachine::Evm(_) => {
				keys.push(
					derive_unhashed_map_key::<<T as Config>::IsmpHost>(
						commitment.0.to_vec(),
						REQUEST_COMMITMENTS_SLOT,
					)
					.0
					.to_vec(),
				);
			},
			StateMachine::Polkadot(_) |
			StateMachine::Kusama(_) |
			StateMachine::Grandpa(_) |
			StateMachine::Beefy(_) => keys.push(RequestCommitments::<T>::storage_key(commitment)),
			// unsupported
			StateMachine::Tendermint(_) => {},
		}
	}
	keys
}
