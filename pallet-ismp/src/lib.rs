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

//! ISMP implementation for substrate-based chains.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;
extern crate core;

pub mod benchmarking;
pub mod dispatcher;
mod errors;
pub mod events;
pub mod handlers;
pub mod host;
mod mmr;
#[cfg(any(feature = "runtime-benchmarks", feature = "testing", test))]
pub mod mocks;
pub mod primitives;
#[cfg(test)]
pub mod tests;
pub mod weight_info;

pub use mmr::utils::NodesUtils;

use crate::host::Host;
use codec::{Decode, Encode};
use core::time::Duration;
use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo, Pays, PostDispatchInfo},
    log::debug,
    traits::{Get, UnixTime},
    RuntimeDebug,
};
use ismp_rs::{
    consensus::{ConsensusClientId, StateMachineId},
    handlers::{handle_incoming_message, MessageResult},
    host::StateMachine,
    messaging::CreateConsensusState,
    router::{Request, Response},
};
use sp_core::{offchain::StorageKind, H256};
// Re-export pallet items so that they can be accessed from the crate namespace.
use crate::{
    errors::{HandlingError, ModuleCallbackResult},
    mmr::mmr::Mmr,
    weight_info::get_weight,
};
use ismp_primitives::{
    mmr::{DataOrHash, Leaf, LeafIndex, NodeIndex},
    LeafIndexQuery,
};
use ismp_rs::{host::IsmpHost, messaging::Message};
pub use pallet::*;
use sp_std::prelude::*;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[frame_support::pallet]
pub mod pallet {

    // Import various types used to declare pallet in scope.
    use super::*;
    use crate::{
        dispatcher::Receipt,
        errors::HandlingError,
        primitives::{ConsensusClientProvider, WeightUsed},
        weight_info::{WeightInfo, WeightProvider},
    };
    use alloc::collections::BTreeSet;
    use frame_support::{pallet_prelude::*, traits::UnixTime};
    use frame_system::pallet_prelude::*;
    use ismp_primitives::{
        mmr::{LeafIndex, NodeIndex},
        ISMP_ID,
    };
    use ismp_rs::{
        consensus::{
            ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
            StateMachineId,
        },
        handlers::{self},
        host::StateMachine,
        messaging::Message,
        router::IsmpRouter,
    };
    use sp_core::H256;

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
        type IsmpRouter: IsmpRouter + Default;
        /// Provides concrete implementations of consensus clients
        type ConsensusClientProvider: ConsensusClientProvider;

        /// Weight Info
        type WeightInfo: WeightInfo;

        /// Weight provider for consensus clients and module callbacks
        type WeightProvider: WeightProvider;
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

    /// Holds a map of state machine heights to their verified state commitments
    #[pallet::storage]
    #[pallet::getter(fn state_commitments)]
    pub type StateCommitments<T: Config> =
        StorageMap<_, Blake2_128Concat, StateMachineHeight, StateCommitment, OptionQuery>;

    /// Holds a map of consensus clients to their consensus state.
    #[pallet::storage]
    #[pallet::getter(fn consensus_states)]
    pub type ConsensusStates<T: Config> =
        StorageMap<_, Twox64Concat, ConsensusClientId, Vec<u8>, OptionQuery>;

    /// Holds a map of state machines to the height at which they've been frozen due to byzantine
    /// behaviour
    #[pallet::storage]
    #[pallet::getter(fn frozen_heights)]
    pub type FrozenHeights<T: Config> =
        StorageMap<_, Blake2_128Concat, StateMachineId, u64, OptionQuery>;

    /// A mapping of ConsensusStateId to ConsensusClientId
    #[pallet::storage]
    pub type ConsensusStateClient<T: Config> =
        StorageMap<_, Blake2_128Concat, ConsensusStateId, ConsensusClientId, OptionQuery>;

