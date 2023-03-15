use crate::host::Host;
use crate::mmr::{self, Leaf, Mmr};
use crate::{
    Config, Pallet, RequestOffchainKey, RequestsStore, ResponseOffchainKey, ResponseStore,
};
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
            let request_leaves = Pallet::<T>::number_of_request_leaves();
            let (dest_chain, source_chain, nonce) =
                (request.dest_chain, request.source_chain, request.nonce);
            let mut request_mmr: Mmr<
                mmr::storage::RuntimeStorage,
                T,
                Leaf,
                RequestOffchainKey<T, Leaf>,
                RequestsStore<T>,
            > = mmr::Mmr::new(request_leaves);
            let offchain_key = Pallet::<T>::request_leaf_index_offchain_key(&request);
            let leaf_index = request_mmr.push(Leaf::Request(request)).ok_or_else(|| {
                Error::RequestVerificationFailed {
                    nonce,
                    source: source_chain,
                    dest: dest_chain,
                }
            })?;
            // Deposit Event
            // Store a map of request to leaf_index
            Pallet::<T>::store_leaf_index_offchain(offchain_key, leaf_index)
        }

        Ok(())
    }

    fn write_response(&self, response: Response) -> Result<(), Error> {
        let host = Host::<T>::default();
        if host.host() != response.request.dest_chain {
            let response_leaves = Pallet::<T>::number_of_response_leaves();
            let (dest_chain, source_chain, nonce) = (
                response.request.dest_chain,
                response.request.source_chain,
                response.request.nonce,
            );
            let mut response_mmr: Mmr<
                mmr::storage::RuntimeStorage,
                T,
                Leaf,
                ResponseOffchainKey<T, Leaf>,
                ResponseStore<T>,
            > = mmr::Mmr::new(response_leaves);
            let offchain_key = Pallet::<T>::response_leaf_index_offchain_key(&response);
            let leaf_index = response_mmr.push(Leaf::Response(response)).ok_or_else(|| {
                Error::RequestVerificationFailed {
                    nonce,
                    source: source_chain,
                    dest: dest_chain,
                }
            })?;
            // Deposit Event
            Pallet::<T>::store_leaf_index_offchain(offchain_key, leaf_index)
        }

        Ok(())
    }
}
