use crate::{
    host::Host,
    mmr::{self, Leaf, Mmr},
    Config, Event, Pallet, RequestAcks, ResponseAcks,
};
use alloc::{format, string::ToString};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use ismp_rs::{
    error::Error,
    host::ISMPHost,
    router::{ISMPRouter, Request, Response},
};

#[derive(Encode, Decode, scale_info::TypeInfo)]
pub enum Receipt {
    Ok,
}

#[derive(Clone)]
pub struct Router<T: Config>(PhantomData<T>);

impl<T: Config> Default for Router<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Config> ISMPRouter for Router<T> {
    fn dispatch(&self, request: Request) -> Result<(), Error> {
        let host = Host::<T>::default();

        let commitment = host.get_request_commitment(&request);

        if RequestAcks::<T>::contains_key(commitment.clone()) {
            return Err(Error::ImplementationSpecific(format!(
                "Duplicate request: nonce: {} , source: {:?} , dest: {:?}",
                request.nonce(),
                request.source_chain(),
                request.dest_chain()
            )))
        }

        if host.host() != request.dest_chain() {
            let leaves = Pallet::<T>::number_of_leaves();
            let (dest_chain, source_chain, nonce) =
                (request.dest_chain(), request.source_chain(), request.nonce());
            let mut mmr: Mmr<mmr::storage::RuntimeStorage, T, Leaf> = mmr::Mmr::new(leaves);
            let offchain_key =
                Pallet::<T>::request_leaf_index_offchain_key(source_chain, dest_chain, nonce);
            let leaf_index = mmr.push(Leaf::Request(request)).ok_or_else(|| {
                Error::ImplementationSpecific("Failed to push request into mmr".to_string())
            })?;
            // Deposit Event
            Pallet::<T>::deposit_event(Event::Request {
                request_nonce: nonce,
                source_chain,
                dest_chain,
            });
            // Store a map of request to leaf_index
            Pallet::<T>::store_leaf_index_offchain(offchain_key, leaf_index)
        }

        RequestAcks::<T>::insert(commitment, Receipt::Ok);
        Ok(())
    }

    fn write_response(&self, response: Response) -> Result<(), Error> {
        let host = Host::<T>::default();

        let commitment = host.get_response_commitment(&response);

        if ResponseAcks::<T>::contains_key(commitment.clone()) {
            return Err(Error::ImplementationSpecific(format!(
                "Duplicate response: nonce: {} , source: {:?} , dest: {:?}",
                response.request.nonce(),
                response.request.source_chain(),
                response.request.dest_chain()
            )))
        }

        if host.host() != response.request.source_chain() {
            let leaves = Pallet::<T>::number_of_leaves();
            let (dest_chain, source_chain, nonce) = (
                response.request.source_chain(),
                response.request.dest_chain(),
                response.request.nonce(),
            );
            let mut mmr: Mmr<mmr::storage::RuntimeStorage, T, Leaf> = mmr::Mmr::new(leaves);
            let offchain_key =
                Pallet::<T>::response_leaf_index_offchain_key(source_chain, dest_chain, nonce);
            let leaf_index = mmr.push(Leaf::Response(response)).ok_or_else(|| {
                Error::ImplementationSpecific("Failed to push response into mmr".to_string())
            })?;
            Pallet::<T>::deposit_event(Event::Response {
                request_nonce: nonce,
                dest_chain,
                source_chain,
            });
            Pallet::<T>::store_leaf_index_offchain(offchain_key, leaf_index)
        }

        ResponseAcks::<T>::insert(commitment, Receipt::Ok);

        Ok(())
    }
}
