use crate::{host::Host, mmr, mmr::mmr::Mmr, Config, Event, Pallet, RequestAcks, ResponseAcks};
use alloc::{format, string::ToString};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use ismp_primitives::mmr::Leaf;
use ismp_rs::{
    error::Error,
    host::ISMPHost,
    router::{ISMPRouter, Request, Response},
    util::{hash_request, hash_response},
};
use sp_core::H256;

#[derive(Encode, Decode, scale_info::TypeInfo)]
pub enum Receipt {
    Ok,
}

/// The proxy router, This router allows for routing requests & responses from a source chain
/// to a destination chain.
#[derive(Clone)]
pub struct ProxyRouter<T, R> {
    inner: Option<R>,
    _phantom: PhantomData<T>,
}

impl<T, R> ProxyRouter<T, R> {
    /// Initialize the proxy router with an inner router.
    pub fn new(router: R) -> Self {
        Self { inner: Some(router), _phantom: PhantomData }
    }
}

impl<T, R> Default for ProxyRouter<T, R> {
    fn default() -> Self {
        Self { inner: None, _phantom: PhantomData }
    }
}

impl<T, R> ISMPRouter for ProxyRouter<T, R>
where
    T: Config,
    R: ISMPRouter,
    <T as frame_system::Config>::Hash: From<H256>,
{
    fn dispatch(&self, request: Request) -> Result<(), Error> {
        let host = Host::<T>::default();

        if host.host() != request.dest_chain() {
            let commitment = hash_request::<Host<T>>(&request).0.to_vec();

            if RequestAcks::<T>::contains_key(commitment.clone()) {
                return Err(Error::ImplementationSpecific(format!(
                    "Duplicate request: nonce: {} , source: {:?} , dest: {:?}",
                    request.nonce(),
                    request.source_chain(),
                    request.dest_chain()
                )))
            }

            let leaves = Pallet::<T>::number_of_leaves();
            let (dest_chain, source_chain, nonce) =
                (request.dest_chain(), request.source_chain(), request.nonce());
            let mut mmr: Mmr<mmr::storage::RuntimeStorage, T> = Mmr::new(leaves);
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
            Pallet::<T>::store_leaf_index_offchain(offchain_key, leaf_index);
            RequestAcks::<T>::insert(commitment, Receipt::Ok);
        } else if let Some(ref router) = self.inner {
            router.dispatch(request)?
        }

        Ok(())
    }

    fn dispatch_timeout(&self, _request: Request) -> Result<(), Error> {
        todo!()
    }

    fn write_response(&self, response: Response) -> Result<(), Error> {
        let host = Host::<T>::default();

        if host.host() != response.request.source_chain() {
            let commitment = hash_response::<Host<T>>(&response).0.to_vec();

            if ResponseAcks::<T>::contains_key(commitment.clone()) {
                return Err(Error::ImplementationSpecific(format!(
                    "Duplicate response: nonce: {} , source: {:?} , dest: {:?}",
                    response.request.nonce(),
                    response.request.source_chain(),
                    response.request.dest_chain()
                )))
            }

            let leaves = Pallet::<T>::number_of_leaves();
            let (dest_chain, source_chain, nonce) = (
                response.request.source_chain(),
                response.request.dest_chain(),
                response.request.nonce(),
            );
            let mut mmr: Mmr<mmr::storage::RuntimeStorage, T> = Mmr::new(leaves);
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
            Pallet::<T>::store_leaf_index_offchain(offchain_key, leaf_index);
            ResponseAcks::<T>::insert(commitment, Receipt::Ok);
        } else if let Some(ref router) = self.inner {
            router.write_response(response)?
        }

        Ok(())
    }
}
