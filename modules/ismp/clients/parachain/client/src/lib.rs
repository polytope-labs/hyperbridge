// Copyright (C) Polytope Labs Ltd.
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
#![deny(missing_docs)]

extern crate alloc;
extern crate core;

pub mod consensus;
pub mod migration;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
/// weights trait crate
pub mod weights;

pub use consensus::*;
use polkadot_sdk::*;

use alloc::{vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use cumulus_pallet_parachain_system::{
	RelayChainState, RelaychainDataProvider, RelaychainStateProvider,
};
use cumulus_primitives_core::relay_chain;
use frame_support::weights::Weight;
use ismp::{
	consensus::ConsensusStateId,
	handlers,
	host::{IsmpHost, StateMachine},
	messaging::CreateConsensusState,
};
pub use pallet::*;
pub use weights::WeightInfo;

/// Maximum number of relay-chain state commitments retained in
/// [`pallet::CurrentRelayChainStateRoots`].
/// At ~6s per relay block, 256 entries covers ~25 minutes of history.
pub const MAX_RELAY_STATE_COMMITMENTS: u32 = 256;

/// State of the multi-block drain of the legacy `RelayChainStateCommitments`
/// map. `Active(None)` is the initial state and means the next `on_idle` call
/// should pass `None` as the `clear()` cursor (start from the beginning).
/// `Active(Some(cursor))` carries the cursor returned by the previous call so
/// the next call resumes where it left off. `Done` is set when `clear()` returns
/// `None`, indicating the prefix has been fully cleared; subsequent `on_idle`
/// calls become a single cheap read and do no further work.
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
	/// Drain in progress. `None` means start from the beginning of the prefix;
	/// `Some(cursor)` means resume from the given cursor.
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

frame_support::parameter_types! {
	/// Type-level `Get<u32>` mirror of [`MAX_RELAY_STATE_COMMITMENTS`], used as the
	/// bound for [`pallet::KnownRelayHeights`].
	pub const MaxRelayStateCommitments: u32 = MAX_RELAY_STATE_COMMITMENTS;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use cumulus_primitives_core::relay_chain;
	use frame_support::{pallet_prelude::*, BoundedBTreeSet};
	use frame_system::pallet_prelude::*;
	use ismp::{consensus::StateMachineId, host::StateMachine};
	use migration::{StorageV0, StorageV1};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);
	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config
		+ pallet_ismp::Config
		+ cumulus_pallet_parachain_system::Config
	{
		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + Default;
		/// WeightInfo
		type WeightInfo: WeightInfo;
		/// Origin for privileged actions
		type RootOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	/// **Legacy / read-only.** The original unbounded map of relay chain heights
	/// to state roots. New entries are no longer written here. Existing entries
	/// are drained at [`LEGACY_DRAIN_BATCH_SIZE`] per block in `on_finalize`
	/// until the map is empty, at which point this storage item can be removed
	/// in a future runtime upgrade.
	///
	/// The [`RelayChainOracle`] implementation reads from
	/// [`CurrentRelayChainStateRoots`] first and falls back to this map, so
	/// consensus proofs that reference relay heights still in the legacy map
	/// continue to verify during the transition.
	#[pallet::storage]
	pub type RelayChainStateCommitments<T: Config> =
		StorageMap<_, Blake2_128Concat, relay_chain::BlockNumber, relay_chain::Hash, OptionQuery>;

	// ── New bounded storage ─────────────────────────────────────────────

	/// Bounded map of recent relay chain heights to state roots. Capped at
	/// [`MAX_RELAY_STATE_COMMITMENTS`] entries by the eviction logic in
	/// `on_finalize`. This replaces [`RelayChainStateCommitments`] as the
	/// primary write target.
	#[pallet::storage]
	pub type CurrentRelayChainStateRoots<T: Config> = CountedStorageMap<
		_,
		Blake2_128Concat,
		relay_chain::BlockNumber,
		relay_chain::Hash,
		OptionQuery,
	>;

	/// Sorted pointer set of relay chain block numbers currently held in
	/// [`CurrentRelayChainStateRoots`]. The map uses a `Blake2_128Concat` hasher,
	/// so its `iter_keys()` order is hash order, not numeric order, and cannot
	/// tell us which entry is the oldest. `BTreeSet` iterates in ascending key
	/// order, so `.iter().next()` on this set returns the smallest known height
	/// deterministically. Every write to `CurrentRelayChainStateRoots` must keep
	/// this set in sync.
	#[pallet::storage]
	pub type KnownRelayHeights<T: Config> = StorageValue<
		_,
		BoundedBTreeSet<relay_chain::BlockNumber, super::MaxRelayStateCommitments>,
		ValueQuery,
	>;

	/// State of the multi-block drain of the legacy [`RelayChainStateCommitments`]
	/// map performed in `on_idle`. `Active(None)` starts a fresh `clear()` from
	/// the beginning of the prefix; `Active(Some(cursor))` continues from `cursor`;
	/// `Done` short-circuits `on_idle` so the drain becomes a single cheap read
	/// once it finishes.
	#[pallet::storage]
	pub type LegacyRelayDrainState<T: Config> = StorageValue<_, LegacyDrainState, ValueQuery>;

	/// List of parachains that this state machine is interested in.
	#[pallet::storage]
	pub type Parachains<T: Config> = StorageMap<_, Identity, u32, ()>;

	/// Events emitted by this pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Parachains with the `para_ids` have been added to the whitelist
		ParachainsAdded {
			/// The parachains in question
			para_ids: Vec<ParachainData>,
		},
		/// Parachains with the `para_ids` have been removed from the whitelist
		ParachainsRemoved {
			/// The parachains in question
			para_ids: Vec<u32>,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add some new parachains to the parachains whitelist
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::add_parachain(para_ids.len() as u32))]
		pub fn add_parachain(origin: OriginFor<T>, para_ids: Vec<ParachainData>) -> DispatchResult {
			T::RootOrigin::ensure_origin(origin)?;
			let host = <T::IsmpHost>::default();
			for para in &para_ids {
				let state_id = match host.host_state_machine() {
					StateMachine::Kusama(_) => StateMachine::Kusama(para.id),
					StateMachine::Polkadot(_) => StateMachine::Polkadot(para.id),
					_ => continue,
				};
				Parachains::<T>::insert(para.id, ());
				let _ = host.store_challenge_period(
					StateMachineId {
						state_id,
						consensus_state_id: parachain_consensus_state_id(host.host_state_machine()),
					},
					0,
				);
			}

			Self::deposit_event(Event::ParachainsAdded { para_ids });

			Ok(())
		}

		/// Removes some parachains from the parachains whitelist
		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::remove_parachain(para_ids.len() as u32))]
		pub fn remove_parachain(origin: OriginFor<T>, para_ids: Vec<u32>) -> DispatchResult {
			T::RootOrigin::ensure_origin(origin)?;
			for id in &para_ids {
				Parachains::<T>::remove(id);
			}

			Self::deposit_event(Event::ParachainsRemoved { para_ids });

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_n: BlockNumberFor<T>) {
			let state = RelaychainDataProvider::<T>::current_relay_chain_state();

			if CurrentRelayChainStateRoots::<T>::contains_key(state.number) {
				return;
			}

			// Evict first so neither the map nor the pointer set ever exceeds the cap.
			if CurrentRelayChainStateRoots::<T>::count() >= MAX_RELAY_STATE_COMMITMENTS {
				Self::evict_oldest_relay_commitment();
			}

			CurrentRelayChainStateRoots::<T>::insert(state.number, state.state_root);
			KnownRelayHeights::<T>::mutate(|heights| {
				let _ = heights.try_insert(state.number);
			});
		}

		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let host = T::IsmpHost::default();
			if let Err(_) =
				host.consensus_state(parachain_consensus_state_id(host.host_state_machine()))
			{
				Pallet::<T>::initialize();
			}

			<T as pallet::Config>::WeightInfo::on_finalize_bound_relay_state_commitments()
		}

		fn on_runtime_upgrade() -> Weight {
			StorageV0::migrate_to_v1::<T>() + StorageV1::migrate_to_v2::<T>()
		}

		/// Drains the legacy [`RelayChainStateCommitments`] map using leftover block
		/// weight. Runs after all extrinsics, so it never blocks user transactions.
		/// The `clear()` cursor is persisted in [`LegacyRelayDrainState`] between
		/// blocks so each call resumes where the previous one stopped, and we mark
		/// the drain as `Done` once `clear()` reports no remaining cursor.
		fn on_idle(_n: BlockNumberFor<T>, remaining: Weight) -> Weight {
			// We charge weight for `BATCH_SIZE` up front but only remove
			// `BATCH_SIZE - SAFETY_BUFFER` entries per call. The buffer absorbs
			// any imprecision in the benchmarked weight so a single step never
			// exceeds its charged budget.
			const SAFETY_BUFFER: u32 = 10;
			const BATCH_SIZE: u32 = 500;
			const DRAIN: u32 = BATCH_SIZE - SAFETY_BUFFER;

			let state = LegacyRelayDrainState::<T>::get();
			let cursor = match state {
				LegacyDrainState::Done => return Weight::zero(),
				LegacyDrainState::Active(c) => c,
			};

			let required =
				<T as pallet::Config>::WeightInfo::drain_relay_state_commitments_step(BATCH_SIZE);
			if remaining.any_lt(required) {
				log::trace!(
					target: "ismp",
					"Skipping RelayChainStateCommitments drain: insufficient weight (need {:?}, have {:?})",
					required, remaining,
				);
				return Weight::zero();
			}

			let result = RelayChainStateCommitments::<T>::clear(
				DRAIN,
				cursor.as_ref().map(|v| v.as_slice()),
			);

			let new_state = match result.maybe_cursor {
				Some(c) => {
					if result.unique > 0 {
						log::info!(
							target: "ismp",
							"Draining legacy RelayChainStateCommitments: removed {} entries, continuing",
							result.unique,
						);
					}
					match polkadot_sdk::frame_support::BoundedVec::try_from(c) {
						Ok(bounded) => LegacyDrainState::Active(Some(bounded)),
						// Cursor too large to bound: fall back to None so the next call
						// restarts from the beginning of the prefix rather than getting stuck.
						Err(_) => LegacyDrainState::Active(None),
					}
				},
				None => {
					log::info!(
						target: "ismp",
						"Legacy RelayChainStateCommitments drain complete (final batch removed {} entries)",
						result.unique,
					);
					LegacyDrainState::Done
				},
			};
			LegacyRelayDrainState::<T>::put(new_state);

			<T as pallet::Config>::WeightInfo::drain_relay_state_commitments_step(result.unique)
		}
	}

	/// The genesis config
	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T> {
		/// List of parachains to track at genesis
		pub parachains: Vec<ParachainData>,
		/// phantom data
		#[serde(skip)]
		pub _marker: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize();
			let host = <T::IsmpHost>::default();
			let host_state_machine = host.host_state_machine();

			// insert the parachain ids
			for para in &self.parachains {
				Parachains::<T>::insert(para.id, ());
				let state_id = match host.host_state_machine() {
					StateMachine::Kusama(_) => StateMachine::Kusama(para.id),
					StateMachine::Polkadot(_) => StateMachine::Polkadot(para.id),
					_ => continue,
				};
				let _ = host.store_challenge_period(
					StateMachineId {
						state_id,
						consensus_state_id: parachain_consensus_state_id(host_state_machine),
					},
					0,
				);
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Evicts the oldest entry from [`CurrentRelayChainStateRoots`] using the
	/// sorted [`KnownRelayHeights`] pointer set. The map's `Blake2_128Concat`
	/// hasher means `iter_keys()` does not yield keys in numeric order, so it
	/// cannot identify the oldest height. `BTreeSet` iterates in ascending key
	/// order, so `.iter().next()` here returns the smallest known height
	/// deterministically.
	fn evict_oldest_relay_commitment() {
		KnownRelayHeights::<T>::mutate(|heights| {
			if let Some(&oldest) = heights.iter().next() {
				heights.remove(&oldest);
				CurrentRelayChainStateRoots::<T>::remove(oldest);
			}
		});
	}

	/// Returns the list of parachains who's consensus updates will be inserted by the inherent
	/// data provider
	pub fn para_ids() -> Vec<u32> {
		Parachains::<T>::iter_keys().collect()
	}

	/// Returns the current relay chain state
	pub fn current_relay_chain_state() -> RelayChainState {
		RelaychainDataProvider::<T>::current_relay_chain_state()
	}

	/// Initializes the parachain consensus state. Rather than requiring a seperate
	/// `create_consensus_state` call, simply including this pallet in your runtime will create the
	/// ismp parachain client consensus state, either through `genesis_build` or `on_initialize`.
	pub fn initialize() {
		let host = T::IsmpHost::default();
		let host_state_machine = host.host_state_machine();
		let message = CreateConsensusState {
			// insert empty bytes
			consensus_state: vec![],
			unbonding_period: u64::MAX,
			challenge_periods: Default::default(),
			consensus_state_id: parachain_consensus_state_id(host_state_machine),
			consensus_client_id: PARACHAIN_CONSENSUS_ID,
			state_machine_commitments: vec![],
		};
		handlers::create_client(&host, message)
			.expect("Failed to initialize parachain consensus client");
	}
}

/// Interface that exposes the relay chain state roots.
pub trait RelayChainOracle {
	/// Returns the state root for a given height if it exists.
	fn state_root(height: relay_chain::BlockNumber) -> Option<relay_chain::Hash>;
}

impl<T: Config> RelayChainOracle for Pallet<T> {
	fn state_root(height: relay_chain::BlockNumber) -> Option<relay_chain::Hash> {
		CurrentRelayChainStateRoots::<T>::get(height)
	}
}

/// Data provided when registering a parachain to be tracked by hyperbridge consensus client
#[derive(
	Debug,
	Clone,
	Copy,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Hash,
	Eq,
	MaxEncodedLen,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct ParachainData {
	/// parachain id
	pub id: u32,
}

/// Returns the consensus state id for The relay chain
pub fn parachain_consensus_state_id(host: StateMachine) -> ConsensusStateId {
	match host {
		StateMachine::Kusama(_) => PASEO_CONSENSUS_ID,
		StateMachine::Polkadot(_) => POLKADOT_CONSENSUS_ID,
		StateMachine::Evm(id) if id == PASSET_HUB_TESTNET_CHAIN_ID => PASEO_CONSENSUS_ID,
		StateMachine::Evm(id) if id == ASSET_HUB_MAINNET_CHAIN_ID => POLKADOT_CONSENSUS_ID,
		_ => POLKADOT_CONSENSUS_ID,
	}
}
