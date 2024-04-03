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

pub mod dispatcher;
mod errors;
pub mod events;
pub mod handlers;
pub mod host;
pub mod mmr;
use events::deposit_ismp_events;
pub use mmr::ProofKeys;
pub mod child_trie;
pub mod primitives;
pub mod weight_info;

pub use mmr::utils::NodesUtils;

use crate::host::Host;
use codec::{Decode, Encode};
use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo, Pays, PostDispatchInfo},
    traits::Get,
};
use ismp::{
    consensus::{ConsensusClientId, StateMachineId},
    handlers::{handle_incoming_message, MessageResult},
    messaging::CreateConsensusState,
    router::{Request, Response},
    util::hash_request,
};
use log::debug;
use sp_core::{offchain::StorageKind, H256};
// Re-export pallet items so that they can be accessed from the crate namespace.
use crate::{
    errors::HandlingError,
    mmr::{
        primitives::{DataOrHash, Leaf, LeafIndex, NodeIndex},
        Mmr,
    },
    primitives::LeafIndexAndPos,
    weight_info::get_weight,
};
use frame_system::pallet_prelude::BlockNumberFor;
use ismp::{host::IsmpHost, messaging::Message};
use merkle_mountain_range::leaf_index_to_pos;
pub use pallet::*;
use sp_runtime::{
    traits::ValidateUnsigned,
    transaction_validity::{
        InvalidTransaction, TransactionLongevity, TransactionSource, TransactionValidity,
        TransactionValidityError, ValidTransaction,
    },
    RuntimeDebug,
};
use sp_std::prelude::*;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[frame_support::pallet]
pub mod pallet {

    use self::primitives::IsmpConsensusLog;

    // Import various types used to declare pallet in scope.
    use super::*;
    use crate::{
        child_trie::CHILD_TRIE_PREFIX,
        errors::HandlingError,
        mmr::primitives::{LeafIndex, NodeIndex},
        primitives::{ConsensusClientProvider, WeightUsed, ISMP_ID},
        weight_info::WeightProvider,
    };
    use frame_support::{pallet_prelude::*, traits::UnixTime};
    use frame_system::pallet_prelude::*;
    use ismp::{
        consensus::{
            ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
            StateMachineId,
        },
        events::{RequestResponseHandled, TimeoutHandled},
        handlers,
        host::StateMachine,
        messaging::{ConsensusMessage, FraudProofMessage, Message, RequestMessage},
        router::IsmpRouter,
    };
    use sp_core::{storage::ChildInfo, H256};

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Prefix for elements stored in the Off-chain DB via Indexing API
        const INDEXING_PREFIX: &'static [u8];

        /// Admin origin for privileged actions
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Host state machine identifier
        type HostStateMachine: Get<StateMachine>;

        /// The coprocessor state machine which proxies requests on our behalf
        type Coprocessor: Get<Option<StateMachine>>;

        /// Timestamp provider
        type TimeProvider: UnixTime;

        /// Configurable router that dispatches calls to modules
        type Router: IsmpRouter + Default;

        /// Provides concrete implementations of consensus clients
        type ConsensusClients: ConsensusClientProvider;

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
    pub type RootHash<T: Config> = StorageValue<_, H256, ValueQuery>;

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
    pub type Nodes<T: Config> = StorageMap<_, Identity, NodeIndex, H256, OptionQuery>;

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
    #[pallet::getter(fn frozen_state_machine)]
    pub type FrozenStateMachine<T: Config> =
        StorageMap<_, Blake2_128Concat, StateMachineId, bool, OptionQuery>;

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

    /// Holds the timestamp at which a consensus client was recently updated.
    /// Used in ensuring that the configured challenge period elapses.
    #[pallet::storage]
    #[pallet::getter(fn consensus_update_time)]
    pub type ConsensusClientUpdateTime<T: Config> =
        StorageMap<_, Twox64Concat, ConsensusClientId, u64, OptionQuery>;

    /// Holds the timestamp at which a state machine height was updated.
    /// Used in ensuring that the configured challenge period elapses.
    #[pallet::storage]
    #[pallet::getter(fn state_machine_update_time)]
    pub type StateMachineUpdateTime<T: Config> =
        StorageMap<_, Twox64Concat, StateMachineHeight, u64, OptionQuery>;

    /// Tracks requests that have been responded to
    /// The key is the request commitment
    #[pallet::storage]
    #[pallet::getter(fn responded)]
    pub type Responded<T: Config> = StorageMap<_, Identity, H256, bool, ValueQuery>;

    /// Latest nonce for messages sent from this chain
    #[pallet::storage]
    #[pallet::getter(fn nonce)]
    pub type Nonce<T> = StorageValue<_, u64, ValueQuery>;

