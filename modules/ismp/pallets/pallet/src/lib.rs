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

#![doc = include_str!("../README.md")]
// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;
extern crate core;

pub mod dispatcher;
pub mod errors;
pub mod events;
pub mod host;
pub mod mmr;
pub use mmr::ProofKeys;
pub mod child_trie;
mod impls;
pub mod primitives;
pub mod weight_info;

pub use sp_mmr_primitives::utils::NodesUtils;

use crate::host::Host;
use codec::{Decode, Encode};
use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    traits::Get,
};
use sp_core::H256;
// Re-export pallet items so that they can be accessed from the crate namespace.
use crate::{mmr::Leaf, weight_info::get_weight};
use frame_system::pallet_prelude::BlockNumberFor;
use ismp::host::IsmpHost;
use mmr_primitives::{MerkleMountainRangeTree, NoOpTree};
pub use pallet::*;
#[cfg(feature = "unsigned")]
use sp_runtime::transaction_validity::{
    InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError,
    ValidTransaction,
};
use sp_std::prelude::*;

/// No-op mmr implementation for runtimes that don't want to build an offchain mmr tree. This
/// implementation does not panic for any runtime called methods, eg `push` or `finalize`
/// It will always return the default values for those methods.
///
/// *NOTE* it will return an error if you try to generate proofs.
pub type NoOpMmrTree = NoOpTree<Leaf>;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[frame_support::pallet]
pub mod pallet {
    use self::primitives::IsmpConsensusLog;
    use super::*;
    use crate::{
        child_trie::CHILD_TRIE_PREFIX,
        errors::HandlingError,
        primitives::{ConsensusClientProvider, ISMP_ID},
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
        messaging::{CreateConsensusState, Message},
        router::IsmpRouter,
    };
    use sp_core::{storage::ChildInfo, H256};

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Admin origin for privileged actions such as adding new consensus clients as well as
        /// modifying existing consensus clients (eg. challenge period, unbonding period)
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// The state machine identifier for the host chain. This is the identifier that will be
        /// used to accept requests that are addressed to this state machine. Remote chains
        /// will also use this identifier to accept requests originating from this state
        /// machine.
        type HostStateMachine: Get<StateMachine>;

        /// The coprocessor state machine which proxies requests on our behalf. The coprocessor in
        /// question performs costly consensus proof verification needed to verify
        /// requests/responses that are addressed to this host state machine. Before then
        /// providing cheap proofs of consensus and state needed to verify the legitimacy of
        /// the requests
        type Coprocessor: Get<Option<StateMachine>>;

        /// Timestamp provider, for various checks within the ISMP subsystems. This should be the
        /// pallet_timestamp::Pallet.
        type TimestampProvider: UnixTime;

        /// Router implementation for routing requests & responses to their appropriate modules.
        type Router: IsmpRouter + Default;

        /// Consenus clients which should be used to validate incoming requests or responses. There
        /// should be at least one consensus client present to allow messages be processed by the
        /// ISMP subsystems,
        type ConsensusClients: ConsensusClientProvider;

        /// This implementation should provide the weight consumed by `IsmpModule` callbacks from
        /// their benchmarks.
        type WeightProvider: WeightProvider;

