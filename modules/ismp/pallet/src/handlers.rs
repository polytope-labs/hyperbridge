//! Some extra utilities for pallet-ismp

use crate::{
    child_trie::{RequestCommitments, ResponseCommitments},
    dispatcher::{FeeMetadata, LeafMetadata},
    host::Host,
    mmr::Leaf,
    primitives::LeafIndexAndPos,
    Config, Event, Pallet, Responded,
};
use alloc::string::ToString;
use ismp::{
    error::Error as IsmpError,
    router::{Request, Response},
    util::{hash_request, hash_response},
};
use pallet_mmr_labs::MerkleMountainRangeTree;

impl<T: Config> Pallet<T>
where
    <T as pallet_mmr_labs::Config>::Leaf: From<Leaf>,
{
    /// Dispatch an outgoing request
    pub fn dispatch_request(request: Request, meta: FeeMetadata<T>) -> Result<(), IsmpError> {
        let commitment = hash_request::<Host<T>>(&request);

        if RequestCommitments::<T>::contains_key(commitment) {
            Err(IsmpError::ImplementationSpecific("Duplicate request".to_string()))?
        }

        let (dest_chain, source_chain, nonce) =
            (request.dest_chain(), request.source_chain(), request.nonce());
        let leaf_index_and_pos = pallet_mmr_labs::Pallet::<T>::push(Leaf::Request(request).into());
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
    pub fn dispatch_response(response: Response, meta: FeeMetadata<T>) -> Result<(), IsmpError> {
        let req_commitment = hash_request::<Host<T>>(&response.request());

        if Responded::<T>::contains_key(req_commitment) {
            Err(IsmpError::ImplementationSpecific("Request has been responded to".to_string()))?
        }

        let commitment = hash_response::<Host<T>>(&response);

        let (dest_chain, source_chain, nonce) =
            (response.dest_chain(), response.source_chain(), response.nonce());

        let leaf_index_and_pos =
            pallet_mmr_labs::Pallet::<T>::push(Leaf::Response(response).into());

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
}
