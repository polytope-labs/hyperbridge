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

extern crate alloc;

mod errors;
pub mod events;
pub mod host;
pub mod mmr;
pub mod primitives;
mod router;

use crate::{
    host::Host,
    mmr::{DataOrHash, Leaf, LeafIndex, NodeIndex, NodeOf},
};
use codec::{Decode, Encode};
use frame_support::{log::debug, RuntimeDebug};
use ismp_rs::{
    host::ChainID,
    messaging::Message,
    router::{Request, Response},
};
use sp_core::offchain::StorageKind;
// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;
use sp_std::prelude::*;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[frame_support::pallet]
pub mod pallet {
    // Import various types used to declare pallet in scope.
    use super::*;
    use crate::{
        errors::HandlingError,
        mmr::{LeafIndex, Mmr, NodeIndex},
        primitives::ISMP_ID,
    };
    use alloc::collections::BTreeSet;
    use frame_support::{pallet_prelude::*, traits::UnixTime};
    use frame_system::pallet_prelude::*;
    use ismp_rs::{
        consensus_client::{
            ConsensusClientId, StateCommitment, StateMachineHeight, StateMachineId,
        },
        handlers::{handle_incoming_message, MessageResult},
        host::ChainID,
    };
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
            + From<[u8; 32]>
            + Copy
            + Default
            + codec::Codec
            + codec::EncodeLike
            + scale_info::TypeInfo
            + MaxEncodedLen;
        type TimeProvider: UnixTime;
    }

    // Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
    // method.
    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Latest MMR Root hash
    #[pallet::storage]
    #[pallet::getter(fn mmr_root_hash)]
    pub type RootHash<T: Config> = StorageValue<_, <T as Config>::Hash, ValueQuery>;

    /// Current size of the MMR (number of leaves) for requests.
    #[pallet::storage]
    #[pallet::getter(fn number_of_leaves)]
    pub type NumberOfLeaves<T> = StorageValue<_, LeafIndex, ValueQuery>;

    /// Hashes of the nodes in the MMR for requests.
    ///
    /// Note this collection only contains MMR peaks, the inner nodes (and leaves)
    /// are pruned and only stored in the Offchain DB.
    #[pallet::storage]
    #[pallet::getter(fn request_peaks)]
    pub type Nodes<T: Config> =
        StorageMap<_, Identity, NodeIndex, <T as Config>::Hash, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn state_commitments)]
    pub type StateCommitments<T: Config> =
        StorageMap<_, Blake2_128Concat, StateMachineHeight, StateCommitment, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn consensus_states)]
    pub type ConsensusStates<T: Config> =
        StorageMap<_, Twox64Concat, ConsensusClientId, Vec<u8>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn frozen_heights)]
    pub type FrozenHeights<T: Config> =
        StorageMap<_, Blake2_128Concat, StateMachineId, u64, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn latest_state_height)]
    /// The latest accepted state machine height
    pub type LatestStateMachineHeight<T: Config> =
        StorageMap<_, Blake2_128Concat, StateMachineId, u64, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn consensus_update_time)]
    pub type ConsensusClientUpdateTime<T: Config> =
        StorageMap<_, Twox64Concat, ConsensusClientId, u64, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn request_acks)]
    /// Acknowledgements for receipt of requests
    /// No hashing, just insert raw key in storage
    pub type RequestAcks<T: Config> = StorageMap<_, Identity, Vec<u8>, Vec<u8>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn response_acks)]
    /// Acknowledgements for receipt of responses
    /// No hashing, just insert raw key in storage
    pub type ResponseAcks<T: Config> = StorageMap<_, Identity, Vec<u8>, Vec<u8>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn consensus_update_results)]
    /// Consensus update results still in challenge period
    /// Set contains a tuple of previous height and latest height
    pub type ConsensusUpdateResults<T: Config> = StorageMap<
        _,
        Twox64Concat,
        ConsensusClientId,
        BTreeSet<(StateMachineHeight, StateMachineHeight)>,
        OptionQuery,
    >;

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: T::BlockNumber) -> Weight {
            // return Mmr finalization weight here
            Weight::zero()
        }

        fn on_finalize(_n: T::BlockNumber) {
            use crate::mmr;
            let leaves = Self::number_of_leaves();

            let mmr: Mmr<mmr::storage::RuntimeStorage, T, Leaf> = mmr::Mmr::new(leaves);

            // Update the size, `mmr.finalize()` should also never fail.
            let (leaves, root) = match mmr.finalize() {
                Ok((leaves, root)) => (leaves, root),
                Err(e) => {
                    log::error!(target: "runtime::mmr", "MMR finalize failed: {:?}", e);
                    return
                }
            };

            <NumberOfLeaves<T>>::put(leaves);
            <RootHash<T>>::put(root);

            let log = RequestResponseLog::<T> { mmr_root_hash: root };

            let digest = sp_runtime::generic::DigestItem::Consensus(ISMP_ID, log.encode());
            <frame_system::Pallet<T>>::deposit_log(digest);
        }

        fn offchain_worker(_n: T::BlockNumber) {}
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Handles ismp messages
        #[pallet::weight(0)]
        #[pallet::call_index(0)]
        pub fn handle(origin: OriginFor<T>, messages: Vec<Message>) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // Define a host
            let host = Host::<T>::default();
            let mut errors: Vec<HandlingError> = vec![];
            for message in messages {
                match handle_incoming_message(&host, message) {
                    Ok(MessageResult::ConsensusMessage(res)) => {
                        // Deposit events for previous update result that has passed the challenge
                        // period
                        if let Some(pending_updates) =
                            ConsensusUpdateResults::<T>::get(res.consensus_client_id)
                        {
                            for (prev_height, latest_height) in pending_updates.into_iter() {
                                Self::deposit_event(Event::<T>::StateMachineUpdated {
                                    state_machine_id: latest_height.id,
                                    latest_height: latest_height.height,
                                    previous_height: prev_height.height,
                                })
                            }
                        }

                        Self::deposit_event(Event::<T>::ChallengePeriodStarted {
                            consensus_client_id: res.consensus_client_id,
                            state_machines: res.state_updates.clone(),
                        });

                        // Store the new update result that have just entered the challenge period
                        ConsensusUpdateResults::<T>::insert(
                            res.consensus_client_id,
                            res.state_updates,
                        );
                    }
                    Ok(_) => {
                        // Do nothing, event has been deposited in ismp router
                    }
                    Err(err) => {
                        errors.push(err.into());
                    }
                }
            }

            if !errors.is_empty() {
                debug!(target: "ismp-rust", "Handling Errors {:?}", errors);
                Self::deposit_event(Event::<T>::HandlingErrors { errors })
            }

            Ok(())
        }
    }

    /// Events are a simple means of reporting specific conditions and
    /// circumstances that have happened that users, Dapps and/or chain explorers would find
    /// interesting and otherwise difficult to detect.
    #[pallet::event]
    /// This attribute generate the function `deposit_event` to deposit one of this pallet event,
    /// it is optional, it is also possible to provide a custom implementation.
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event to be emitted when the challenge period for a state machine update has elapsed
        StateMachineUpdated {
            state_machine_id: StateMachineId,
            latest_height: u64,
            previous_height: u64,
        },
        /// Signifies that a client has begun it's challenge period
        ChallengePeriodStarted {
            consensus_client_id: ConsensusClientId,
            state_machines: BTreeSet<(StateMachineHeight, StateMachineHeight)>,
        },
        /// Response was process successfully
        Response {
            /// Chain that this response will be routed to
            dest_chain: ChainID,
            /// Source Chain for this response
            source_chain: ChainID,
            /// Nonce for the request which this response is for
            request_nonce: u64,
        },
        /// Request processed successfully
        Request {
            /// Chain that this request will be routed to
            dest_chain: ChainID,
            /// Source Chain for request
            source_chain: ChainID,
            /// Request nonce
            request_nonce: u64,
        },
        HandlingErrors {
            errors: Vec<HandlingError>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {}
}

