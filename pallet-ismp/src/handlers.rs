//! Some extra utilities for pallet-ismp

use crate::{
    dispatcher::Receipt, host::Host, Config, Event, IncomingRequestAcks, IncomingResponseAcks,
    Pallet,
};
use alloc::string::ToString;
use ismp_primitives::mmr::Leaf;
use ismp_rs::{
    router::{DispatchError, DispatchResult, DispatchSuccess, Request, Response},
    util::{hash_request, hash_response},
};
use sp_core::H256;

impl<T: Config> Pallet<T>
where
    <T as frame_system::Config>::Hash: From<H256>,
{
    /// Handle an incoming request
    pub fn handle_request(&self, request: Request) -> DispatchResult {
        let commitment = hash_request::<Host<T>>(&request).0.to_vec();

        if IncomingRequestAcks::<T>::contains_key(commitment.clone()) {
            Err(DispatchError {
                msg: "Duplicate request".to_string(),
                nonce: request.nonce(),
                source: request.source_chain(),
                dest: request.dest_chain(),
            })?
        }

        let (dest_chain, source_chain, nonce) =
            (request.dest_chain(), request.source_chain(), request.nonce());
        Pallet::<T>::mmr_push(Leaf::Request(request)).ok_or_else(|| DispatchError {
            msg: "Failed to push request into mmr".to_string(),
            nonce,
            source: source_chain,
            dest: dest_chain,
        })?;
        // Deposit Event
        Pallet::<T>::deposit_event(Event::Request {
            request_nonce: nonce,
            source_chain,
            dest_chain,
        });

        IncomingRequestAcks::<T>::insert(commitment, Receipt::Ok);
        Ok(DispatchSuccess { dest_chain, source_chain, nonce })
    }

    /// Handle an incoming response
    pub fn handle_response(&self, response: Response) -> DispatchResult {
        let commitment = hash_response::<Host<T>>(&response).0.to_vec();

        if IncomingResponseAcks::<T>::contains_key(commitment.clone()) {
            Err(DispatchError {
                msg: "Duplicate response".to_string(),
                nonce: response.nonce(),
                source: response.source_chain(),
                dest: response.dest_chain(),
            })?
        }

        let (dest_chain, source_chain, nonce) =
            (response.dest_chain(), response.source_chain(), response.nonce());

        Pallet::<T>::mmr_push(Leaf::Response(response)).ok_or_else(|| DispatchError {
            msg: "Failed to push response into mmr".to_string(),
            nonce,
            source: source_chain,
            dest: dest_chain,
        })?;

        Pallet::<T>::deposit_event(Event::Response {
            request_nonce: nonce,
            dest_chain,
            source_chain,
        });
        IncomingResponseAcks::<T>::insert(commitment, Receipt::Ok);
        Ok(DispatchSuccess { dest_chain, source_chain, nonce })
    }
}