    /// A mapping of ConsensusStateId to Unbonding periods
    #[pallet::storage]
    pub type UnbondingPeriod<T: Config> =
        StorageMap<_, Blake2_128Concat, ConsensusStateId, u64, OptionQuery>;

    /// A mapping of ConsensusStateId to Challenge periods
    #[pallet::storage]
    pub type ChallengePeriod<T: Config> =
        StorageMap<_, Blake2_128Concat, ConsensusStateId, u64, OptionQuery>;

    /// Holds a map of consensus clients frozen due to byzantine
    /// behaviour
    #[pallet::storage]
    #[pallet::getter(fn frozen_consensus_clients)]
    pub type FrozenConsensusClients<T: Config> =
        StorageMap<_, Blake2_128Concat, ConsensusStateId, bool, ValueQuery>;

    /// The latest verified height for a state machine
    #[pallet::storage]
    #[pallet::getter(fn latest_state_height)]
    pub type LatestStateMachineHeight<T: Config> =
        StorageMap<_, Blake2_128Concat, StateMachineId, u64, ValueQuery>;

    /// Bounded vec of allowed proxies
    #[pallet::storage]
    #[pallet::getter(fn allowed_proxies)]
    pub type AllowedProxies<T: Config> = StorageValue<_, Vec<StateMachine>, ValueQuery>;

    /// Holds the timestamp at which a consensus client was recently updated.
    /// Used in ensuring that the configured challenge period elapses.
    #[pallet::storage]
    #[pallet::getter(fn consensus_update_time)]
    pub type ConsensusClientUpdateTime<T: Config> =
        StorageMap<_, Twox64Concat, ConsensusClientId, u64, OptionQuery>;

    /// Commitments for outgoing requests
    /// The key is the request commitment
    #[pallet::storage]
    #[pallet::getter(fn request_commitments)]
    pub type RequestCommitments<T: Config> =
        StorageMap<_, Identity, H256, LeafIndexQuery, OptionQuery>;

    /// Commitments for outgoing responses
    /// The key is the response commitment
    #[pallet::storage]
    #[pallet::getter(fn response_commitments)]
    pub type ResponseCommitments<T: Config> = StorageMap<_, Identity, H256, Receipt, OptionQuery>;

    /// Receipts for incoming requests
    /// The key is the request commitment
    #[pallet::storage]
    #[pallet::getter(fn request_receipts)]
    pub type RequestReceipts<T: Config> = StorageMap<_, Identity, H256, Receipt, OptionQuery>;

    /// Receipts for incoming responses
    /// The key is the request commitment
    #[pallet::storage]
    #[pallet::getter(fn response_receipts)]
    pub type ResponseReceipts<T: Config> = StorageMap<_, Identity, H256, Receipt, OptionQuery>;

    /// Consensus update results still in challenge period
    /// Set contains a tuple of previous height and latest height
    #[pallet::storage]
    #[pallet::getter(fn consensus_update_results)]
    pub type ConsensusUpdateResults<T: Config> = StorageMap<
        _,
        Twox64Concat,
        ConsensusClientId,
        BTreeSet<(StateMachineHeight, StateMachineHeight)>,
        OptionQuery,
    >;

    /// Latest nonce for messages sent from this chain
    #[pallet::storage]
    #[pallet::getter(fn nonce)]
    pub type Nonce<T> = StorageValue<_, u64, ValueQuery>;

    /// Contains a tuple of the weight consumed and weight limit in executing contract callbacks in
    /// a transaction
    #[pallet::storage]
    #[pallet::getter(fn weight_consumed)]
    pub type WeightConsumed<T: Config> = StorageValue<_, WeightUsed, ValueQuery>;

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
        fn on_initialize(_n: T::BlockNumber) -> Weight {
            // return Mmr finalization weight here
            <T as Config>::WeightInfo::on_finalize(Self::number_of_leaves() as u32)
        }

