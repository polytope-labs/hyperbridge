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
// See the License for the specific language governing permissions and
// limitations under the License.

//! Pallet methods

use crate::{
    child_trie::{RequestCommitments, ResponseCommitments},
    dispatcher::{FeeMetadata, RequestMetadata},
    events,
    mmr::{Leaf, LeafIndexAndPos, Proof, ProofKeys},
    weights::get_weight,
    Config, Error, Event, Pallet, Responded,
};
use alloc::{string::ToString, vec, vec::Vec};
use frame_support::dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo};
use frame_system::Phase;
use ismp::{
    handlers::{handle_incoming_message, MessageResult},
    messaging::{hash_request, hash_response, Message},
    router::{Request, Response},
};
use log::debug;
use mmr_primitives::{ForkIdentifier, MerkleMountainRangeTree};
use sp_core::H256;

impl<T: Config> Pallet<T> {
    /// Deposit a pallet [`Event<T>`]
    pub fn deposit_pallet_event<E: Into<Event<T>>>(event: E) {
        Self::deposit_event(event.into())
    }

    /// Generate an MMR proof for the given `leaf_indices`.
    /// Note this method can only be used from an off-chain context
    /// (Offchain Worker or Runtime API call), since it requires
    /// all the leaves to be present.
    /// It may return an error or panic if used incorrectly.
    pub fn generate_proof(
        keys: ProofKeys,
    ) -> Result<(Vec<Leaf>, Proof<H256>), sp_mmr_primitives::Error> {
        let leaf_indices_and_positions = match keys {
            ProofKeys::Requests(commitments) => commitments
                .into_iter()
                .map(|commitment| {
                    let val = RequestCommitments::<T>::get(commitment)
                        .ok_or_else(|| sp_mmr_primitives::Error::LeafNotFound)?
                        .mmr;
                    Ok(val)
                })
                .collect::<Result<Vec<_>, _>>()?,
            ProofKeys::Responses(commitments) => commitments
                .into_iter()
                .map(|commitment| {
                    let val = ResponseCommitments::<T>::get(commitment)
                        .ok_or_else(|| sp_mmr_primitives::Error::LeafNotFound)?
                        .mmr;
                    Ok(val)
                })
                .collect::<Result<Vec<_>, _>>()?,
        };
        let indices =
            leaf_indices_and_positions.iter().map(|val| val.leaf_index).collect::<Vec<_>>();
        let (leaves, proof) = T::Mmr::generate_proof(indices)?;
        let proof = Proof {
            leaf_indices_and_pos: leaf_indices_and_positions,
            leaf_count: proof.leaf_count,
            items: proof.items,
        };

        Ok((leaves, proof))
    }

    /// Provides a way to handle messages.
    pub fn handle_messages(messages: Vec<Message>) -> DispatchResultWithPostInfo {
        // Define a host
        let host = Pallet::<T>::default();
        let events = messages
            .iter()
            .map(|msg| handle_incoming_message(&host, msg.clone()))
            .collect::<Result<Vec<_>, _>>()
            .and_then(|result| {
                result
                    .into_iter()
                    // check that requests will be successfully dispatched
                    // so we can not be spammed with failing txs
                    .map(|result| match result {
                        MessageResult::Request(results) |
                        MessageResult::Response(results) |
                        MessageResult::Timeout(results) => results,
                        MessageResult::ConsensusMessage(events) =>
                            events.into_iter().map(Ok).collect(),
                        MessageResult::FrozenClient(_) => {
                            vec![]
                        },
                    })
                    .flatten()
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|err| {
                debug!(target: "ismp", "Handling Error {:?}", err);
                Pallet::<T>::deposit_event(Event::<T>::Errors { errors: vec![err.into()] });
                Error::<T>::InvalidMessage
            })?;

        for event in events {
            // deposit any relevant events
            Pallet::<T>::deposit_event(event.into())
        }

        Ok(PostDispatchInfo {
            actual_weight: Some(get_weight::<T>(&messages)),
            pays_fee: Pays::Yes,
        })
    }

    /// Dispatch an outgoing request, returns the request commitment
    pub fn dispatch_request(request: Request, meta: FeeMetadata<T>) -> Result<H256, ismp::Error> {
        let commitment = hash_request::<Pallet<T>>(&request);

        if RequestCommitments::<T>::contains_key(commitment) {
            Err(ismp::Error::Custom("Duplicate request".to_string()))?
        }

        let (dest_chain, source_chain, nonce) =
            (request.dest_chain(), request.source_chain(), request.nonce());
        let leaf_index_and_pos = T::Mmr::push(Leaf::Request(request));
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
                mmr: LeafIndexAndPos {
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

        let leaf_index_and_pos = T::Mmr::push(Leaf::Response(response));

        Pallet::<T>::deposit_event(Event::Response {
            request_nonce: nonce,
            dest_chain,
            source_chain,
            commitment,
        });
        ResponseCommitments::<T>::insert(
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
        Responded::<T>::insert(req_commitment, true);
        Ok(commitment)
    }

    /// Gets the request from the offchain storage
    pub fn request(commitment: H256) -> Option<Request> {
        let pos = RequestCommitments::<T>::get(commitment)?.mmr.pos;
        let Ok(Some(Leaf::Request(req))) = T::Mmr::get_leaf(pos) else { None? };
        Some(req)
    }

    /// Gets the response from the offchain storage
    pub fn response(commitment: H256) -> Option<Response> {
        let pos = ResponseCommitments::<T>::get(commitment)?.mmr.pos;
        let Ok(Some(Leaf::Response(res))) = T::Mmr::get_leaf(pos) else { None? };
        Some(res)
    }

    /// Fetch all ISMP handler events in the block, should only be called from runtime-api.
    pub fn block_events() -> Vec<events::Event>
    where
        <T as frame_system::Config>::RuntimeEvent: TryInto<Event<T>>,
    {
        frame_system::Pallet::<T>::read_events_no_consensus()
            .filter_map(|e| {
                let frame_system::EventRecord { event, .. } = *e;

                events::to_handler_events::<T>(event.try_into().ok()?)
            })
            .collect()
    }

    /// Fetch all ISMP handler events and their extrinsic metadata, should only be called from
    /// runtime-api.
    pub fn block_events_with_metadata() -> Vec<(events::Event, u32)>
    where
        <T as frame_system::Config>::RuntimeEvent: TryInto<Event<T>>,
    {
        frame_system::Pallet::<T>::read_events_no_consensus()
            .filter_map(|e| {
                let frame_system::EventRecord { event, phase, .. } = *e;
                let Phase::ApplyExtrinsic(index) = phase else { None? };
                let event = events::to_handler_events::<T>(event.try_into().ok()?)?;

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
