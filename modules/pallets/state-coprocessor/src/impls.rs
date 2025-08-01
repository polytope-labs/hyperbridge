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

use polkadot_sdk::*;

use super::{Config, Pallet};
use alloc::{string::ToString, vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode};
use evm_state_machine::{derive_unhashed_map_key, presets::REQUEST_COMMITMENTS_SLOT};
use ismp::{
	events::RequestResponseHandled,
	handlers::validate_state_machine,
	host::{IsmpHost, StateMachine},
	messaging::{hash_get_response, hash_request, Proof},
	router::{GetRequest, GetResponse, Request, RequestResponse, Response, StorageValue},
	Error,
};
use pallet_ismp::{
	child_trie::RequestCommitments,
	dispatcher::{FeeMetadata, RequestMetadata},
	offchain::{Leaf, LeafIndexAndPos, OffchainDBProvider},
};
use sp_core::U256;

/// Message for processing state queries
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, scale_info::TypeInfo,
)]
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
		if !checked.iter().all(|get| get.height == response.height.height) {
			Err(Error::InsufficientProofHeight)?
		}

		// Verify source proof
		let source_state_machine = validate_state_machine(&host, source.height)?;
		let state_root = host.state_machine_commitment(source.height)?;

		let source_storage_keys = get_request_keys::<T>(&requests, source.height.id.state_id);
		// Verify membership proof to ensure that requests where committed on source chain
		let all_requests = requests.clone().into_iter().map(|req| Request::Get(req)).collect();
		source_state_machine.verify_membership(
			&host,
			RequestResponse::Request(all_requests),
			state_root,
			&source,
		)?;

		// Extract the request fee from the proof
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
						s if s.is_evm() => {
							use alloy_rlp::Decodable;
							let fee = alloy_primitives::U256::decode(&mut &*value)
								.map_err(|_| Error::Custom("Failed to decode fee".to_string()))?;
							U256::from_big_endian(&fee.to_be_bytes::<32>())
						},
						s if s.is_substrate() => {
							let fee: u128 =
								pallet_ismp::dispatcher::RequestMetadata::<T>::decode(&mut &*value)
									.map_err(|_| Error::Custom("Failed to decode fee".to_string()))?
									.fee
									.fee
									.into();
							U256::from(fee)
						},
						// unsupported
						s => Err(Error::Custom(alloc::format!("Unsupported State Machine {s:?}")))?,
					}
				};

				total_fee += fee;
			}
		}

		if total_fee != Default::default() {
			pallet_ismp_relayer::Pallet::<T>::accumulate_fee_and_deposit_event(
				source.height.id.state_id,
				address.clone(),
				total_fee,
			);
		}

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
			Self::dispatch_get_response(get_response, address.clone())
				.map_err(|_| Error::Custom("Failed to dispatch get response".to_string()))?;
		}

		Ok(())
	}

	/// Insert a get response into the MMR and emits an event
	pub fn dispatch_get_response(
		get_response: GetResponse,
		address: Vec<u8>,
	) -> Result<(), ismp::Error> {
		let commitment = hash_get_response::<<T as Config>::IsmpHost>(&get_response);
		let req_commitment =
			hash_request::<<T as Config>::IsmpHost>(&Request::Get(get_response.get.clone()));
		let event = pallet_ismp::Event::Response {
			request_nonce: get_response.get.nonce,
			dest_chain: get_response.get.source,
			source_chain: get_response.get.dest,
			commitment,
			req_commitment,
		};

		let leaf_index_and_pos =
			<T as Config>::Mmr::push(Leaf::Response(Response::Get(get_response)));
		let meta = FeeMetadata::<T> { payer: [0u8; 32].into(), fee: Default::default() };

		pallet_ismp::child_trie::ResponseCommitments::<T>::insert(
			commitment,
			RequestMetadata {
				offchain: LeafIndexAndPos {
					leaf_index: leaf_index_and_pos.index,
					pos: leaf_index_and_pos.position,
				},
				fee: meta,
				claimed: true,
			},
		);
		pallet_ismp::Responded::<T>::insert(req_commitment, true);
		pallet_ismp::Pallet::<T>::deposit_event(event.into());
		let event = pallet_ismp::Event::GetRequestHandled(RequestResponseHandled {
			commitment: req_commitment,
			relayer: address.clone(),
		});

		pallet_ismp::Pallet::<T>::deposit_event(event.into());

		Ok(())
	}
}

/// Returns the storage keys for
fn get_request_keys<T: Config>(requests: &[GetRequest], source: StateMachine) -> Vec<Vec<u8>> {
	let mut keys = vec![];
	for req in requests {
		let full = Request::Get(req.clone());
		let commitment = hash_request::<<T as Config>::IsmpHost>(&full);

		match source {
			s if s.is_evm() => {
				keys.push(
					derive_unhashed_map_key::<<T as Config>::IsmpHost>(
						commitment.0.to_vec(),
						REQUEST_COMMITMENTS_SLOT,
					)
					.0
					.to_vec(),
				);
			},
			s if s.is_substrate() => keys.push(RequestCommitments::<T>::storage_key(commitment)),
			// unsupported
			_ => {},
		}
	}
	keys
}
