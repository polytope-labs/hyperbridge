// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod host;
mod mmr;
mod primitives;
mod router;

use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use ismp_rust::router::{Request, Response};
use sp_core::offchain::StorageKind;
// Re-export pallet items so that they can be accessed from the crate namespace.
use crate::mmr::storage::{OffchainKeyGenerator, StorageReadWrite};
use crate::mmr::{DataOrHash, FullLeaf, Leaf, LeafIndex, Node, NodeIndex, NodeOf};
pub use pallet::*;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[frame_support::pallet]
pub mod pallet {
    // Import various types used to declare pallet in scope.
    use super::*;
    use crate::mmr::{LeafIndex, Mmr, NodeIndex};
    use crate::primitives::ISMP_ID;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use ismp_rust::host::ChainID;
    use sp_runtime::traits;

    /// Our pallet's configuration trait. All our types and constants go in here. If the
    /// pallet is dependent on specific other pallets, then their configuration traits
    /// should be added to our implied traits list.
    ///
    /// `frame_system::Config` should always be included.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Prefix for elements stored in the Off-chain DB via Indexing API.
        ///
        /// Each node of the MMR is inserted both on-chain and off-chain via Indexing API.
        /// The former does not store full leaf content, just its compact version (hash),
        /// and some of the inner mmr nodes might be pruned from on-chain storage.
        /// The latter will contain all the entries in their full form.
        ///
        /// Each node is stored in the Off-chain DB under key derived from the
        /// [`Self::INDEXING_PREFIX`] and its in-tree index (MMR position).
        const INDEXING_PREFIX: &'static [u8];

        /// A hasher type for MMR.
        ///
        /// To construct trie nodes that result in merging (bagging) two peaks, depending on the
        /// node kind we take either:
        /// - The node (hash) itself if it's an inner node.
        /// - The hash of SCALE-encoding of the leaf data if it's a leaf node.
        ///
        /// Then we create a tuple of these two hashes, SCALE-encode it (concatenate) and
        /// hash, to obtain a new MMR inner node - the new peak.
        type Hashing: traits::Hash<Output = <Self as Config>::Hash>;
        const CHAIN_ID: ChainID;
        /// The hashing output type.
        ///
        /// This type is actually going to be stored in the MMR.
        /// Required to be provided again, to satisfy trait bounds for storage items.
        type Hash: traits::Member
            + traits::MaybeSerializeDeserialize
            + sp_std::fmt::Debug
            + sp_std::hash::Hash
            + AsRef<[u8]>
            + AsMut<[u8]>
            + Copy
            + Default
            + codec::Codec
            + codec::EncodeLike
            + scale_info::TypeInfo
            + MaxEncodedLen;
    }

    // Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
    // method.
    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Latest MMR Root hash for requests
    #[pallet::storage]
    #[pallet::getter(fn requests_root_hash)]
    pub type RequestsRootHash<T: Config> = StorageValue<_, <T as Config>::Hash, ValueQuery>;

    /// Latest MMR Root hash for responses
    #[pallet::storage]
    #[pallet::getter(fn responses_root_hash)]
    pub type ResponsesRootHash<T: Config> = StorageValue<_, <T as Config>::Hash, ValueQuery>;

    /// Current size of the MMR (number of leaves) for requests.
    #[pallet::storage]
    #[pallet::getter(fn number_of_request_leaves)]
    pub type NumberOfRequestLeaves<T> = StorageValue<_, LeafIndex, ValueQuery>;

    /// Current size of the MMR (number of leaves) for responses.
    #[pallet::storage]
    #[pallet::getter(fn number_of_response_leaves)]
    pub type NumberOfResponseLeaves<T> = StorageValue<_, LeafIndex, ValueQuery>;

    /// Hashes of the nodes in the MMR for requests.
    ///
    /// Note this collection only contains MMR peaks, the inner nodes (and leaves)
    /// are pruned and only stored in the Offchain DB.
    #[pallet::storage]
    #[pallet::getter(fn request_peaks)]
    pub type RequestNodes<T: Config> =
        StorageMap<_, Identity, NodeIndex, <T as Config>::Hash, OptionQuery>;

    /// Hashes of the nodes in the MMR for responses.
    ///
    /// Note this collection only contains MMR peaks, the inner nodes (and leaves)
    /// are pruned and only stored in the Offchain DB.
    #[pallet::storage]
    #[pallet::getter(fn response_peaks)]
    pub type ResponseNodes<T: Config> =
        StorageMap<_, Identity, NodeIndex, <T as Config>::Hash, OptionQuery>;

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: T::BlockNumber) -> Weight {
            // return Mmr finalization weight here
            Weight::zero()
        }

        fn on_finalize(_n: T::BlockNumber) {
            use crate::mmr;
            // handle finalizing requests Mmr
            let request_leaves = Self::number_of_request_leaves();

            let request_mmr: Mmr<
                mmr::storage::RuntimeStorage,
                T,
                Leaf,
                RequestOffchainKey<T, Leaf>,
                RequestsStore<T>,
            > = mmr::Mmr::new(request_leaves);

            // Update the size, `mmr.finalize()` should also never fail.
            let (leaves, requests_root) = match request_mmr.finalize() {
                Ok((leaves, root)) => (leaves, root),
                Err(e) => {
                    log::error!(target: "runtime::mmr", "MMR finalize failed: {:?}", e);
                    return;
                }
            };

            <NumberOfRequestLeaves<T>>::put(leaves);
            <RequestsRootHash<T>>::put(requests_root);

            // handle finalizing response Mmr
            let response_leaves = Self::number_of_response_leaves();

            let response_mmr: Mmr<
                mmr::storage::RuntimeStorage,
                T,
                Leaf,
                ResponseOffchainKey<T, Leaf>,
                ResponseStore<T>,
            > = mmr::Mmr::new(response_leaves);

            // Update the size, `mmr.finalize()` should also never fail.
            let (leaves, responses_root) = match response_mmr.finalize() {
                Ok((leaves, root)) => (leaves, root),
                Err(e) => {
                    log::error!(target: "runtime::mmr", "MMR finalize failed: {:?}", e);
                    return;
                }
            };

            <NumberOfResponseLeaves<T>>::put(leaves);
            <ResponsesRootHash<T>>::put(responses_root);

            let log = RequestResponseLog::<T> {
                requests_root_hash: requests_root,
                responses_root_hash: responses_root,
            };

            let digest = sp_runtime::generic::DigestItem::Consensus(ISMP_ID, log.encode());
            <frame_system::Pallet<T>>::deposit_log(digest);
        }

        fn offchain_worker(_n: T::BlockNumber) {}
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    /// Events are a simple means of reporting specific conditions and
    /// circumstances that have happened that users, Dapps and/or chain explorers would find
    /// interesting and otherwise difficult to detect.
    #[pallet::event]
    /// This attribute generate the function `deposit_event` to deposit one of this pallet event,
    /// it is optional, it is also possible to provide a custom implementation.
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}
}