        /// Merkle mountain range overlay tree implementation. Outgoing requests and responses are
        /// inserted in this "overlay tree" to enable cheap proofs for messages.
        ///
        /// State machines that do not need this can simply use the `NoOpMmrTree`
        type Mmr: MerkleMountainRangeTree<Leaf = Leaf>;
    }

    // Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
    // method.
    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Holds a map of state machine heights to their verified state commitments. These state
    /// commitments end up here after they are successfully verified by a `ConsensusClient`
    #[pallet::storage]
    #[pallet::getter(fn state_commitments)]
    pub type StateCommitments<T: Config> =
        StorageMap<_, Blake2_128Concat, StateMachineHeight, StateCommitment, OptionQuery>;

    /// Holds a map of consensus state identifiers to their consensus state.
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

    /// A mapping of consensus state identifier to it's associated consensus client identifier
    #[pallet::storage]
    pub type ConsensusStateClient<T: Config> =
        StorageMap<_, Blake2_128Concat, ConsensusStateId, ConsensusClientId, OptionQuery>;

    /// A mapping of consensus state identifiers to their unbonding periods
    #[pallet::storage]
    pub type UnbondingPeriod<T: Config> =
        StorageMap<_, Blake2_128Concat, ConsensusStateId, u64, OptionQuery>;

    /// A mapping of consensus state identifiers to their challenge periods
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

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(_n: BlockNumberFor<T>) {
            // Only finalize if mmr was modified
            let root = match T::Mmr::finalize() {
                Ok(root) => root,
                Err(e) => {
                    log::error!(target:"ismp", "Failed to finalize MMR {e:?}");
                    return
                },
            };

            let child_trie_root = frame_support::storage::child::root(
                &ChildInfo::new_default(CHILD_TRIE_PREFIX),
                Default::default(),
            );

            let log = IsmpConsensusLog {
                child_trie_root: H256::from_slice(&child_trie_root),
                mmr_root: root.into(),
            };

            let digest = sp_runtime::generic::DigestItem::Consensus(ISMP_ID, log.encode());
            <frame_system::Pallet<T>>::deposit_log(digest);
        }
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
        /// Execute the provided batch of ISMP messages, this will short-circuit and revert if any
        /// of the provided messages are invalid. This is an unsigned extrinsic that permits anyone
        /// execute ISMP messages for free, provided they have valid proofs and the messages have
        /// not been previously processed.
        ///
        /// The dispatch origin for this call must be an unsigned one.
        ///
        /// - `messages`: the messages to handle or process.
        ///
        /// Emits different message events based on the Message received if successful.
        #[cfg(feature = "unsigned")]
        #[pallet::weight(get_weight::<T>(&messages))]
        #[pallet::call_index(0)]
        #[frame_support::transactional]
        pub fn handle_unsigned(
            origin: OriginFor<T>,
            messages: Vec<Message>,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            Self::handle_messages(messages)
        }

        /// Execute the provided batch of ISMP messages. This call will short-circuit and revert if
        /// any of the provided messages are invalid.
        ///
        /// The dispatch origin for this call must be an unsigned one.
        ///
        /// - `messages`: the messages to handle or process.
        ///
        /// Emits different message events based on the Message received if successful.
        #[cfg(not(feature = "unsigned"))]
        #[pallet::weight(get_weight::<T>(&messages))]
        #[pallet::call_index(1)]
        #[frame_support::transactional]
        pub fn handle(origin: OriginFor<T>, messages: Vec<Message>) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            Self::handle_messages(messages)
        }

        /// Create a consensus client, using a subjectively chosen consensus state. This can also
        /// be used to overwrite an existing consensus state. The dispatch origin for this
        /// call must be `T::AdminOrigin`.
        ///
        /// - `message`: `CreateConsensusState` struct.
        ///
        /// Emits `ConsensusClientCreated` if successful.
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
        #[pallet::call_index(2)]
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

        /// Modify the unbonding period and challenge period for a consensus state.
        /// The dispatch origin for this call must be `T::AdminOrigin`.
        ///
        /// - `message`: `UpdateConsensusState` struct.
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(2))]
        #[pallet::call_index(3)]
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
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Emitted when a state machine is successfully updated to a new height
        StateMachineUpdated {
            /// State machine identifier
            state_machine_id: StateMachineId,
            /// State machine latest height
            latest_height: u64,
        },
        /// Emitted when a state commitment is vetoed by a fisherman
        StateCommitmentVetoed {
            /// State machine height
            height: StateMachineHeight,
            /// responsible fisherman
            fisherman: BoundedVec<u8, ConstU32<32>>,
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

    /// This allows users execute ISMP datagrams for free. Use with caution.
    #[cfg(feature = "unsigned")]
    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        // empty pre-dispatch do we don't modify storage
        fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
            Ok(())
        }

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            use ismp::{
                handlers::MessageResult,
                messaging::{ConsensusMessage, FraudProofMessage, RequestMessage},
                router::Request,
                util::hash_request,
            };
            let messages = match call {
                Call::handle_unsigned { messages } => messages,
                _ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
            };

            let host = Host::<T>::default();
            let _ = messages
                .iter()
                .map(|msg| handlers::handle_incoming_message(&host, msg.clone()))
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
                // they should all have the same priority so they can be rejected
                priority: 100,
                // they are all self-contained batches that have no dependencies
                requires: vec![],
                // provides this unique hash of transactions
                provides: vec![msg_hash],
                // should only live for at most 10 blocks
                longevity: 25,
                // always propagate
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
