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
mod mmr;
pub mod primitives;
pub mod router;

use crate::host::Host;
use codec::{Decode, Encode};
use core::time::Duration;
use frame_support::{log::debug, RuntimeDebug};
use ismp_rs::{
    consensus::{ConsensusClientId, StateMachineId},
    host::StateMachine,
    messaging::CreateConsensusClient,
    router::{Request, Response},
};
use sp_core::{offchain::StorageKind, H256};
// Re-export pallet items so that they can be accessed from the crate namespace.
use ismp_primitives::{
    mmr::{DataOrHash, Leaf, LeafIndex, NodeIndex},
    LeafIndexQuery,
};
use ismp_rs::host::ISMPHost;
use mmr::mmr::Mmr;
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
        primitives::{ConsensusClientProvider, ISMP_ID},
        router::Receipt,
    };
    use alloc::collections::BTreeSet;
    use frame_support::{pallet_prelude::*, traits::UnixTime};
    use frame_system::pallet_prelude::*;
    use ismp_primitives::mmr::{LeafIndex, NodeIndex};
    use ismp_rs::{
        consensus::{ConsensusClientId, StateCommitment, StateMachineHeight, StateMachineId},
        handlers::{self, handle_incoming_message, MessageResult},
        host::StateMachine,
        messaging::Message,
        router::ISMPRouter,
    };
    use sp_core::H256;

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

        /// Admin origin for privileged actions
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Host state machine identifier
        type StateMachine: Get<StateMachine>;

        /// Timestamp provider
        type TimeProvider: UnixTime;

        /// Configurable router that dispatches calls to modules
        type IsmpRouter: ISMPRouter + Default;
        /// Provides concrete implementations of consensus clients
        type ConsensusClientProvider: ConsensusClientProvider;
    }

    // Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
    // method.
    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Latest MMR Root hash
    #[pallet::storage]
    #[pallet::getter(fn mmr_root_hash)]
    pub type RootHash<T: Config> = StorageValue<_, <T as frame_system::Config>::Hash, ValueQuery>;

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
        StorageMap<_, Identity, NodeIndex, <T as frame_system::Config>::Hash, OptionQuery>;

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
    /// The key is the request commitment
    pub type RequestAcks<T: Config> =
        StorageMap<_, Blake2_128Concat, Vec<u8>, Receipt, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn response_acks)]
    /// Acknowledgements for receipt of responses
    /// The key is the response commitment
    pub type ResponseAcks<T: Config> =
        StorageMap<_, Blake2_128Concat, Vec<u8>, Receipt, OptionQuery>;

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

    /// State variable that tells us if at least one new leaf was added to the mmr
    #[pallet::storage]
    #[pallet::getter(fn new_leaves)]
    pub type NewLeavesAdded<T> = StorageValue<_, LeafIndex, OptionQuery>;

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
        fn on_initialize(_n: T::BlockNumber) -> Weight {
            // return Mmr finalization weight here
            Weight::zero()
        }

        fn on_finalize(_n: T::BlockNumber) {
            // Only finalize if mmr was modified
            let root = if !NewLeavesAdded::<T>::exists() {
                <RootHash<T>>::get()
            } else {
                let leaves = Self::number_of_leaves();
                let mmr: Mmr<mmr::storage::RuntimeStorage, T> = Mmr::new(leaves);

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
                NewLeavesAdded::<T>::kill();

                root
            };

            let digest = sp_runtime::generic::DigestItem::Consensus(ISMP_ID, root.encode());
            <frame_system::Pallet<T>>::deposit_log(digest);
        }

        fn offchain_worker(_n: T::BlockNumber) {}
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
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
                        // check if this is a trusted state machine
                        let is_trusted_state_machine = host
                            .challenge_period(res.consensus_client_id.clone()) ==
                            Duration::from_secs(0);

                        if is_trusted_state_machine {
                            for (_, latest_height) in res.state_updates.into_iter() {
                                Self::deposit_event(Event::<T>::StateMachineUpdated {
                                    state_machine_id: latest_height.id,
                                    latest_height: latest_height.height,
                                })
                            }
                        } else {
                            if let Some(pending_updates) =
                                ConsensusUpdateResults::<T>::get(res.consensus_client_id)
                            {
                                for (_, latest_height) in pending_updates.into_iter() {
                                    Self::deposit_event(Event::<T>::StateMachineUpdated {
                                        state_machine_id: latest_height.id,
                                        latest_height: latest_height.height,
                                    })
                                }
                            }

                            Self::deposit_event(Event::<T>::ChallengePeriodStarted {
                                consensus_client_id: res.consensus_client_id,
                                state_machines: res.state_updates.clone(),
                            });

                            // Store the new update result that have just entered the challenge
                            // period
                            ConsensusUpdateResults::<T>::insert(
                                res.consensus_client_id,
                                res.state_updates,
                            );
                        }
                    }
                    Ok(_) => {
                        // Do nothing, event should have been deposited by the ismp router
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

        /// Create consensus clients
        #[pallet::weight(0)]
        #[pallet::call_index(1)]
        pub fn create_consensus_client(
            origin: OriginFor<T>,
            message: CreateConsensusClient,
        ) -> DispatchResult {
            <T as Config>::AdminOrigin::ensure_origin(origin)?;
            let host = Host::<T>::default();

            let result = handlers::create_consensus_client(&host, message)
                .map_err(|_| Error::<T>::ConsensusClientCreationFailed)?;

            Self::deposit_event(Event::<T>::ConsensusClientCreated {
                consensus_client_id: result.consensus_client_id,
            });

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
        /// Emitted when a state machine is successfully updated to a new height
        StateMachineUpdated { state_machine_id: StateMachineId, latest_height: u64 },
        /// Signifies that a client has begun it's challenge period
        ChallengePeriodStarted {
            consensus_client_id: ConsensusClientId,
            state_machines: BTreeSet<(StateMachineHeight, StateMachineHeight)>,
        },
        /// Indicates that a consensus client has been created
        ConsensusClientCreated { consensus_client_id: ConsensusClientId },
        /// Response was process successfully
        Response {
            /// Chain that this response will be routed to
            dest_chain: StateMachine,
            /// Source Chain for this response
            source_chain: StateMachine,
            /// Nonce for the request which this response is for
            request_nonce: u64,
        },
        /// Request processed successfully
        Request {
            /// Chain that this request will be routed to
            dest_chain: StateMachine,
            /// Source Chain for request
            source_chain: StateMachine,
            /// Request nonce
            request_nonce: u64,
        },
        /// Some errors handling some ismp messages
        HandlingErrors { errors: Vec<HandlingError> },
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidMessage,
        ConsensusClientCreationFailed,
    }
}

