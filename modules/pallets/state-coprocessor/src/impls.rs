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

use super::{BalanceOf, Config, Event, Pallet};
use alloc::{string::ToString, vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::traits::fungible::Mutate;
use ismp::{
	events::RequestResponseHandled,
	handlers::validate_state_machine,
	host::IsmpHost,
	messaging::{dedup_requests, hash_get_response, hash_request, Proof},
	router::{GetRequest, GetResponse, Request, RequestResponse, StorageValue},
	Error,
};
use pallet_bandwidth::BandwidthGate;
use pallet_ismp::{
	dispatcher::{FeeMetadata, RequestMetadata},
	offchain::{Leaf, LeafIndexAndPos, OffchainDBProvider},
};
use sp_runtime::{
	traits::{Saturating, Zero},
	SaturatedConversion,
};

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
		let host = <<T as Config>::IsmpHost>::default();

		// Reject duplicate requests within the batch.
		let wrapped: Vec<Request> = requests.iter().cloned().map(Request::Get).collect();
		dedup_requests::<<T as Config>::IsmpHost>(&wrapped)?;

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

			// Proof must come from the requested chain
			if full.dest_chain() != response.height.id.state_id {
				Err(Error::RequestProofMetadataNotValid { meta: full.clone().into() })?
			}

			// This request has already been previously processed
			if host.request_receipt(&full).is_some() {
				Err(Error::DuplicateResponse { meta: full.into() })?
			}
		}

		// Ensure the proof height is equal to each retrieval height specified in the Get
		// requests
		if !requests.iter().all(|get| get.height == response.height.height) {
			Err(Error::InsufficientProofHeight)?
		}

		// Verify source proof
		let source_state_machine = validate_state_machine(&host, source.height)?;
		let state_root = host.state_machine_commitment(source.height)?;

		// Verify membership proof to ensure that requests where committed on source chain
		let all_requests = requests.clone().into_iter().map(|req| Request::Get(req)).collect();
		source_state_machine.verify_membership(
			&host,
			RequestResponse::Request(all_requests),
			state_root,
			&source,
		)?;

		// Verify response proof
		let dest_state_machine = validate_state_machine(&host, response.height)?;
		let state_root = host.state_machine_commitment(response.height)?;

		// Insert GetResponses into mmr
		let mut get_responses = vec![];
		// Total payload bytes across this batch, used to mint reputation to
		// the relayer named in `address`. Each request contributes the same
		// `max(payload, 32)` quantity that the bandwidth gate charges so the
		// mint stays proportional to the work paid for.
		let mut total_bytes: u32 = 0;
		for req in requests {
			let values: Vec<StorageValue> = dest_state_machine
				.verify_state_proof(&host, req.keys.clone(), state_root, &response)?
				.into_iter()
				.map(|(key, value)| StorageValue { key, value })
				.collect();

			// Meter the app's bandwidth: query payload (keys + context)
			// plus the storage values being returned. 32-byte floor
			// mirrors the on_accept precedent. Charged once per request
			// after proof verification so the value size is final.
			let value_bytes: usize =
				values.iter().map(|sv| sv.value.as_ref().map(|v| v.len()).unwrap_or(0)).sum();
			let payload_bytes: usize =
				req.keys.iter().map(|k| k.len()).sum::<usize>() + req.context.len() + value_bytes;
			let bytes = core::cmp::max(payload_bytes, 32) as u32;
			<T as Config>::BandwidthGate::try_consume(&req.source, &req.from, bytes)
				.map_err(|err| Error::Custom(alloc::format!("bandwidth gate: {err}")))?;
			total_bytes = total_bytes.saturating_add(bytes);

			let get_response = GetResponse { get: req, values };

			get_responses.push(get_response);
		}

		// Mint reputation tokens to the named relayer. The address is the
		// relayer's raw 32-byte public key as supplied by the coprocessor.
		// A zero rate disables minting and a malformed address simply skips
		// the mint — we don't want a non-32-byte address to fail the whole
		// batch since the response insertion below has no dependency on it.
		// The per-byte rate and reputation asset are inherited from
		// `pallet-messaging-incentives` so both pallets share one source of truth.
		let rate = pallet_messaging_incentives::MintPerByte::<T>::get();
		if !rate.is_zero() && total_bytes > 0 {
			if let Ok(bytes32) = <[u8; 32]>::try_from(address.as_slice()) {
				let relayer: T::AccountId = bytes32.into();
				let bytes_balance: BalanceOf<T> = (total_bytes as u128).saturated_into();
				let amount = rate.saturating_mul(bytes_balance);
				if !amount.is_zero() {
					match <T as pallet_messaging_incentives::Config>::ReputationAsset::mint_into(
						&relayer, amount,
					) {
						Ok(_) => Pallet::<T>::deposit_event(Event::ReputationMinted {
							relayer,
							bytes: total_bytes,
							amount,
						}),
						Err(err) => log::warn!(
							target: "ismp",
							"state-coprocessor: reputation mint failed for {total_bytes}b: {err:?}",
						),
					}
				}
			}
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

		let leaf_index_and_pos = <T as Config>::Mmr::push(Leaf::GetResponse(get_response));
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