impl<T: Config> Pallet<T> {
    /// Generate an MMR proof for the given `leaf_indices`.
    /// Note this method can only be used from an off-chain context
    /// (Offchain Worker or Runtime API call), since it requires
    /// all the leaves to be present.
    /// It may return an error or panic if used incorrectly.
    pub fn generate_request_proof(
        leaf_indices: Vec<LeafIndex>,
    ) -> Result<(Vec<Leaf>, primitives::Proof<<T as Config>::Hash>), primitives::Error> {
        let leaves_count = NumberOfRequestLeaves::<T>::get();
        let mmr = mmr::Mmr::<
            mmr::storage::OffchainStorage,
            T,
            Leaf,
            RequestOffchainKey<T, Leaf>,
            RequestsStore<T>,
        >::new(leaves_count);
        mmr.generate_request_proof(leaf_indices)
    }

    pub fn generate_response_proof(
        leaf_indices: Vec<LeafIndex>,
    ) -> Result<(Vec<Leaf>, primitives::Proof<<T as Config>::Hash>), primitives::Error> {
        let leaves_count = NumberOfRequestLeaves::<T>::get();
        let mmr = mmr::Mmr::<
            mmr::storage::OffchainStorage,
            T,
            Leaf,
            ResponseOffchainKey<T, Leaf>,
            ResponseStore<T>,
        >::new(leaves_count);
        mmr.generate_response_proof(leaf_indices)
    }

    /// Return the on-chain MMR root hash.
    pub fn requests_mmr_root() -> <T as Config>::Hash {
        Self::requests_root_hash()
    }
    /// Return the on-chain MMR root hash.
    pub fn responses_mmr_root() -> <T as Config>::Hash {
        Self::responses_root_hash()
    }
}

pub struct RequestsStore<T>(core::marker::PhantomData<T>);