    /// Contains a tuple of the weight consumed and weight limit in executing contract callbacks in
    /// a transaction
    #[pallet::storage]
    #[pallet::getter(fn weight_consumed)]
    pub type WeightConsumed<T: Config> = StorageValue<_, WeightUsed, ValueQuery>;

    /// Mmr positions to commitments
    #[pallet::storage]
    #[pallet::getter(fn mmr_positions)]
    pub type MmrPositions<T: Config> =
        StorageMap<_, Blake2_128Concat, NodeIndex, H256, OptionQuery>;

    /// Temporary leaf storage for when the block is still executing
    #[pallet::storage]
    #[pallet::getter(fn intermediate_leaves)]
    pub type IntermediateLeaves<T: Config> =
        CountedStorageMap<_, Blake2_128Concat, NodeIndex, Leaf, OptionQuery>;

    /// Temporary store to increment the leaf index as the block is executed
    #[pallet::storage]
    #[pallet::getter(fn intermediate_number_of_leaves)]
    pub type IntermediateNumberOfLeaves<T> = StorageValue<_, LeafIndex, ValueQuery>;

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            IntermediateNumberOfLeaves::<T>::put(Self::number_of_leaves());
            Weight::default()
        }

        fn on_finalize(_n: BlockNumberFor<T>) {
            // Only finalize if mmr was modified
            let leaves = Self::intermediate_number_of_leaves();
            let root = if leaves != 0 {
                let mut mmr: Mmr<mmr::storage::RuntimeStorage, T> =
                    Mmr::new(Self::number_of_leaves());
                let range = Self::number_of_leaves()..leaves;
                for index in range {
                    let leaf = IntermediateLeaves::<T>::get(index)
                        .expect("Infallible: Leaf was inserted in this block");
                    // Mmr push should never fail
                    match mmr.push(leaf) {
                        None => {
                            log::error!(target: "ismp::mmr", "MMR push failed ");
                        },
                        Some(position) => {
                            log::trace!(target: "ismp::mmr", "MMR push {position}");
                        },
                    }
                }

                // Update the size, `mmr.commit()` should also never fail.
                match mmr.commit() {
                    Ok(_) => {
                        log::trace!(target: "ismp::mmr", "Committed to mmr, No of leaves: {leaves}");
                    },
                    Err(e) => {
                        log::error!(target: "ismp::mmr", "MMR finalize failed: {:?}", e);
                        return;
                    },
                }

                // Calculate the mmr's new root
                let mmr: Mmr<mmr::storage::RuntimeStorage, T> = Mmr::new(leaves);
                // Update the size, `mmr.finalize()` should also never fail.
                let root = match mmr.finalize() {
                    Ok(root) => root,
                    Err(e) => {
                        log::error!(target: "ismp::mmr", "MMR finalize failed: {:?}", e);
                        return;
                    },
                };

                // Insert root in storage
                <RootHash<T>>::put(root);
                let total = IntermediateLeaves::<T>::count();
                // Clear intermediate values
                let _ = IntermediateLeaves::<T>::clear(total, None);
                IntermediateNumberOfLeaves::<T>::kill();
                root
            } else {
                H256::default()
            };

            let child_trie_root = frame_support::storage::child::root(
                &ChildInfo::new_default(CHILD_TRIE_PREFIX),
                Default::default(),
            );

            let log = IsmpConsensusLog {
                child_trie_root: H256::from_slice(&child_trie_root),
                mmr_root: root,
            };

            let digest = sp_runtime::generic::DigestItem::Consensus(ISMP_ID, log.encode());
            <frame_system::Pallet<T>>::deposit_log(digest);
        }

        fn offchain_worker(_n: BlockNumberFor<T>) {}
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
    impl<T: Config> Pallet<T> {
        /// Handles ismp messages
        #[pallet::weight(get_weight::<T>(&messages))]
        #[pallet::call_index(0)]
        #[frame_support::transactional]
        pub fn handle(origin: OriginFor<T>, messages: Vec<Message>) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            Self::handle_messages(messages)
        }

        /// Create a consensus client, using a subjectively chosen consensus state.
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
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

        /// This is for validating if an ISMP message would succeed through dry_run, prefer to use
        /// [`Call::handle`] over this.
        #[pallet::weight(get_weight::<T>(&messages))]
        #[pallet::call_index(3)]
        pub fn validate_messages(origin: OriginFor<T>, messages: Vec<Message>) -> DispatchResult {
            ensure_none(origin)?;

            <Pallet<T> as ValidateUnsigned>::validate_unsigned(
                TransactionSource::External,
                &Call::handle { messages },
            )
            .map_err(|_err| Error::<T>::InvalidMessage)?;

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
        /// Indicates that a consensus client has been created
        ConsensusClientCreated {
            /// Consensus client id
            consensus_client_id: ConsensusClientId,
        },
        /// Indicates that a consensus client has been created
        ConsensusClientFrozen {
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
            /// Commitment
            commitment: H256,
        },
        /// An Outgoing Request has been deposited
        Request {
            /// Chain that this request will be routed to
            dest_chain: StateMachine,
            /// Source Chain for request
            source_chain: StateMachine,
            /// Request nonce
            request_nonce: u64,
            /// Commitment
            commitment: H256,
        },
        /// Some errors handling some ismp messages
        Errors {
            /// Message handling errors
            errors: Vec<HandlingError>,
        },
        /// Post Request Handled
        PostRequestHandled(RequestResponseHandled),
        /// Post Response Handled
        PostResponseHandled(RequestResponseHandled),
        /// Get Response Handled
        GetRequestHandled(RequestResponseHandled),
        /// Post request timeout handled
        PostRequestTimeoutHandled(TimeoutHandled),
        /// Post response timeout handled
        PostResponseTimeoutHandled(TimeoutHandled),
        /// Get request timeout handled
        GetRequestTimeoutHandled(TimeoutHandled),
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

    /// Users should not pay to submit valid ISMP datagrams.
    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        // empty pre-dispatch do we don't modify storage
        fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
            Ok(())
        }

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            let messages = match call {
                Call::handle { messages } | Call::validate_messages { messages } => messages,
                _ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
            };

            let host = Host::<T>::default();
            let _ = messages
                .iter()
                .map(|msg| handle_incoming_message(&host, msg.clone()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_err| {
                    log::info!(target: "ismp", "Validation Errors: {:#?}", _err);
                    TransactionValidityError::Invalid(InvalidTransaction::BadProof)
                })?
                .into_iter()
                // check that requests will be successfully dispatched
                // so we can not be spammed with failing txs
                .map(|result| match result {
                    MessageResult::Request(results) |
                    MessageResult::Response(results) |
                    MessageResult::Timeout(results) =>
                        results.into_iter().map(|result| result.map(|_| ())).collect::<Vec<_>>(),
                    MessageResult::ConsensusMessage(_) | MessageResult::FrozenClient(_) => {
                        vec![Ok(())]
                    },
                })
                .flatten()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_err| {
                    log::info!(target: "ismp", "Validation Errors: {:#?}", _err);
                    TransactionValidityError::Invalid(InvalidTransaction::BadProof)
                })?;

            let mut requests = messages
                .into_iter()
                .map(|message| match message {
                    Message::Consensus(ConsensusMessage { consensus_proof, .. }) => {
                        vec![H256(sp_io::hashing::keccak_256(&consensus_proof))]
                    },
                    Message::FraudProof(FraudProofMessage { proof_1, proof_2, .. }) => vec![
                        H256(sp_io::hashing::keccak_256(&proof_1)),
                        H256(sp_io::hashing::keccak_256(&proof_2)),
                    ],
                    Message::Request(RequestMessage { requests, .. }) => requests
                        .into_iter()
                        .map(|post| hash_request::<Host<T>>(&Request::Post(post.clone())))
                        .collect::<Vec<_>>(),
                    Message::Response(message) => message
                        .requests()
                        .iter()
                        .map(|request| hash_request::<Host<T>>(request))
                        .collect::<Vec<_>>(),
                    Message::Timeout(message) => message
                        .requests()
                        .iter()
                        .map(|request| hash_request::<Host<T>>(request))
                        .collect::<Vec<_>>(),
                })
                .collect::<Vec<_>>();
            requests.sort();

            // this is so we can reject duplicate batches at the mempool level
            let msg_hash = sp_io::hashing::keccak_256(&requests.encode()).to_vec();

            Ok(ValidTransaction {
                priority: 100,
                requires: vec![],
                provides: vec![msg_hash],
                longevity: TransactionLongevity::MAX,
                propagate: true,
            })
        }
    }
}