impl<T: Config> Pallet<T> {
    /// Generate an MMR proof for the given `leaf_indices`.
    /// Note this method can only be used from an off-chain context
    /// (Offchain Worker or Runtime API call), since it requires
    /// all the leaves to be present.
    /// It may return an error or panic if used incorrectly.
    pub fn generate_proof(
        leaf_indices: Vec<LeafIndex>,
    ) -> Result<(Vec<Leaf>, primitives::Proof<<T as Config>::Hash>), primitives::Error> {
        let leaves_count = NumberOfLeaves::<T>::get();
        let mmr = mmr::Mmr::<mmr::storage::OffchainStorage, T, Leaf>::new(leaves_count);
        mmr.generate_proof(leaf_indices)
    }

    /// Return the on-chain MMR root hash.
    pub fn mmr_root() -> <T as Config>::Hash {
        Self::mmr_root_hash()
    }
}

impl<T: Config> Pallet<T> {
    fn get_node<L>(pos: NodeIndex) -> Option<NodeOf<T, L>> {
        Nodes::<T>::get(pos).map(NodeOf::Hash)
    }

    fn remove_node(pos: NodeIndex) {
        Nodes::<T>::remove(pos);
    }

    fn insert_node(pos: NodeIndex, node: <T as Config>::Hash) {
        Nodes::<T>::insert(pos, node)
    }