impl<T: Config, L: FullLeaf<<T as Config>::Hashing>> StorageReadWrite<T, L> for RequestsStore<T> {
    fn get_node(pos: NodeIndex) -> Option<NodeOf<T, L>> {
        RequestNodes::<T>::get(pos).map(Node::Hash)
    }

    fn remove_node(pos: NodeIndex) {
        RequestNodes::<T>::remove(pos);
    }

    fn insert_node(pos: NodeIndex, node: <T as Config>::Hash) {
        RequestNodes::<T>::insert(pos, node)
    }

    fn get_num_leaves() -> LeafIndex {
        NumberOfRequestLeaves::<T>::get()
    }

    fn set_num_leaves(num_leaves: LeafIndex) {
        NumberOfRequestLeaves::<T>::put(num_leaves)
    }
}

pub struct ResponseStore<T>(core::marker::PhantomData<T>);

impl<T: Config, L: FullLeaf<<T as Config>::Hashing>> StorageReadWrite<T, L> for ResponseStore<T> {
    fn get_node(pos: NodeIndex) -> Option<NodeOf<T, L>> {
        ResponseNodes::<T>::get(pos).map(Node::Hash)
    }

    fn remove_node(pos: NodeIndex) {
        ResponseNodes::<T>::remove(pos);
    }

    fn insert_node(pos: NodeIndex, node: <T as Config>::Hash) {
        ResponseNodes::<T>::insert(pos, node)
    }

    fn get_num_leaves() -> LeafIndex {
        NumberOfResponseLeaves::<T>::get()
    }

    fn set_num_leaves(num_leaves: LeafIndex) {
        NumberOfResponseLeaves::<T>::put(num_leaves)
    }
}

pub struct RequestOffchainKey<T, L>(core::marker::PhantomData<(T, L)>);

impl<T: Config, L: FullLeaf<<T as Config>::Hashing>> OffchainKeyGenerator
    for RequestOffchainKey<T, L>
{
    fn offchain_key(pos: NodeIndex) -> Vec<u8> {
        (T::INDEXING_PREFIX, "Requests", pos).encode()
    }
}

pub struct ResponseOffchainKey<T, L>(core::marker::PhantomData<(T, L)>);

impl<T: Config, L: FullLeaf<<T as Config>::Hashing>> OffchainKeyGenerator
    for ResponseOffchainKey<T, L>
{
    fn offchain_key(pos: NodeIndex) -> Vec<u8> {
        (T::INDEXING_PREFIX, "Responses", pos).encode()
    }
}

#[derive(RuntimeDebug, Encode, Decode)]
pub struct RequestResponseLog<T: Config> {
    requests_root_hash: <T as Config>::Hash,
    responses_root_hash: <T as Config>::Hash,
}

impl<T: Config> Pallet<T> {
    fn request_leaf_index_offchain_key(req: &Request) -> Vec<u8> {
        (
            T::INDEXING_PREFIX,
            "Requests/leaf_indices",
            req.dest_chain,
            req.nonce,
        )
            .encode()
    }
    fn response_leaf_index_offchain_key(res: &Response) -> Vec<u8> {
        (
            T::INDEXING_PREFIX,
            "Responses/leaf_indices",
            res.request.source_chain,
            res.request.nonce,
        )
            .encode()
    }

    fn store_leaf_index_offchain(key: Vec<u8>, leaf_index: LeafIndex) {
        sp_io::offchain_index::set(&key, &leaf_index.encode());
    }

    fn get_request(leaf_index: LeafIndex) -> Option<Request> {
        let key = RequestOffchainKey::<T, Leaf>::offchain_key(leaf_index);
        if let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
            let data_or_hash =
                DataOrHash::<<T as Config>::Hashing, Leaf>::decode(&mut &*elem).ok()?;
            return match data_or_hash {
                DataOrHash::Data(leaf) => match leaf {
                    Leaf::Request(req) => Some(req),
                    _ => None,
                },
                _ => None,
            };
        }
        None
    }

    fn get_response(leaf_index: LeafIndex) -> Option<Response> {
        let key = ResponseOffchainKey::<T, Leaf>::offchain_key(leaf_index);
        if let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
            let data_or_hash =
                DataOrHash::<<T as Config>::Hashing, Leaf>::decode(&mut &*elem).ok()?;
            return match data_or_hash {
                DataOrHash::Data(leaf) => match leaf {
                    Leaf::Response(res) => Some(res),
                    _ => None,
                },
                _ => None,
            };
        }
        None
    }
}