impl<T: Config> Pallet<T>
where
    <T as frame_system::Config>::Hash: From<H256>,
{
    /// Generate an MMR proof for the given `leaf_indices`.
    /// Note this method can only be used from an off-chain context
    /// (Offchain Worker or Runtime API call), since it requires
    /// all the leaves to be present.
    /// It may return an error or panic if used incorrectly.
    pub fn generate_proof(
        leaf_indices: Vec<LeafIndex>,
    ) -> Result<(Vec<Leaf>, primitives::Proof<<T as frame_system::Config>::Hash>), primitives::Error>
    {
        let leaves_count = NumberOfLeaves::<T>::get();
        let mmr = Mmr::<mmr::storage::OffchainStorage, T>::new(leaves_count);
        mmr.generate_proof(leaf_indices)
    }

    /// Return the on-chain MMR root hash.
    pub fn mmr_root() -> <T as frame_system::Config>::Hash {
        Self::mmr_root_hash()
    }
}

impl<T: Config> Pallet<T> {
    fn get_node(pos: NodeIndex) -> Option<DataOrHash<T>> {
        Nodes::<T>::get(pos).map(DataOrHash::Hash)
    }

    fn remove_node(pos: NodeIndex) {
        Nodes::<T>::remove(pos);
    }

    fn insert_node(pos: NodeIndex, node: <T as frame_system::Config>::Hash) {
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
    mmr_root_hash: <T as frame_system::Config>::Hash,
}

impl<T: Config> Pallet<T>
where
    <T as frame_system::Config>::Hash: From<H256>,
{
    pub fn request_leaf_index_offchain_key(
        source_chain: StateMachine,
        dest_chain: StateMachine,
        nonce: u64,
    ) -> Vec<u8> {
        (T::INDEXING_PREFIX, "Requests/leaf_indices", source_chain, dest_chain, nonce).encode()
    }

    pub fn response_leaf_index_offchain_key(
        source_chain: StateMachine,
        dest_chain: StateMachine,
        nonce: u64,
    ) -> Vec<u8> {
        (T::INDEXING_PREFIX, "Responses/leaf_indices", source_chain, dest_chain, nonce).encode()
    }

    pub fn store_leaf_index_offchain(key: Vec<u8>, leaf_index: LeafIndex) {
        sp_io::offchain_index::set(&key, &leaf_index.encode());
    }

    pub fn get_request(leaf_index: LeafIndex) -> Option<Request> {
        let key = Pallet::<T>::offchain_key(leaf_index);
        if let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
            let data_or_hash = DataOrHash::<T>::decode(&mut &*elem).ok()?;
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
            let data_or_hash = DataOrHash::<T>::decode(&mut &*elem).ok()?;
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
        source_chain: StateMachine,
        dest_chain: StateMachine,
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

    /// Return the scale encoded consensus state
    pub fn get_consensus_state(id: ConsensusClientId) -> Option<Vec<u8>> {
        ConsensusStates::<T>::get(id)
    }

    /// Return the timestamp this client was last updated in seconds
    pub fn get_consensus_update_time(id: ConsensusClientId) -> Option<u64> {
        ConsensusClientUpdateTime::<T>::get(id)
    }

    /// Return the latest height of the state machine
    pub fn get_latest_state_machine_height(id: StateMachineId) -> Option<u64> {
        LatestStateMachineHeight::<T>::get(id)
    }

    /// Get Request Leaf Indices
    pub fn get_request_leaf_indices(leaf_queries: Vec<LeafIndexQuery>) -> Vec<LeafIndex> {
        leaf_queries
            .into_iter()
            .filter_map(|query| {
                Self::get_leaf_index(query.source_chain, query.dest_chain, query.nonce, true)
            })
            .collect()
    }

    /// Get Response Leaf Indices
    pub fn get_response_leaf_indices(leaf_queries: Vec<LeafIndexQuery>) -> Vec<LeafIndex> {
        leaf_queries
            .into_iter()
            .filter_map(|query| {
                Self::get_leaf_index(query.source_chain, query.dest_chain, query.nonce, false)
            })
            .collect()
    }

    /// Get actual requests
    pub fn get_requests(leaf_indices: Vec<LeafIndex>) -> Vec<Request> {
        leaf_indices.into_iter().filter_map(|leaf_index| Self::get_request(leaf_index)).collect()
    }

    /// Get actual requests
    pub fn get_responses(leaf_indices: Vec<LeafIndex>) -> Vec<Response> {
        leaf_indices.into_iter().filter_map(|leaf_index| Self::get_response(leaf_index)).collect()
    }

    pub fn mmr_push(leaf: Leaf) -> Option<NodeIndex> {
        let leaves = Self::number_of_leaves();
        let mut mmr: Mmr<mmr::storage::RuntimeStorage, T> = Mmr::new(leaves);
        let index = mmr.push(leaf);
        if !NewLeavesAdded::<T>::exists() && index.is_some() {
            NewLeavesAdded::<T>::put(index.unwrap())
        }
        index
    }
}
