// Copyright (c) 2024 Polytope Labs.
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

//! # Pallet ISMP
//!
//!
//! The interoperable state machine protocol implementation for substrate-based chains. This pallet
//! provides the ability to
//!
//! * Track the finalized state of a remote state machine (blockchain) through the use of consensus
//!   proofs which attest to a finalized "state commitment".
//! * Execute incoming ISMP-compliant messages from a connected chain, through the use of state
//!   proofs which are verified through a known, previously finalized state commitment.
//! * Dispatch ISMP requests and responses to a connected chain.
//! * Funding in-flight messages (Request or Response)
//!
//!
//!
//! ## Overview
//!
//! The ISMP Pallet provides calls which alow for:
//!
//! * Creating consensus clients with their respective unbonding, challenge periods and any initial
//!   state machine commitments.
//! * Updating consensus clients metadata
//! * Executing ISMP-compliant Messages
//!
//! To use it in your runtime, you need to implement the ismp
//! [`pallet_ismp::Config`](pallet/trait.Config.html). The supported dispatchable functions are
//! documented in the [`pallet_ismp::Call`](pallet/enum.Call.html) enum.
//!
//!
//! ### Terminology
//!
//! * **ISMP:** Interoperable State Machine Protocol, is a framework for secure, cross-chain
//!   interoperability. Providing both messaging and state reading capabilities.
//! * **State Commitment:** This refers to a cryptographic commitment of an entire blockchain state,
//!   otherwise known as state root.
//! * **State Machine:** This refers to the blockchain itself, we identify blockchains as state
//!   machines.
//! * **Consensus State:** This is the minimum data required by consensus client to verify consensus
//!   proofs which attest to a newly finalized state.
//! * **Consensus Client:** This is an algorithm that verifies consensus proofs of a particular
//!   consensus mechanism.
//! * **Unbonding Period:** Refers to how long it takes for validators to unstake their funds from
//!   the connected chain.
//! * **Challenge Period:** A configurable value for how long to wait for state commitments to be
//!   challenged, before they can be used to verify incoming requests/responses.
//!
//! ### Dispatchable Functions
//!
//! * `handle` - Handles incoming ISMP messages.
//! * `handle_unsigned` Unsigned variant for handling incoming messages, enabled by `feature =
//!   ["unsigned"]`
//! * `create_consensus_client` - Handles creation of various properties for a particular consensus
//!   client. Can only be called by the `AdminOrigin`.
//! * `update_consensus_state` - Updates consensus client properties in storage. Can only be called
//!   by the `AdminOrigin`.
//! * `fund_message` - In cases where the initially provided relayer fees have now become
//!   insufficient, perhaps due to a transaction fee spike on the destination chain. Allows a user
//!   to add more funds to the message to be used for delivery and execution. Should never be called
//!   on a completed message.
//!
//! Please refer to the [`Call`](pallet/enum.Call.html) enum and its associated
//! variants for documentation on each function.
//!
//! ### Runtime Configuration
//!
//! The following example shows how to configure `pallet-ismp` in your runtime
//!
//! ```rust,ignore
//! use frame_support::parameter_types;
//! use frame_system::EnsureRoot;
//! use ismp::Error;
//! use pallet_ismp::NoOpMmrTree;
//! use ismp::host::StateMachine;
//! use ismp::module::IsmpModule;
//! use ismp::router::{IsmpRouter, Post, Response, Timeout};
//!
//! parameter_types! {
//!     // The hyperbridge parachain on Polkadot
//!     pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Polkadot(3367));
//!     // The host state machine of this pallet
//!     pub const HostStateMachine: StateMachine = StateMachine::Polkadot(1000); // your paraId here
//! }
//!
//! impl pallet_ismp::Config for Runtime {
//!     // configure the runtime event
//!     type RuntimeEvent = RuntimeEvent;
//!     // Permissioned origin who can create or update consensus clients
//!     type AdminOrigin = EnsureRoot<AccountId>;
//!     // The state machine identifier for this state machine
//!     type HostStateMachine = HostStateMachine;
//!     // The pallet_timestamp pallet
//!     type TimestampProvider = Timestamp;
//!     // The currency implementation that is offered to relayers
//!     type Currency = Balances;
//!     // The balance type for the currency implementation
//!     type Balance = Balance;
//!     // Router implementation for routing requests/responses to their respective modules
//!     type Router = Router;
//!     // Optional coprocessor for incoming requests/responses
//!     type Coprocessor = Coprocessor;
//!     // Supported consensus clients
//!     type ConsensusClients = (
//!         // as an example, the parachain consensus client
//!         ismp_parachain::ParachainConsensusClient<Runtime, IsmpParachain>,
//!     );
//!     // Optional merkle mountain range overlay tree, for cheaper outgoing request proofs.
//!     // You most likely don't need it, just use the `NoOpMmrTree`
//!     type Mmr = NoOpMmrTree;
//!     // Weight provider for local modules
//!     type WeightProvider = ();
//! }
//!
//! #[derive(Default)]
//! struct Router;
//!
//! impl IsmpRouter for Router {
//!     fn module_for_id(&self, id: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error> {
//!         let module = match id.as_slice() {
//!             YOUR_MODULE_ID => Box::new(YourModule::default()),
//!             _ => Err(Error::ModuleNotFound(id))?
//!         };
//!
//!         Ok(module)
//!     }
//! }
//!
//! /// Some custom module capable of processing some incoming/request or response.
//! /// This could also be a pallet itself.
//! #[derive(Default)]
//! struct YourModule;
//!
//! impl IsmpModule for YourModule {
//!     /// Called by the ISMP hanlder, to notify module of a new POST request
//!     /// the module may choose to respond immediately, or in a later block
//!     fn on_accept(&self, request: Post) -> Result<(), Error> {
//!         // do something useful with the request
//!         Ok(())
//!     }
//!
//!     /// Called by the ISMP hanlder, to notify module of a response to a previously
//!     /// sent out request
//!     fn on_response(&self, response: Response) -> Result<(), Error> {
//!         // do something useful with the response
//!         Ok(())
//!     }
//!
//!     /// Called by the ISMP hanlder, to notify module of requests that were previously
//!     /// sent but have now timed-out
//! 	fn on_timeout(&self, request: Timeout) -> Result<(), Error> {
//!         // revert any state changes that were made prior to dispatching the request
//!         Ok(())
//!     }
//! }
//! ```

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs, unused_imports)]

