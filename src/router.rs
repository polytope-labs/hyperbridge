use crate::host::Host;
use crate::mmr::{self, Leaf, Mmr};
use crate::{Config, Event, Pallet};
use core::marker::PhantomData;
use ismp_rust::error::Error;
use ismp_rust::host::ISMPHost;
use ismp_rust::router::{IISMPRouter, Request, Response};

#[derive(Default, Clone)]
pub struct Router<T: Config>(PhantomData<T>);

impl<T: Config> IISMPRouter for Router<T> {
    fn dispatch(&self, request: Request) -> Result<(), Error> {
        let host = Host::<T>::default();
        if host.host() != request.dest_chain {
            let leaves = Pallet::<T>::number_of_leaves();
            let (dest_chain, source_chain, nonce) =
                (request.dest_chain, request.source_chain, request.nonce);
            let mut mmr: Mmr<mmr::storage::RuntimeStorage, T, Leaf> = mmr::Mmr::new(leaves);
            let offchain_key = Pallet::<T>::request_leaf_index_offchain_key(dest_chain, nonce);
            let leaf_index = mmr.push(Leaf::Request(request)).ok_or_else(|| {
                Error::RequestVerificationFailed {
                    nonce,
                    source: source_chain,
                    dest: dest_chain,
                }
            })?;
            // Deposit Event
            Pallet::<T>::deposit_event(Event::RequestReceived {
                request_nonce: nonce,
            });
            // Store a map of request to leaf_index
            Pallet::<T>::store_leaf_index_offchain(offchain_key, leaf_index)
        }

        Ok(())
    }

    fn write_response(&self, response: Response) -> Result<(), Error> {
        let host = Host::<T>::default();
        if host.host() != response.request.source_chain {
            let leaves = Pallet::<T>::number_of_leaves();
            let (dest_chain, source_chain, nonce) = (
                response.request.dest_chain,
                response.request.source_chain,
                response.request.nonce,
            );
            let mut mmr: Mmr<mmr::storage::RuntimeStorage, T, Leaf> = mmr::Mmr::new(leaves);
            let offchain_key = Pallet::<T>::response_leaf_index_offchain_key(source_chain, nonce);
            let leaf_index = mmr.push(Leaf::Response(response)).ok_or_else(|| {
                Error::ResponseVerificationFailed {
                    nonce,
                    source: source_chain,
                    dest: dest_chain,
                }
            })?;
            Pallet::<T>::deposit_event(Event::ResponseReceived {
                request_nonce: nonce,
            });
            Pallet::<T>::store_leaf_index_offchain(offchain_key, leaf_index)
        }

        Ok(())
    }
}
