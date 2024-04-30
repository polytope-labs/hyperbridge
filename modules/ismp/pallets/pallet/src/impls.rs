// Copyright (C) 2023 Polytope Labs.
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
    dispatcher::{FeeMetadata, LeafMetadata},
    host::Host,
    mmr::{Leaf, LeafIndexAndPos, Proof, ProofKeys},
    weights::get_weight,
    ChallengePeriod, Config, ConsensusClientUpdateTime, ConsensusStates, Error, Event,
    LatestStateMachineHeight, Pallet, Responded,
};
use alloc::{string::ToString, vec, vec::Vec};
use frame_support::dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo};
use ismp::{
    consensus::{ConsensusClientId, StateMachineId},
    handlers::{handle_incoming_message, MessageResult},
    messaging::Message,
    router::{Request, Response},
    util::{hash_request, hash_response},
};
use log::debug;
use mmr_primitives::MerkleMountainRangeTree;
use sp_core::H256;

impl<T: Config> Pallet<T> {
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
        let host = Host::<T>::default();
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

    /// Dispatch an outgoing request
    pub fn dispatch_request(request: Request, meta: FeeMetadata<T>) -> Result<(), ismp::Error> {
        let commitment = hash_request::<Host<T>>(&request);

        if RequestCommitments::<T>::contains_key(commitment) {
            Err(ismp::Error::ImplementationSpecific("Duplicate request".to_string()))?
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
            LeafMetadata {
                mmr: LeafIndexAndPos {
                    leaf_index: leaf_index_and_pos.index,
                    pos: leaf_index_and_pos.position,
                },
                meta,
            },
        );

        Ok(())
    }

    /// Dispatch an outgoing response
    pub fn dispatch_response(response: Response, meta: FeeMetadata<T>) -> Result<(), ismp::Error> {
        let req_commitment = hash_request::<Host<T>>(&response.request());

        if Responded::<T>::contains_key(req_commitment) {
            Err(ismp::Error::ImplementationSpecific("Request has been responded to".to_string()))?
        }

        let commitment = hash_response::<Host<T>>(&response);

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
            LeafMetadata {
                mmr: LeafIndexAndPos {
                    leaf_index: leaf_index_and_pos.index,
                    pos: leaf_index_and_pos.position,
                },
                meta,
            },
        );
        Responded::<T>::insert(req_commitment, true);
        Ok(())
    }

    /// Gets the request from the offchain storage
    pub fn get_request(commitment: H256) -> Option<Request> {
        let pos = RequestCommitments::<T>::get(commitment)?.mmr.pos;
        let Ok(Some(Leaf::Request(req))) = T::Mmr::get_leaf(pos) else { None? };
        Some(req)
    }

    /// Gets the response from the offchain storage
    pub fn get_response(commitment: H256) -> Option<Response> {
        let pos = ResponseCommitments::<T>::get(commitment)?.mmr.pos;
        let Ok(Some(Leaf::Response(res))) = T::Mmr::get_leaf(pos) else { None? };
        Some(res)
    }

    /// Return the scale encoded consensus state
    pub fn get_consensus_state(id: ConsensusClientId) -> Option<Vec<u8>> {
        ConsensusStates::<T>::get(id)
    }

    /// Return the timestamp this client was last updated in seconds
    pub fn get_consensus_update_time(id: ConsensusClientId) -> Option<u64> {
        ConsensusClientUpdateTime::<T>::get(id)
    }

    /// Return the challenge period
    pub fn get_challenge_period(id: ConsensusClientId) -> Option<u64> {
        ChallengePeriod::<T>::get(id)
    }

    /// Return the latest height of the state machine
    pub fn get_latest_state_machine_height(id: StateMachineId) -> Option<u64> {
        Some(LatestStateMachineHeight::<T>::get(id))
    }

    /// Get actual requests
    pub fn get_requests(commitments: Vec<H256>) -> Vec<Request> {
        commitments.into_iter().filter_map(|cm| Self::get_request(cm)).collect()
    }

    /// Get actual requests
    pub fn get_responses(commitments: Vec<H256>) -> Vec<Response> {
        commitments.into_iter().filter_map(|cm| Self::get_response(cm)).collect()
    }
}
