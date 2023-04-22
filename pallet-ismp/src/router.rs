use crate::{host::Host, Config, Event, Pallet, RequestAcks, ResponseAcks};
use alloc::{boxed::Box, string::ToString};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use ismp_primitives::mmr::Leaf;
use ismp_rs::{
    host::ISMPHost,
    router::{DispatchError, DispatchResult, DispatchSuccess, ISMPRouter, Request, Response},
    util::{hash_request, hash_response},
};
use sp_core::H256;

#[derive(Encode, Decode, scale_info::TypeInfo)]
pub enum Receipt {
    Ok,
}

/// The proxy router, This router allows for routing requests & responses from a source chain
/// to a destination chain.
pub struct ProxyRouter<T> {
    inner: Option<Box<dyn ISMPRouter>>,
    _phantom: PhantomData<T>,
}

impl<T> ProxyRouter<T> {
    /// Initialize the proxy router with an inner router.
    pub fn new<R>(router: R) -> Self
    where
        R: ISMPRouter + 'static,
    {
        Self { inner: Some(Box::new(router)), _phantom: PhantomData }
    }
}

impl<T> Default for ProxyRouter<T> {
    fn default() -> Self {
        Self { inner: None, _phantom: PhantomData }
    }
}

impl<T> ISMPRouter for ProxyRouter<T>
where
    T: Config,
    <T as frame_system::Config>::Hash: From<H256>,
{
    fn dispatch(&self, request: Request) -> DispatchResult {
        let host = Host::<T>::default();

        if host.host_state_machine() != request.dest_chain() {
            let commitment = hash_request::<Host<T>>(&request).0.to_vec();

            if RequestAcks::<T>::contains_key(commitment.clone()) {
                Err(DispatchError {
                    msg: "Duplicate request".to_string(),
                    nonce: request.nonce(),
                    source: request.source_chain(),
                    dest: request.dest_chain(),
                })?
            }

            let (dest_chain, source_chain, nonce) =
                (request.dest_chain(), request.source_chain(), request.nonce());
            let offchain_key =
                Pallet::<T>::request_leaf_index_offchain_key(source_chain, dest_chain, nonce);
            let leaf_index = if let Some(leaf_index) = Pallet::<T>::mmr_push(Leaf::Request(request))
            {
                leaf_index
            } else {
                Err(DispatchError {
                    msg: "Failed to push request into mmr".to_string(),
                    nonce,
                    source: source_chain,
                    dest: dest_chain,
                })?
            };
            // Deposit Event
            Pallet::<T>::deposit_event(Event::Request {
                request_nonce: nonce,
                source_chain,
                dest_chain,
            });
            // Store a map of request to leaf_index
            Pallet::<T>::store_leaf_index_offchain(offchain_key, leaf_index);
            RequestAcks::<T>::insert(commitment, Receipt::Ok);
            Ok(DispatchSuccess { dest_chain, source_chain, nonce })
        } else if let Some(ref router) = self.inner {
            router.dispatch(request)
        } else {
            Err(DispatchError {
                msg: "Missing a module router".to_string(),
                nonce: request.nonce(),
                source: request.source_chain(),
                dest: request.dest_chain(),
            })?
        }
    }

    fn dispatch_timeout(&self, request: Request) -> DispatchResult {
        if let Some(ref router) = self.inner {
            router.dispatch(request)
        } else {
            Err(DispatchError {
                msg: "Missing a module router".to_string(),
                nonce: request.nonce(),
                source: request.source_chain(),
                dest: request.dest_chain(),
            })?
        }
    }

    fn write_response(&self, response: Response) -> DispatchResult {
        let host = Host::<T>::default();

        if host.host_state_machine() != response.request.source_chain() {
            let commitment = hash_response::<Host<T>>(&response).0.to_vec();

            if ResponseAcks::<T>::contains_key(commitment.clone()) {
                Err(DispatchError {
                    msg: "Duplicate response".to_string(),
                    nonce: response.request.nonce(),
                    source: response.request.source_chain(),
                    dest: response.request.dest_chain(),
                })?
            }

            let (dest_chain, source_chain, nonce) = (
                response.request.source_chain(),
                response.request.dest_chain(),
                response.request.nonce(),
            );

            let offchain_key =
                Pallet::<T>::response_leaf_index_offchain_key(source_chain, dest_chain, nonce);
            let leaf_index =
                if let Some(leaf_index) = Pallet::<T>::mmr_push(Leaf::Response(response)) {
                    leaf_index
                } else {
                    Err(DispatchError {
                        msg: "Failed to push response into mmr".to_string(),
                        nonce,
                        source: source_chain,
                        dest: dest_chain,
                    })?
                };

            Pallet::<T>::deposit_event(Event::Response {
                request_nonce: nonce,
                dest_chain,
                source_chain,
            });
            Pallet::<T>::store_leaf_index_offchain(offchain_key, leaf_index);
            ResponseAcks::<T>::insert(commitment, Receipt::Ok);
            Ok(DispatchSuccess { dest_chain, source_chain, nonce })
        } else if let Some(ref router) = self.inner {
            router.write_response(response)
        } else {
            Err(DispatchError {
                msg: "Missing a module router".to_string(),
                nonce: response.request.nonce(),
                source: response.request.source_chain(),
                dest: response.request.dest_chain(),
            })?
        }
    }
}