    fn get_num_leaves() -> LeafIndex {
        NumberOfLeaves::<T>::get()
    }

    fn set_num_leaves(num_leaves: LeafIndex) {
        NumberOfLeaves::<T>::put(num_leaves)
    }

    fn offchain_key(pos: NodeIndex) -> Vec<u8> {
        (T::INDEXING_PREFIX, "Requests/Responses", pos).encode()
    }
}

#[derive(RuntimeDebug, Encode, Decode)]
pub struct RequestResponseLog<T: Config> {
    mmr_root_hash: <T as Config>::Hash,
}

impl<T: Config> Pallet<T> {
    pub fn request_leaf_index_offchain_key(
        source_chain: ChainID,
        dest_chain: ChainID,
        nonce: u64,
    ) -> Vec<u8> {
        (T::INDEXING_PREFIX, "Requests/leaf_indices", source_chain, dest_chain, nonce).encode()
    }

    pub fn response_leaf_index_offchain_key(
        source_chain: ChainID,
        dest_chain: ChainID,
        nonce: u64,
    ) -> Vec<u8> {
        (T::INDEXING_PREFIX, "Responses/leaf_indices", source_chain, dest_chain, nonce).encode()
    }

    fn store_leaf_index_offchain(key: Vec<u8>, leaf_index: LeafIndex) {
        sp_io::offchain_index::set(&key, &leaf_index.encode());
    }

    pub fn get_request(leaf_index: LeafIndex) -> Option<Request> {
        let key = Pallet::<T>::offchain_key(leaf_index);
        if let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
            let data_or_hash = DataOrHash::<T, Leaf>::decode(&mut &*elem).ok()?;
            return match data_or_hash {
                DataOrHash::Data(leaf) => match leaf {
                    Leaf::Request(req) => Some(req),
                    _ => None,
                },
                _ => None,
            }
        }
        None
    }

    pub fn get_response(leaf_index: LeafIndex) -> Option<Response> {
        let key = Pallet::<T>::offchain_key(leaf_index);
        if let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
            let data_or_hash = DataOrHash::<T, Leaf>::decode(&mut &*elem).ok()?;
            return match data_or_hash {
                DataOrHash::Data(leaf) => match leaf {
                    Leaf::Response(res) => Some(res),
                    _ => None,
                },
                _ => None,
            }
        }
        None
    }

    pub fn get_leaf_index(
        source_chain: ChainID,
        dest_chain: ChainID,
        nonce: u64,
        is_req: bool,
    ) -> Option<LeafIndex> {
        let key = if is_req {
            Self::request_leaf_index_offchain_key(source_chain, dest_chain, nonce)
        } else {
            Self::response_leaf_index_offchain_key(source_chain, dest_chain, nonce)
        };
        if let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
            return LeafIndex::decode(&mut &*elem).ok()
        }
        None
    }
}
