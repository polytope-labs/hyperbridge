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
// See the License for the specific language governing permissions and
// limitations under the License.

//! Pallet methods
use polkadot_sdk::*;

use crate::{
	child_trie::{RequestCommitments, ResponseCommitments},
	dispatcher::{FeeMetadata, RequestMetadata},
	fee_handler::FeeHandler,
	offchain::{self, ForkIdentifier, Leaf, LeafIndexAndPos, OffchainDBProvider},
	Config, Error, Event, Pallet, Responded,
};
use alloc::{string::ToString, vec, vec::Vec};
use codec::Decode;
use frame_system::Phase;
use ismp::{
	events,
	handlers::{handle_incoming_message, MessageResult},
	messaging::{hash_request, hash_response, Message, MessageWithWeight},
	router::{Request, Response},
};
use sp_core::{offchain::StorageKind, H256};

impl<T: Config> Pallet<T> {
	/// Execute the provided ISMP datagrams, this will short circuit if any messages are invalid.
	/// This also charges fee on valid message delivery
	pub fn execute(messages: Vec<Message>) -> Result<Vec<events::Event>, Error<T>> {
		let host = Pallet::<T>::default();

		let message_results = messages
			.iter()
			.map(|msg| handle_incoming_message(&host, msg.clone()))
			.collect::<Result<Vec<_>, _>>()
			.map_err(|err| {
				log::debug!(target: "ismp", "Handling Error {:#?}", err);
				Pallet::<T>::deposit_event(Event::<T>::Errors { errors: vec![err.into()] });
				Error::<T>::InvalidMessage
			})?;

		let messages_with_weights = message_results
			.iter()
			.zip(messages)
			.map(|(result, message)| MessageWithWeight { message, weight: result.weight() })
			.collect::<Vec<_>>();

		let events = message_results
			.into_iter()
			// check that requests will be successfully dispatched
			// so we can not be spammed with failing txs
			.map(|result| match result {
				MessageResult::Request { events, .. } |
				MessageResult::Response { events, .. } |
				MessageResult::Timeout { events, .. } => events,
				MessageResult::ConsensusMessage(events) => events.into_iter().map(Ok).collect(),
				MessageResult::FrozenClient(_) => vec![],
			})
			.flatten()
			.collect::<Result<Vec<_>, _>>()
			.map_err(|err| {
				log::debug!(target: "ismp", "Handling Error {:#?}", err);
				Pallet::<T>::deposit_event(Event::<T>::Errors { errors: vec![err.into()] });
				Error::<T>::InvalidMessage
			})?;

		T::FeeHandler::on_executed(messages_with_weights, events.clone())
			.map_err(|_| Error::<T>::ErrorChargingFee)?;

		for event in events.clone() {
			// deposit any relevant events
			Pallet::<T>::deposit_event(event.into());
		}

		Ok(events)
	}

	/// Dispatch an outgoing request, returns the request commitment
	pub fn dispatch_request(request: Request, meta: FeeMetadata<T>) -> Result<H256, ismp::Error> {
		let commitment = hash_request::<Pallet<T>>(&request);

		if RequestCommitments::<T>::contains_key(commitment) {
			Err(ismp::Error::Custom("Duplicate request".to_string()))?
		}

		let (dest_chain, source_chain, nonce) =
			(request.dest_chain(), request.source_chain(), request.nonce());
		let leaf_index_and_pos = T::OffchainDB::push(Leaf::Request(request));
		// Deposit Event
		Pallet::<T>::deposit_event(Event::Request {
			request_nonce: nonce,
			source_chain,
			dest_chain,
			commitment,
		});

		RequestCommitments::<T>::insert(
			commitment,
			RequestMetadata {
				offchain: LeafIndexAndPos {
					leaf_index: leaf_index_and_pos.index,
					pos: leaf_index_and_pos.position,
				},
				fee: meta,
				claimed: false,
			},
		);

		Ok(commitment)
	}