extern crate alloc;
extern crate core;

pub mod child_trie;
pub mod dispatcher;
pub mod errors;
pub mod events;
pub mod host;
mod impls;
pub mod mmr;
mod utils;
pub mod weights;

use crate::mmr::Leaf;
use mmr_primitives::{MerkleMountainRangeTree, NoOpTree};
// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

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
    use super::*;
    use crate::{
        child_trie::{RequestCommitments, ResponseCommitments, CHILD_TRIE_PREFIX},
        errors::HandlingError,
        weights::{get_weight, WeightProvider},
    };
    use codec::{Codec, Encode};
    use core::fmt::Debug;
    use frame_support::{
        dispatch::{DispatchResult, DispatchResultWithPostInfo},
        pallet_prelude::*,
        traits::{fungible::Mutate, tokens::Preservation, Get, UnixTime},
        PalletId,
    };
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use ismp::{
        consensus::{
            ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight,
            StateMachineId,
        },
        events::{RequestResponseHandled, TimeoutHandled},
        handlers,
        host::{IsmpHost, StateMachine},
        messaging::{CreateConsensusState, Message},
        router::IsmpRouter,
    };
    use sp_core::{storage::ChildInfo, H256};
    #[cfg(feature = "unsigned")]
    use sp_runtime::transaction_validity::{
        InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError,
        ValidTransaction,
    };
    use sp_runtime::{
        traits::{AccountIdConversion, AtLeast32BitUnsigned},
        FixedPointOperand,
    };
    use sp_std::prelude::*;
    pub use utils::*;

    /// [`PalletId`] where relayer fees will be collected
    pub const RELAYER_FEE_ACCOUNT: PalletId = PalletId(*b"ISMPFEES");

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Admin origin for privileged actions such as adding new consensus clients as well as
        /// modifying existing consensus clients (eg. challenge period, unbonding period)
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Timestamp interface [`UnixTime`] for querying the current timestamp. This is used within
        /// the various ISMP sub-protocols.
        type TimestampProvider: UnixTime;

        /// The balance of an account.
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Codec
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Debug
            + MaxEncodedLen
            + TypeInfo
            + FixedPointOperand;

        /// The currency that is offered to relayers as payment for request delivery
        /// and execution. This should ideally be a stablecoin of some kind to guarantee
        /// predictable and stable revenue for relayers.
        ///
        /// This can also be used with pallet-assets through the
        /// [ItemOf](frame_support::traits::tokens::fungible::ItemOf) implementation
        type Currency: Mutate<Self::AccountId, Balance = Self::Balance>;

        /// The state machine identifier for the host chain. This is the identifier that will be
        /// used to accept requests that are addressed to this state machine. Remote chains
        /// will also use this identifier to accept requests originating from this state
        /// machine.
        type HostStateMachine: Get<StateMachine>;

        /// The coprocessor is a state machine which proxies requests on our behalf. The coprocessor
        /// does this by performing the costly consensus and state proof verification needed to
        /// verify requests/responses that are addressed to this host state machine.
        ///
        /// The ISMP framework permits the coprocessor to aggregate messages from potentially
        /// multiple state machines. Finally producing much cheaper proofs of consensus and state
        /// needed to verify the legitimacy of the messages.
        type Coprocessor: Get<Option<StateMachine>>;

        /// [`IsmpRouter`] implementation for routing requests & responses to their appropriate
        /// modules.
        type Router: IsmpRouter + Default;

        /// This should provide a list of [`ConsenusClient`](ismp::consensus::ConsensusClient)s
        /// which should be used to validate incoming requests or responses. There should be
        /// at least one consensus client present to allow messages be processed by the ISMP
        /// subsystems.
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
    #[pallet::getter(fn challenge_period)]
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
    #[pallet::getter(fn latest_state_machine_height)]
    pub type LatestStateMachineHeight<T: Config> =
        StorageMap<_, Blake2_128Concat, StateMachineId, u64, OptionQuery>;

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

    /// The child trie root of messages
    #[pallet::storage]
    #[pallet::getter(fn child_trie_root)]
    pub type ChildTrieRoot<T: Config> =
        StorageValue<_, <T as frame_system::Config>::Hash, ValueQuery>;

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
    where
        <T as frame_system::Config>::Hash: From<H256>,
    {
        fn on_finalize(_n: BlockNumberFor<T>) {
            let child_trie_root = storage::child::root(
                &ChildInfo::new_default(CHILD_TRIE_PREFIX),
                Default::default(),
            );

            let child_trie_root = H256::from_slice(&child_trie_root);
            ChildTrieRoot::<T>::put::<<T as frame_system::Config>::Hash>(child_trie_root.into());
            // Only finalize if mmr was modified
            let root = match T::Mmr::finalize() {
                Ok(root) => root,
                Err(e) => {
                    log::error!(target:"ismp", "Failed to finalize MMR {e:?}");
                    return;
                },
            };

            let log = ConsensusDigest { child_trie_root, mmr_root: root.into() };

            let digest = sp_runtime::generic::DigestItem::Consensus(ISMP_ID, log.encode());
            <frame_system::Pallet<T>>::deposit_log(digest);
        }
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
        /// - `messages`: A set of ISMP [`Message`]s to handle or process.
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
        /// - `message`: [`CreateConsensusState`] struct.
        ///
        /// Emits [`Event::ConsensusClientCreated`] if successful.
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))]
        #[pallet::call_index(2)]
        pub fn create_consensus_client(
            origin: OriginFor<T>,
            message: CreateConsensusState,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            let host = Pallet::<T>::default();

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

            let host = Pallet::<T>::default();

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

        /// Add more funds to a message (request or response) to be used for delivery and execution.
        ///
        /// Should not be called on a message that has been completed (delivered or timed-out) as
        /// those funds will be lost forever.
        #[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(5))]
        #[pallet::call_index(4)]
        pub fn fund_message(
            origin: OriginFor<T>,
            message: FundMessageParams<T::Balance>,
        ) -> DispatchResult {
            let account = ensure_signed(origin)?;

            let metadata = match message.commitment {
                MessageCommitment::Request(commitment) => RequestCommitments::<T>::get(commitment),
                MessageCommitment::Response(commitment) =>
                    ResponseCommitments::<T>::get(commitment),
            };

            let Some(mut metadata) = metadata else {
                return Err(Error::<T>::MessageNotFound.into());
            };

            T::Currency::transfer(
                &account,
                &RELAYER_FEE_ACCOUNT.into_account_truncating(),
                message.amount,
                Preservation::Expendable,
            )?;

            match message.commitment {
                MessageCommitment::Request(commiment) => {
                    metadata.fee.fee += message.amount;
                    RequestCommitments::<T>::insert(commiment, metadata);
                },
                MessageCommitment::Response(commiment) => {
                    metadata.fee.fee += message.amount;
                    ResponseCommitments::<T>::insert(commiment, metadata);
                },
            };

            Ok(())
        }
    }

    /// Pallet Events
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
        /// Requested message was not found
        MessageNotFound,
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
                messaging::{hash_request, ConsensusMessage, FraudProofMessage, RequestMessage},
                router::Request,
            };
            let messages = match call {
                Call::handle_unsigned { messages } => messages,
                _ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
            };

            let host = Pallet::<T>::default();
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
                        .map(|post| hash_request::<Pallet<T>>(&Request::Post(post.clone())))
                        .collect::<Vec<_>>(),
                    Message::Response(message) => message
                        .requests()
                        .iter()
                        .map(|request| hash_request::<Pallet<T>>(request))
                        .collect::<Vec<_>>(),
                    Message::Timeout(message) => message
                        .requests()
                        .iter()
                        .map(|request| hash_request::<Pallet<T>>(request))
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

    // Hack for implementing the [`Default`] bound needed for
    // [`IsmpDispatcher`](ismp::dispatcher::IsmpDispatcher) and
    // [`IsmpModule`](ismp::module::IsmpModule)
    impl<T> Default for Pallet<T> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }
}
