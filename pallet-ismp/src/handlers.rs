//! Some extra utilities for pallet-ismp

use crate::{
    dispatcher::Receipt, host::Host, Config, Event, Pallet, RequestCommitments, ResponseCommitments,
};
use alloc::string::ToString;
use ismp_primitives::{mmr::Leaf, LeafIndexQuery};
use ismp_rs::{
    error::Error as IsmpError,
    router::{Request, Response},
    util::{hash_request, hash_response},
};
use sp_core::H256;

impl<T: Config> Pallet<T>
where
    <T as frame_system::Config>::Hash: From<H256>,
{
    /// Dispatch an outgoing request
    pub fn dispatch_request(request: Request) -> Result<(), IsmpError> {
        let commitment = hash_request::<Host<T>>(&request);

        if RequestCommitments::<T>::contains_key(commitment) {
            Err(IsmpError::ImplementationSpecific("Duplicate request".to_string()))?
        }

        let (dest_chain, source_chain, nonce) =
            (request.dest_chain(), request.source_chain(), request.nonce());
        Pallet::<T>::mmr_push(Leaf::Request(request)).ok_or_else(|| {
            IsmpError::ImplementationSpecific("Failed to push request into mmr".to_string())
        })?;
        // Deposit Event
        Pallet::<T>::deposit_event(Event::Request {
            request_nonce: nonce,
            source_chain,
            dest_chain,
        });

        RequestCommitments::<T>::insert(
            commitment,
            LeafIndexQuery { source_chain, dest_chain, nonce },
        );
        Ok(())
    }

    /// Dispatch an outgoing response
    pub fn dispatch_response(response: Response) -> Result<(), IsmpError> {
        let commitment = hash_request::<Host<T>>(&response.request());

        if !RequestCommitments::<T>::contains_key(commitment) {
            Err(IsmpError::ImplementationSpecific("Unknown request for response".to_string()))?
        }

        let commitment = hash_response::<Host<T>>(&response);

        if ResponseCommitments::<T>::contains_key(commitment) {
            Err(IsmpError::ImplementationSpecific("Duplicate response".to_string()))?
        }

        let (dest_chain, source_chain, nonce) =
            (response.dest_chain(), response.source_chain(), response.nonce());

        Pallet::<T>::mmr_push(Leaf::Response(response)).ok_or_else(|| {
            IsmpError::ImplementationSpecific("Failed to push response into mmr".to_string())
        })?;

        Pallet::<T>::deposit_event(Event::Response {
            request_nonce: nonce,
            dest_chain,
            source_chain,
        });
        ResponseCommitments::<T>::insert(commitment, Receipt::Ok);
        Ok(())
    }
}