/// Receipt for a Response
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct ResponseReceipt {
    /// Hash of the response object
    pub response: H256,
    /// Address of the relayer
    pub relayer: Vec<u8>,
}

/// Digest log for mmr root hash
#[derive(RuntimeDebug, Encode, Decode)]
pub struct RequestResponseLog<T: Config> {
    /// The mmr root hash
    mmr_root_hash: <T as frame_system::Config>::Hash,
}

impl<T: Config> Pallet<T> {
    /// Generate an MMR proof for the given `leaf_indices`.
    /// Note this method can only be used from an off-chain context
    /// (Offchain Worker or Runtime API call), since it requires
    /// all the leaves to be present.
    /// It may return an error or panic if used incorrectly.
    pub fn generate_proof(
        commitments: ProofKeys,
    ) -> Result<(Vec<Leaf>, primitives::Proof<H256>), primitives::Error> {
        let leaves_count = NumberOfLeaves::<T>::get();
        let mmr = Mmr::<mmr::storage::OffchainStorage, T>::new(leaves_count);
        mmr.generate_proof(commitments)
    }

    /// Provides a way to handle messages.
    pub fn handle_messages(messages: Vec<Message>) -> DispatchResultWithPostInfo {
        // Define a host
        WeightConsumed::<T>::kill();
        let host = Host::<T>::default();
        let mut errors: Vec<HandlingError> = vec![];
        let total_weight = get_weight::<T>(&messages);
        for message in messages {
            match handle_incoming_message(&host, message.clone()) {
                Ok(MessageResult::ConsensusMessage(res)) => deposit_ismp_events::<T>(
                    res.into_iter().map(|ev| Ok(ev)).collect(),
                    &mut errors,
                ),
                Ok(MessageResult::Response(res)) => deposit_ismp_events::<T>(res, &mut errors),
                Ok(MessageResult::Request(res)) => deposit_ismp_events::<T>(res, &mut errors),
                Ok(MessageResult::Timeout(res)) => deposit_ismp_events::<T>(res, &mut errors),
                Ok(MessageResult::FrozenClient(id)) =>
                    Self::deposit_event(Event::<T>::ConsensusClientFrozen {
                        consensus_client_id: id,
                    }),
                Err(err) => {
                    errors.push(err.into());
                },
            }
        }

        if !errors.is_empty() {
            debug!(target: "ismp", "Handling Errors {:?}", errors);
            Self::deposit_event(Event::<T>::Errors { errors })
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
    pub fn mmr_root() -> H256 {
        Self::mmr_root_hash()
    }

    /// Return mmr leaf count
    pub fn mmr_leaf_count() -> LeafIndex {
        Self::number_of_leaves()
    }
    /// Get a node from runtime storage
    fn get_node(pos: NodeIndex) -> Option<DataOrHash> {
        Nodes::<T>::get(pos).map(DataOrHash::Hash)
    }

    /// Insert a node into storage
    fn insert_node(pos: NodeIndex, node: H256) {
        Nodes::<T>::insert(pos, node)
    }

    /// Set the number of leaves in the mmr
    fn set_num_leaves(num_leaves: LeafIndex) {
        NumberOfLeaves::<T>::put(num_leaves)
    }

    /// Returns the offchain key for a request or response leaf index
    pub fn full_leaf_offchain_key(commitment: H256) -> Vec<u8> {
        (T::INDEXING_PREFIX, commitment).encode()
    }

    /// Gets the request from the offchain storage
    pub fn get_request(commitment: H256) -> Option<Request> {
        let key = Pallet::<T>::full_leaf_offchain_key(commitment);
        if let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
            let data_or_hash = DataOrHash::decode(&mut &*elem).ok()?;
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

    /// Gets the response from the offchain storage
    pub fn get_response(commitment: H256) -> Option<Response> {
        let key = Pallet::<T>::full_leaf_offchain_key(commitment);
        if let Some(elem) = sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &key) {
            let data_or_hash = DataOrHash::decode(&mut &*elem).ok()?;
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

    /// Return the latest height of the state machine
    pub fn get_latest_state_machine_height(id: StateMachineId) -> Option<u64> {
        Some(LatestStateMachineHeight::<T>::get(id))
    }

    /// Get actual requests
    pub fn get_requests(commitments: Vec<H256>) -> Vec<Request> {
        commitments.into_iter().filter_map(|cm| Self::get_request(cm)).collect()
    }

    /// Get actual requests
    pub fn get_responses(commitments: Vec<H256>) -> Vec<Response> {
        commitments.into_iter().filter_map(|cm| Self::get_response(cm)).collect()
    }

    /// Insert a leaf into the mmr and return the position and leaf index
    pub(crate) fn mmr_push(leaf: Leaf) -> Option<LeafIndexAndPos> {
        let commitment = leaf.hash::<Host<T>>();
        let leaf_index = Pallet::<T>::intermediate_number_of_leaves();
        IntermediateLeaves::<T>::insert(leaf_index, leaf);
        let pos = leaf_index_to_pos(leaf_index);
        IntermediateNumberOfLeaves::<T>::put(leaf_index + 1);
        MmrPositions::<T>::insert(pos, commitment);
        Some(LeafIndexAndPos { pos, leaf_index })
    }
}
