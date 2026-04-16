// Copyright (c) 2025 Polytope Labs.
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
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unused_imports)]

extern crate alloc;
extern crate core;

pub mod child_trie;
pub mod dispatcher;
pub mod errors;
pub mod events;
pub mod fee_handler;
pub mod host;
mod impls;
pub mod offchain;
mod utils;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
use crate::offchain::Leaf;
use offchain::OffchainDBProvider;
use polkadot_sdk::*;
// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// State of one of the multi-block legacy storage drains. `Active(None)` is the
/// initial state and means the next call should pass `None` as the `clear()`
/// cursor. `Active(Some(cursor))` carries the cursor returned by the previous
/// call so the next one resumes from where it stopped. `Done` is set when
/// `clear()` returns `None`, meaning the prefix has been fully cleared; subsequent
/// `on_idle` calls become a single cheap read for that target.
#[derive(
	codec::Encode,
	codec::Decode,
	codec::DecodeWithMemTracking,
	codec::MaxEncodedLen,
	scale_info::TypeInfo,
	core::fmt::Debug,
	Clone,
	PartialEq,
	Eq,
)]
pub enum LegacyDrainState {
	/// Drain in progress. `None` starts from the beginning of the prefix;
	/// `Some(cursor)` resumes from `cursor`.
	Active(
		Option<polkadot_sdk::frame_support::BoundedVec<u8, polkadot_sdk::sp_core::ConstU32<1024>>>,
	),
	/// Drain complete; nothing left to clear.
	Done,
}

