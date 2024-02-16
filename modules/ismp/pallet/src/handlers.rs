//! Some extra utilities for pallet-ismp

use crate::{
    dispatcher::{FeeMetadata, LeafMetadata},
    host::Host,
    mmr_primitives::Leaf,
    Config, Event, Pallet, RequestCommitments, Responded, ResponseCommitments,
};
use alloc::string::ToString;
use ismp::{
    error::Error as IsmpError,
    router::{Request, Response},
    util::{hash_request, hash_response},
};

impl<T: Config> Pallet<T> {
    /// Dispatch an outgoing request
    pub fn dispatch_request(request: Request, meta: FeeMetadata<T>) -> Result<(), IsmpError> {
        let commitment = hash_request::<Host<T>>(&request);

        if RequestCommitments::<T>::contains_key(commitment) {
            Err(IsmpError::ImplementationSpecific("Duplicate request".to_string()))?
        }

        let (dest_chain, source_chain, nonce) =
            (request.dest_chain(), request.source_chain(), request.nonce());
        let leaf_index_and_pos =
            Pallet::<T>::mmr_push(Leaf::Request(request)).ok_or_else(|| {
                IsmpError::ImplementationSpecific("Failed to push request into mmr".to_string())
            })?;
        // Deposit Event
        Pallet::<T>::deposit_event(Event::Request {
            request_nonce: nonce,
            source_chain,
            dest_chain,
            commitment,
        });

        RequestCommitments::<T>::insert(commitment, LeafMetadata { mmr: leaf_index_and_pos, meta });
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
            Pallet::<T>::mmr_push(Leaf::Response(response)).ok_or_else(|| {
                IsmpError::ImplementationSpecific("Failed to push response into mmr".to_string())
            })?;

        Pallet::<T>::deposit_event(Event::Response {
            request_nonce: nonce,
            dest_chain,
            source_chain,
            commitment,
        });
        ResponseCommitments::<T>::insert(
            commitment,
            LeafMetadata { mmr: leaf_index_and_pos, meta },
        );
        Responded::<T>::insert(req_commitment, true);
        Ok(())
    }
}