	/// Dispatch an outgoing response, returns the response commitment
	pub fn dispatch_response(
		response: Response,
		meta: FeeMetadata<T>,
	) -> Result<H256, ismp::Error> {
		let req_commitment = hash_request::<Pallet<T>>(&response.request());

		if Responded::<T>::contains_key(req_commitment) {
			Err(ismp::Error::Custom("Request has been responded to".to_string()))?
		}

		let commitment = hash_response::<Pallet<T>>(&response);

		let (dest_chain, source_chain, nonce) =
			(response.dest_chain(), response.source_chain(), response.nonce());

		let leaf_index_and_pos = T::OffchainDB::push(Leaf::Response(response));

		Pallet::<T>::deposit_event(Event::Response {
			request_nonce: nonce,
			dest_chain,
			source_chain,
			commitment,
			req_commitment,
		});
		ResponseCommitments::<T>::insert(
			commitment,
			RequestMetadata {
				offchain: LeafIndexAndPos {
					leaf_index: leaf_index_and_pos.index,
					pos: leaf_index_and_pos.position,
				},
				fee: meta,
				claimed: false,
			},
		);
		Responded::<T>::insert(req_commitment, true);
		Ok(commitment)
	}

	/// Gets the request from the offchain storage
	pub fn request(commitment: H256) -> Option<Request> {
		let pos = RequestCommitments::<T>::get(commitment)?.offchain.pos;
		match T::OffchainDB::leaf(pos) {
			Ok(Some(Leaf::Request(req))) => Some(req),
			_ => {
				let key = offchain::leaf_default_key(commitment);
				let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key)
				else {
					None?
				};
				match Leaf::decode(&mut &*elem).ok() {
					Some(Leaf::Request(req)) => Some(req),
					_ => None,
				}
			},
		}
	}

	/// Gets the response from the offchain storage
	pub fn response(commitment: H256) -> Option<Response> {
		let pos = ResponseCommitments::<T>::get(commitment)?.offchain.pos;
		match T::OffchainDB::leaf(pos) {
			Ok(Some(Leaf::Response(res))) => Some(res),
			_ => {
				let key = offchain::leaf_default_key(commitment);
				let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key)
				else {
					None?
				};
				match Leaf::decode(&mut &*elem).ok() {
					Some(Leaf::Response(res)) => Some(res),
					_ => None,
				}
			},
		}
	}

	/// Fetch all ISMP handler events in the block, should only be called from runtime-api.
	pub fn block_events() -> Vec<ismp::events::Event>
	where
		<T as frame_system::Config>::RuntimeEvent: TryInto<Event<T>>,
	{
		frame_system::Pallet::<T>::read_events_no_consensus()
			.filter_map(|e| {
				let frame_system::EventRecord { event, .. } = *e;

				let pallet_event: Event<T> = event.try_into().ok()?;
				pallet_event.try_into().ok()
			})
			.collect()
	}

	/// Fetch all ISMP handler events and their extrinsic metadata, should only be called from
	/// runtime-api.
	pub fn block_events_with_metadata() -> Vec<(ismp::events::Event, Option<u32>)>
	where
		<T as frame_system::Config>::RuntimeEvent: TryInto<Event<T>>,
	{
		frame_system::Pallet::<T>::read_events_no_consensus()
			.filter_map(|e| {
				let frame_system::EventRecord { event, phase, .. } = *e;
				let index = match phase {
					Phase::ApplyExtrinsic(index) => Some(index),
					_ => None,
				};
				let pallet_event: Event<T> = event.try_into().ok()?;
				let event = pallet_event.try_into().ok()?;

				Some((event, index))
			})
			.collect()
	}

	/// Fetches the full requests from the offchain for the given commitments.
	pub fn requests(commitments: Vec<H256>) -> Vec<Request> {
		commitments.into_iter().filter_map(|cm| Self::request(cm)).collect()
	}

	/// Fetches the full responses from the offchain for the given commitments.
	pub fn responses(commitments: Vec<H256>) -> Vec<Response> {
		commitments.into_iter().filter_map(|cm| Self::response(cm)).collect()
	}
}

impl<T: Config> ForkIdentifier<T> for Pallet<T> {
	fn identifier() -> <T as frame_system::Config>::Hash {
		Self::child_trie_root()
	}
}