impl Default for LegacyDrainState {
	fn default() -> Self {
		Self::Active(None)
	}
}

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	use crate::{
		child_trie::{
			RequestCommitments, ResponseCommitments, CHILD_TRIE_PREFIX, STATE_COMMITMENTS_KEY,
		},
		errors::HandlingError,
		fee_handler::FeeHandler,
	};
	use codec::{Codec, Encode};
	use core::fmt::Debug;
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{fungible::Mutate, tokens::Preservation, Get, UnixTime},
		BoundedBTreeSet, PalletId,
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
	use sp_runtime::{
		traits::{AccountIdConversion, AtLeast32BitUnsigned},
		transaction_validity::{
			InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError,
			ValidTransaction,
		},
		FixedPointOperand,
	};
	use sp_std::prelude::*;
	pub use utils::*;

	/// [`PalletId`] where relayer fees will be collected
	pub const RELAYER_FEE_ACCOUNT: PalletId = PalletId(*b"ISMPFEES");

	/// Maximum state commitments retained per chain in [`BoundedStateCommitments`].
	pub const MAX_STATE_MACHINE_COMMITMENTS: u32 = 256;

	frame_support::parameter_types! {
		/// Type-level `Get<u32>` mirror of [`MAX_STATE_MACHINE_COMMITMENTS`], used as
		/// the bound for the per-chain [`KnownStateMachineHeights`] pointer set.
		pub const MaxStateMachineCommitments: u32 = MAX_STATE_MACHINE_COMMITMENTS;
	}

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
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

		/// Fee handling implementation for ISMP message processing.
		///
		/// This type defines how fees are calculated and settled for different ISMP message types.
		/// It provides an extensible way to implement various fee models based on chain-specific
		/// requirements, including:
		///
		/// - Weight-based fee calculations for computational resources
		/// - Custom economic incentives for relayers and validators
		/// - Different fee structures for various message types (requests, responses, consensus)
		/// - Support for subsidized operations or negative fee models
		///
		/// The chosen implementation determines how transaction fees are calculated when
		/// processing ISMP messages, directly affecting the economic sustainability of the
		/// cross-chain messaging system.
		type FeeHandler: FeeHandler;

		/// Offchain database implementation. Outgoing requests and responses are
		/// inserted in this database, while their commitments are stored onchain.
		///
		/// This offchain DB is also allowed to "merkelize" and "generate proofs" for messages.
		/// Most state machines will likey not need this and can just provide `()`
		type OffchainDB: OffchainDBProvider<Leaf = Leaf>;

		/// Weight functions for legacy storage drain migrations.
		type MigrationWeightInfo: crate::weights::MigrationWeightInfo;
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

	/// A mapping of consensus state identifier to it's associated consensus client identifier
	#[pallet::storage]
	pub type ConsensusStateClient<T: Config> =
		StorageMap<_, Blake2_128Concat, ConsensusStateId, ConsensusClientId, OptionQuery>;

	/// A mapping of consensus state identifiers to their unbonding periods
	#[pallet::storage]
	pub type UnbondingPeriod<T: Config> =
		StorageMap<_, Blake2_128Concat, ConsensusStateId, u64, OptionQuery>;

	/// A mapping of state machine Ids to their challenge periods
	#[pallet::storage]
	#[pallet::getter(fn challenge_period)]
	pub type ChallengePeriod<T: Config> =
		StorageMap<_, Blake2_128Concat, StateMachineId, u64, OptionQuery>;

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

	/// The previous verified height for a state machine
	#[pallet::storage]
	#[pallet::getter(fn previous_state_machine_height)]
	pub type PreviousStateMachineHeight<T: Config> =
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

	/// Bounded map of state machine commitments. Keyed by
	/// `(StateMachineId, height)` so we can cap entries per chain at
	/// [`MAX_STATE_MACHINE_COMMITMENTS`]. Writes go here; the legacy
	/// [`StateCommitments`] map is drained gradually.
	#[pallet::storage]
	pub type BoundedStateCommitments<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StateMachineId,
		Blake2_128Concat,
		u64,
		StateCommitment,
		OptionQuery,
	>;

	/// Bounded map of state machine update timestamps. Same per-chain cap
	/// as [`BoundedStateCommitments`]. The legacy [`StateMachineUpdateTime`]
	/// is drained gradually.
	#[pallet::storage]
	pub type BoundedStateMachineUpdateTime<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StateMachineId,
		Blake2_128Concat,
		u64,
		u64,
		OptionQuery,
	>;

	/// Per-chain sorted pointer set of heights currently held in
	/// [`BoundedStateCommitments`] and [`BoundedStateMachineUpdateTime`].
	/// `BoundedStateCommitments` uses `Blake2_128Concat` for its inner key, so
	/// `iter_key_prefix(chain).next()` yields hash order rather than numeric
	/// order and cannot identify the oldest height. Iterating this set does,
	/// because `BTreeSet` is sorted. Every write to the bounded maps for a
	/// given chain must keep this set in sync.
	#[pallet::storage]
	pub type KnownStateMachineHeights<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		StateMachineId,
		BoundedBTreeSet<u64, MaxStateMachineCommitments>,
		ValueQuery,
	>;

	/// State of the multi-block drain of the legacy [`StateCommitments`] map.
	/// See [`super::LegacyDrainState`].
	#[pallet::storage]
	pub type LegacyStateCommitmentsDrainState<T: Config> =
		StorageValue<_, super::LegacyDrainState, ValueQuery>;

	/// State of the multi-block drain of the legacy [`StateMachineUpdateTime`] map.
	/// See [`super::LegacyDrainState`].
	#[pallet::storage]
	pub type LegacyStateMachineUpdateTimeDrainState<T: Config> =
		StorageValue<_, super::LegacyDrainState, ValueQuery>;

	/// State of the multi-block drain of the legacy state-commitment entries
	/// stored in the ISMP child trie. The cursor is the last child-trie key we
	/// processed; on the next call we resume with `next_key(last)`. `Done` short-
	/// circuits subsequent `on_idle` calls.
	#[pallet::storage]
	pub type LegacyChildTrieDrainState<T: Config> =
		StorageValue<_, super::LegacyDrainState, ValueQuery>;

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
			let state_version = <T as polkadot_sdk::frame_system::Config>::Version::get()
				.state_version()
				.try_into()
				.unwrap_or_default();

			let child_trie_root =
				storage::child::root(&ChildInfo::new_default(CHILD_TRIE_PREFIX), state_version);

			let child_trie_root = H256::from_slice(&child_trie_root);
			ChildTrieRoot::<T>::put::<T::Hash>(child_trie_root.into());
			let root = match T::OffchainDB::finalize() {
				Ok(root) => root,
				Err(e) => {
					log::error!(target:"ismp", "Failed to finalize MMR {e:?}");
					return;
				},
			};

			let log = ConsensusDigest { child_trie_root, mmr_root: root.into() };
			let digest = sp_runtime::generic::DigestItem::Consensus(ISMP_ID, log.encode());
			<frame_system::Pallet<T>>::deposit_log(digest);

			let timestamp_secs = T::TimestampProvider::now().as_secs();
			let timestamp_log = TimestampDigest { timestamp: timestamp_secs };
			let timestamp_digest = sp_runtime::generic::DigestItem::Consensus(
				ISMP_TIMESTAMP_ID,
				timestamp_log.encode(),
			);
			<frame_system::Pallet<T>>::deposit_log(timestamp_digest);
		}

		/// Drains the legacy [`StateCommitments`], [`StateMachineUpdateTime`], and child
		/// trie state commitment entries using leftover block weight. Runs after all
		/// extrinsics, so it never blocks user transactions.
		///
		/// Each drain target persists its `clear()` cursor in storage between blocks
		/// so the next call resumes where the previous one stopped. Once `clear()`
		/// returns no cursor (or no more child-trie keys exist) we mark that target
		/// as `Done`, which short-circuits subsequent `on_idle` calls to a single
		/// cheap storage read.
		fn on_idle(_n: BlockNumberFor<T>, remaining: Weight) -> Weight {
			use crate::weights::MigrationWeightInfo;
			// We charge weight for `STATE_BATCH`/`CHILD_TRIE_BATCH` up front but only
			// remove `*_BATCH - SAFETY_BUFFER` entries per call. The buffer absorbs
			// any imprecision in the benchmarked weights so a single step never
			// exceeds its charged budget.
			const SAFETY_BUFFER: u32 = 10;
			const STATE_BATCH: u32 = 500;
			const STATE_DRAIN: u32 = STATE_BATCH - SAFETY_BUFFER;
			const CHILD_TRIE_BATCH: u32 = 200;
			const CHILD_TRIE_DRAIN: u32 = CHILD_TRIE_BATCH - SAFETY_BUFFER;

			let mut consumed = Weight::zero();
			let mut budget = remaining;

			// ── Drain StateCommitments ────────────────────────────────────
			let sc_state = LegacyStateCommitmentsDrainState::<T>::get();
			if let super::LegacyDrainState::Active(cursor) = sc_state {
				let sc_required =
					<T as Config>::MigrationWeightInfo::drain_state_commitments_step(STATE_BATCH);
				if budget.all_gte(sc_required) {
					let result = StateCommitments::<T>::clear(
						STATE_DRAIN,
						cursor.as_ref().map(|v| v.as_slice()),
					);
					let new_state = match result.maybe_cursor {
						Some(c) => match polkadot_sdk::frame_support::BoundedVec::try_from(c) {
							Ok(b) => super::LegacyDrainState::Active(Some(b)),
							Err(_) => super::LegacyDrainState::Active(None),
						},
						None => super::LegacyDrainState::Done,
					};
					LegacyStateCommitmentsDrainState::<T>::put(new_state);
					let used = <T as Config>::MigrationWeightInfo::drain_state_commitments_step(
						result.unique,
					);
					consumed = consumed.saturating_add(used);
					budget = budget.saturating_sub(used);
				}
			}

			// ── Drain StateMachineUpdateTime ──────────────────────────────
			let smu_state = LegacyStateMachineUpdateTimeDrainState::<T>::get();
			if let super::LegacyDrainState::Active(cursor) = smu_state {
				let smu_required =
					<T as Config>::MigrationWeightInfo::drain_state_machine_update_time_step(
						STATE_BATCH,
					);
				if budget.all_gte(smu_required) {
					let result = StateMachineUpdateTime::<T>::clear(
						STATE_DRAIN,
						cursor.as_ref().map(|v| v.as_slice()),
					);
					let new_state = match result.maybe_cursor {
						Some(c) => match polkadot_sdk::frame_support::BoundedVec::try_from(c) {
							Ok(b) => super::LegacyDrainState::Active(Some(b)),
							Err(_) => super::LegacyDrainState::Active(None),
						},
						None => super::LegacyDrainState::Done,
					};
					LegacyStateMachineUpdateTimeDrainState::<T>::put(new_state);
					let used =
						<T as Config>::MigrationWeightInfo::drain_state_machine_update_time_step(
							result.unique,
						);
					consumed = consumed.saturating_add(used);
					budget = budget.saturating_sub(used);
				}
			}

			// ── Drain child trie state commitment entries ─────────────────
			// `next_key(last_key)` resumes iteration just past `last_key`, so we
			// store the most recently processed key as the cursor. When iteration
			// runs out of keys with the `STATE_COMMITMENTS_KEY` prefix we mark the
			// drain as Done.
			let ct_state = LegacyChildTrieDrainState::<T>::get();
			if let super::LegacyDrainState::Active(cursor) = ct_state {
				let ct_required =
					<T as Config>::MigrationWeightInfo::drain_child_trie_state_commitments_step(
						CHILD_TRIE_BATCH,
					);
				if budget.all_gte(ct_required) {
					let child_info = ChildInfo::new_default(CHILD_TRIE_PREFIX);
					let mut removed = 0u32;
					let mut last_key: alloc::vec::Vec<u8> = match cursor {
						Some(c) => c.into_inner(),
						None => STATE_COMMITMENTS_KEY.to_vec(),
					};
					let mut exhausted = false;
					for _ in 0..CHILD_TRIE_DRAIN {
						match sp_io::default_child_storage::next_key(
							child_info.storage_key(),
							&last_key,
						) {
							Some(key) if key.starts_with(STATE_COMMITMENTS_KEY) => {
								sp_io::default_child_storage::clear(child_info.storage_key(), &key);
								last_key = key;
								removed += 1;
							},
							_ => {
								exhausted = true;
								break;
							},
						}
					}

					let new_state = if exhausted {
						super::LegacyDrainState::Done
					} else {
						match polkadot_sdk::frame_support::BoundedVec::try_from(last_key) {
							Ok(b) => super::LegacyDrainState::Active(Some(b)),
							Err(_) => super::LegacyDrainState::Active(None),
						}
					};
					LegacyChildTrieDrainState::<T>::put(new_state);
					let used =
						<T as Config>::MigrationWeightInfo::drain_child_trie_state_commitments_step(
							removed,
						);
					consumed = consumed.saturating_add(used);
				}
			}

			consumed
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
		#[pallet::weight(weight())]
		#[pallet::call_index(0)]
		#[frame_support::transactional]
		pub fn handle_unsigned(
			origin: OriginFor<T>,
			messages: Vec<Message>,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			Self::execute(messages.clone())?;

			Ok(().into())
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

			for (state_id, period) in message.challenge_periods {
				let id =
					StateMachineId { state_id, consensus_state_id: message.consensus_state_id };
				host.store_challenge_period(id, period)
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
	#[pallet::generate_deposit(pub fn deposit_event)]
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
			/// Response Commitment
			commitment: H256,
			/// Request commitment
			req_commitment: H256,
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
		/// Error charging fee
		ErrorChargingFee,
	}

	/// This allows users execute ISMP datagrams for free. Use with caution.
	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		// empty pre-dispatch do we don't modify storage
		fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
			Ok(())
		}

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			use ismp::{
				messaging::{hash_request, FraudProofMessage, RequestMessage},
				router::Request,
			};
			let messages = match call {
				Call::handle_unsigned { messages } => messages,
				_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call))?,
			};

			let events =
				Self::execute(messages.clone()).map_err(|_| InvalidTransaction::BadProof)?;

			if let Some((state_machine_id, latest_height)) = events.iter().find_map(|event| {
				if let ismp::events::Event::StateMachineUpdated(state_machine_updated_event) = event
				{
					Some((
						state_machine_updated_event.state_machine_id.clone(),
						state_machine_updated_event.latest_height,
					))
				} else {
					None
				}
			}) {
				return Ok(ValidTransaction {
					priority: latest_height,
					requires: vec![],
					provides: vec![sp_io::hashing::keccak_256(&state_machine_id.encode()).to_vec()],
					longevity: 25,
					propagate: true,
				});
			}

			let mut requests = messages
				.into_iter()
				.map(|message| match message {
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
					_ => vec![],
				})
				.collect::<Vec<_>>();
			requests.sort();

			if requests.is_empty() {
				return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
			}

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

	/// Static weights because these should get overridden by the FeeHandler
	fn weight() -> Weight {
		Weight::from_parts(300_000_000, 0)
	}

	impl<T: Config> Pallet<T> {
		/// Insert a state commitment into the bounded map. ISMP does not allow
		/// duplicate state updates so we don't have an overwrite path. Evicts the
		/// oldest entry for `chain` first if the per-chain cap has been reached,
		/// using [`KnownStateMachineHeights`] as both the cap counter (via `len`)
		/// and the sorted pointer for eviction.
		pub fn insert_bounded_state_commitment(
			height: StateMachineHeight,
			commitment: StateCommitment,
		) {
			KnownStateMachineHeights::<T>::mutate(height.id, |heights| {
				if heights.len() as u32 >= MAX_STATE_MACHINE_COMMITMENTS {
					if let Some(&oldest) = heights.iter().next() {
						heights.remove(&oldest);
						BoundedStateCommitments::<T>::remove(height.id, oldest);
						BoundedStateMachineUpdateTime::<T>::remove(height.id, oldest);
					}
				}
				let _ = heights.try_insert(height.height);
			});
			BoundedStateCommitments::<T>::insert(height.id, height.height, commitment);
		}

		/// Insert a state machine update time into the bounded map. Shares the
		/// [`KnownStateMachineHeights`] pointer with [`BoundedStateCommitments`]
		/// since both maps track the same heights per chain.
		pub fn insert_bounded_update_time(height: StateMachineHeight, timestamp: u64) {
			BoundedStateMachineUpdateTime::<T>::insert(height.id, height.height, timestamp);
		}
	}
}