        fn on_finalize(_n: T::BlockNumber) {
            // Only finalize if mmr was modified
            let leaves = Self::number_of_leaves();
            let root = if leaves != 0 {
                let mmr: Mmr<mmr::storage::RuntimeStorage, T> = Mmr::new(leaves);
                // Update the size, `mmr.finalize()` should also never fail.
                let root = match mmr.finalize() {
                    Ok(root) => root,
                    Err(e) => {
                        log::error!(target: "runtime::mmr", "MMR finalize failed: {:?}", e);
                        return
                    }
                };

                <RootHash<T>>::put(root);

                root
            } else {
                H256::default().into()
            };

            let digest = sp_runtime::generic::DigestItem::Consensus(ISMP_ID, root.encode());
            <frame_system::Pallet<T>>::deposit_log(digest);
        }

        fn offchain_worker(_n: T::BlockNumber) {}
    }

    /// Params to update the unbonding period for a consensus state
    #[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
    pub struct UpdateConsensusState {
        /// Consensus state identifier
        pub consensus_state_id: ConsensusStateId,
        /// Unbonding duration
        pub unbonding_period: Option<u64>,
        /// Challenge period duration
        pub challenge_period: Option<u64>,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
        /// Handles ismp messages
        #[pallet::weight(get_weight::<T>(&messages))]
        #[pallet::call_index(0)]
        #[frame_support::transactional]
        pub fn handle(origin: OriginFor<T>, messages: Vec<Message>) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;

            Self::handle_messages(messages)
        }

        /// Create a consensus client, using a subjectively chosen consensus state.
        #[pallet::weight(<T as Config>::WeightInfo::create_consensus_client())]
        #[pallet::call_index(1)]
        pub fn create_consensus_client(
            origin: OriginFor<T>,
            message: CreateConsensusState,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            let host = Host::<T>::default();

            let result = handlers::create_client(&host, message)
                .map_err(|_| Error::<T>::ConsensusClientCreationFailed)?;

            Self::deposit_event(Event::<T>::ConsensusClientCreated {
                consensus_client_id: result.consensus_client_id,
            });

            Ok(())
        }

        /// Set the unbonding period for a consensus state.
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(2))]
        #[pallet::call_index(2)]
        pub fn update_consensus_state(
            origin: OriginFor<T>,
            message: UpdateConsensusState,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let host = Host::<T>::default();

            if let Some(unbonding_period) = message.unbonding_period {
                host.store_unbonding_period(message.consensus_state_id, unbonding_period)
                    .map_err(|_| Error::<T>::UnbondingPeriodUpdateFailed)?;
            }

            if let Some(challenge_period) = message.challenge_period {
                host.store_challenge_period(message.consensus_state_id, challenge_period)
                    .map_err(|_| Error::<T>::UnbondingPeriodUpdateFailed)?;
            }

            Ok(())
        }

        /// Set the allowed proxies
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(1))]
        #[pallet::call_index(3)]
        pub fn set_config(origin: OriginFor<T>, allowed: Vec<StateMachine>) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let host = Host::<T>::default();
            host.store_allowed_proxies(allowed);

            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Emitted when a state machine is successfully updated to a new height
        StateMachineUpdated {
            /// State machine height
            state_machine_id: StateMachineId,
            /// State machine latest height
            latest_height: u64,
        },
        /// Signifies that a client has begun it's challenge period
        ChallengePeriodStarted {
            /// Consensus client id
            consensus_client_id: ConsensusClientId,
            /// Tuple of previous height and latest height for state machines
            state_machines: BTreeSet<(StateMachineHeight, StateMachineHeight)>,
        },
        /// Indicates that a consensus client has been created
        ConsensusClientCreated {
            /// Consensus client id
            consensus_client_id: ConsensusClientId,
        },
        /// An Outgoing Response has been deposited
        Response {
            /// Chain that this response will be routed to
            dest_chain: StateMachine,
            /// Source Chain for this response
            source_chain: StateMachine,
            /// Nonce for the request which this response is for
            request_nonce: u64,
        },
        /// An Outgoing Request has been deposited
        Request {
            /// Chain that this request will be routed to
            dest_chain: StateMachine,
            /// Source Chain for request
            source_chain: StateMachine,
            /// Request nonce
            request_nonce: u64,
        },
        /// Some errors handling some ismp messages
        HandlingErrors {
            /// Message handling errors
            errors: Vec<HandlingError>,
        },
    }

    /// Pallet errors
    #[pallet::error]
    pub enum Error<T> {
        /// Invalid ISMP message
        InvalidMessage,
        /// Encountered an error while creating the consensus client.
        ConsensusClientCreationFailed,
        /// Couldn't update unbonding period
        UnbondingPeriodUpdateFailed,
        /// Couldn't update challenge period
        ChallengePeriodUpdateFailed,
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

    /// Provides a way to handle messages.
    pub fn handle_messages(messages: Vec<Message>) -> DispatchResultWithPostInfo {
        // Define a host
        WeightConsumed::<T>::kill();
        let host = Host::<T>::default();
        let mut errors: Vec<HandlingError> = vec![];
        let total_weight = get_weight::<T>(&messages);
        for message in messages {
            match handle_incoming_message(&host, message) {
                Ok(MessageResult::ConsensusMessage(res)) => {
                    // check if this is a trusted state machine
                    let is_trusted_state_machine = host
                        .challenge_period(res.consensus_state_id.clone()) ==
                        Some(Duration::from_secs(0));

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
                Ok(MessageResult::Response(res)) => {
                    debug!(target: "ismp-modules", "Module Callback Results {:?}", ModuleCallbackResult::Response(res));
                }
                Ok(MessageResult::Request(res)) => {
                    debug!(target: "ismp-modules", "Module Callback Results {:?}", ModuleCallbackResult::Request(res));
                }
                Ok(MessageResult::Timeout(res)) => {
                    debug!(target: "ismp-modules", "Module Callback Results {:?}", ModuleCallbackResult::Timeout(res));
                }
                Err(err) => {
                    errors.push(err.into());
                }
                _ => {}
            }
        }

        if !errors.is_empty() {
            debug!(target: "pallet-ismp", "Handling Errors {:?}", errors);
            Self::deposit_event(Event::<T>::HandlingErrors { errors })
        }

        Ok(PostDispatchInfo {
            actual_weight: {
                let acc_weight = WeightConsumed::<T>::get();
                Some((total_weight - acc_weight.weight_limit) + acc_weight.weight_used)
            },
            pays_fee: Pays::Yes,
        })
    }

    /// Return the on-chain MMR root hash.
    pub fn mmr_root() -> <T as frame_system::Config>::Hash {
        Self::mmr_root_hash()
    }

    /// Return mmr leaf count
    pub fn mmr_leaf_count() -> LeafIndex {
        Self::number_of_leaves()
    }
}

/// Digest log for mmr root hash
#[derive(RuntimeDebug, Encode, Decode)]
pub struct RequestResponseLog<T: Config> {
    /// The mmr root hash
    mmr_root_hash: <T as frame_system::Config>::Hash,
}

impl<T: Config> Pallet<T>
where
    <T as frame_system::Config>::Hash: From<H256>,
{
    /// Returns the offchain key for a request leaf index
    pub fn request_leaf_index_offchain_key(
        source_chain: StateMachine,
        dest_chain: StateMachine,
        nonce: u64,
    ) -> Vec<u8> {
        (T::INDEXING_PREFIX, "requests_leaf_indices", source_chain, dest_chain, nonce).encode()
    }

    /// Returns the offchain key for a response leaf index
    pub fn response_leaf_index_offchain_key(
        source_chain: StateMachine,
        dest_chain: StateMachine,
        nonce: u64,
    ) -> Vec<u8> {
        (T::INDEXING_PREFIX, "responses_leaf_indices", source_chain, dest_chain, nonce).encode()
    }

    /// Stores the leaf index  or the given key
    pub fn store_leaf_index_offchain(key: Vec<u8>, leaf_index: LeafIndex) {
        sp_io::offchain_index::set(&key, &leaf_index.encode());
    }

    /// Gets the request from the offchain storage
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

    /// Gets the response from the offchain storage
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

    /// Gets the leaf index for a request or response from the offchain storage
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

    /// Get unfulfilled Get requests
    pub fn pending_get_requests() -> Vec<ismp_rs::router::Get> {
        RequestCommitments::<T>::iter()
            .filter_map(|(key, query)| {
                let leaf_index =
                    Self::get_leaf_index(query.source_chain, query.dest_chain, query.nonce, true)?;
                let req = Self::get_request(leaf_index)?;
                (req.is_type_get() && !ResponseReceipts::<T>::contains_key(key))
                    .then(|| req.get_request().ok())
                    .flatten()
            })
            .collect()
    }

    /// Return the scale encoded consensus state
    pub fn get_consensus_state(id: ConsensusClientId) -> Option<Vec<u8>> {
        ConsensusStates::<T>::get(id)
    }

    /// Return the timestamp this client was last updated in seconds
    pub fn get_consensus_update_time(id: ConsensusClientId) -> Option<u64> {
        ConsensusClientUpdateTime::<T>::get(id)
    }

    /// Return the challenge period
    pub fn get_challenge_period(id: ConsensusClientId) -> Option<u64> {
        ChallengePeriod::<T>::get(id)
    }

    /// Return latest timestamp on chain
    pub fn get_timestamp() -> Option<u64> {
        Some(<T::TimeProvider as UnixTime>::now().as_secs())
    }

    /// Return the latest height of the state machine
    pub fn get_latest_state_machine_height(id: StateMachineId) -> Option<u64> {
        Some(LatestStateMachineHeight::<T>::get(id))
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

    /// Insert a leaf into the mmr
    pub(crate) fn mmr_push(leaf: Leaf) -> Option<NodeIndex> {
        let offchain_key = match &leaf {
            Leaf::Request(req) => Pallet::<T>::request_leaf_index_offchain_key(
                req.source_chain(),
                req.dest_chain(),
                req.nonce(),
            ),
            Leaf::Response(res) => Pallet::<T>::response_leaf_index_offchain_key(
                res.dest_chain(),
                res.source_chain(),
                res.nonce(),
            ),
        };
        let leaves = Self::number_of_leaves();
        let mmr: Mmr<mmr::storage::RuntimeStorage, T> = Mmr::new(leaves);
        let pos = mmr.push(leaf)?;
        Pallet::<T>::store_leaf_index_offchain(offchain_key, pos);
        Some(pos)
    }
}

impl<T: Config> Pallet<T> {
    /// Get a node from runtime storage
    fn get_node(pos: NodeIndex) -> Option<DataOrHash<T>> {
        Nodes::<T>::get(pos).map(DataOrHash::Hash)
    }

    /// Remove a node from storage
    fn remove_node(pos: NodeIndex) {
        Nodes::<T>::remove(pos);
    }

    /// Insert a node into storage
    fn insert_node(pos: NodeIndex, node: <T as frame_system::Config>::Hash) {
        Nodes::<T>::insert(pos, node)
    }

    /// Returns the number of leaves in the mmr
    fn get_num_leaves() -> LeafIndex {
        NumberOfLeaves::<T>::get()
    }

    /// Set the number of leaves in the mmr
    fn set_num_leaves(num_leaves: LeafIndex) {
        NumberOfLeaves::<T>::put(num_leaves)
    }

    /// Returns the offchain key for an index
    fn offchain_key(pos: NodeIndex) -> Vec<u8> {
        (T::INDEXING_PREFIX, "leaves", pos).encode()
    }
}
