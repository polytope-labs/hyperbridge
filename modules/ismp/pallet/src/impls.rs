use crate::{
    child_trie::{RequestCommitments, ResponseCommitments},
    dispatcher::{FeeMetadata, LeafMetadata},
    errors::HandlingError,
    events::deposit_ismp_events,
    host::Host,
    mmr::Leaf,
    primitives::{LeafIndexAndPos, Proof},
    weight_info::get_weight,
    ChallengePeriod, Config, ConsensusClientUpdateTime, ConsensusStates, Event,
    LatestStateMachineHeight, Pallet, ProofKeys, Responded, WeightConsumed,
};
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
            leaf_positions: leaf_indices_and_positions,
            leaf_count: proof.leaf_count,
            items: proof.items,
        };

        Ok((leaves.into_iter().map(|leaf| leaf.into()).collect(), proof))
    }

    /// Provides a way to handle messages.
    pub fn handle_messages(messages: Vec<Message>) -> DispatchResultWithPostInfo {
        // Define a host
        WeightConsumed::<T>::kill();
        let host = Host::<T>::default();
        let mut errors: Vec<HandlingError> = vec![];
        let total_weight = get_weight::<T>(&messages);
        for message in messages {
            match handle_incoming_message(&host, message.clone()) {
                Ok(MessageResult::ConsensusMessage(res)) => deposit_ismp_events::<T>(
                    res.into_iter().map(|ev| Ok(ev)).collect(),
                    &mut errors,
                ),
                Ok(MessageResult::Response(res)) => deposit_ismp_events::<T>(res, &mut errors),
                Ok(MessageResult::Request(res)) => deposit_ismp_events::<T>(res, &mut errors),
                Ok(MessageResult::Timeout(res)) => deposit_ismp_events::<T>(res, &mut errors),
                Ok(MessageResult::FrozenClient(id)) =>
                    Self::deposit_event(Event::<T>::ConsensusClientFrozen {
                        consensus_client_id: id,
                    }),
                Err(err) => {
                    errors.push(err.into());
                },
            }
        }

        if !errors.is_empty() {
            debug!(target: "ismp", "Handling Errors {:?}", errors);
            Self::deposit_event(Event::<T>::Errors { errors })
        }

        Ok(PostDispatchInfo {
            actual_weight: {
                let acc_weight = WeightConsumed::<T>::get();
                Some((total_weight - acc_weight.weight_limit) + acc_weight.weight_used)
            },
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
