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
mod migration;

pub use consensus::*;

use alloc::{vec, vec::Vec};
use codec::{Decode, Encode, MaxEncodedLen};
use cumulus_pallet_parachain_system::{
	RelayChainState, RelaychainDataProvider, RelaychainStateProvider,
};
use cumulus_primitives_core::relay_chain;
use ismp::{handlers, messaging::CreateConsensusState};
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use cumulus_primitives_core::relay_chain;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::{
		host::IsmpHost,
		messaging::{ConsensusMessage, Message},
	};
	use migration::StorageV0;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);
	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_ismp::Config + cumulus_pallet_parachain_system::Config
	{
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + Default;
	}

	/// Mapping of relay chain heights to it's state commitment. The state commitment of the parent
	/// relay block is inserted at every block in `on_finalize`. This commitment is gotten from
	/// parachain-system.
	#[pallet::storage]
	#[pallet::getter(fn relay_chain_state)]
	pub type RelayChainStateCommitments<T: Config> =
		StorageMap<_, Blake2_128Concat, relay_chain::BlockNumber, relay_chain::Hash, OptionQuery>;

	/// Tracks whether we've already seen the `update_parachain_consensus` inherent
	#[pallet::storage]
	pub type ConsensusUpdated<T: Config> = StorageValue<_, bool>;

	/// List of parachains that this state machine is interested in.
	#[pallet::storage]
	pub type Parachains<T: Config> = StorageMap<_, Identity, u32, ParachainData>;

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
		/// This allows block builders submit parachain consensus proofs as inherents. If the
		/// provided [`ConsensusMessage`] is not for a parachain, this call will fail.
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn update_parachain_consensus(
			origin: OriginFor<T>,
			data: ConsensusMessage,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			assert!(
				!ConsensusUpdated::<T>::exists(),
				"ValidationData must be updated only once in a block",
			);

			assert_eq!(
				data.consensus_state_id, PARACHAIN_CONSENSUS_ID,
				"Only parachain consensus updates should be passed in the inherents!"
			);
			pallet_ismp::Pallet::<T>::handle_messages(vec![Message::Consensus(data)])?;
			ConsensusUpdated::<T>::put(true);

			Ok(Pays::No.into())
		}

		/// Add some new parachains to the parachains whitelist
		#[pallet::call_index(1)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(para_ids.len() as u64))]
		pub fn add_parachain(origin: OriginFor<T>, para_ids: Vec<ParachainData>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			for para in &para_ids {
				Parachains::<T>::insert(para.id, para);
			}

			Self::deposit_event(Event::ParachainsAdded { para_ids });

			Ok(())
		}

		/// Removes some parachains from the parachains whitelist
		#[pallet::call_index(2)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().writes(para_ids.len() as u64))]
		pub fn remove_parachain(origin: OriginFor<T>, para_ids: Vec<u32>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			for id in &para_ids {
				Parachains::<T>::remove(id);
			}

			Self::deposit_event(Event::ParachainsRemoved { para_ids });

			Ok(())
		}
	}

	// Pallet implements [`Hooks`] trait to define some logic to execute in some context.
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_n: BlockNumberFor<T>) {
			let state = RelaychainDataProvider::<T>::current_relay_chain_state();
			if !RelayChainStateCommitments::<T>::contains_key(state.number) {
				RelayChainStateCommitments::<T>::insert(state.number, state.state_root);
			}
		}

		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			// kill the storage, since this is the beginning of a new block.
			ConsensusUpdated::<T>::kill();
			let host = T::IsmpHost::default();
			if let Err(_) = host.consensus_state(PARACHAIN_CONSENSUS_ID) {
				Pallet::<T>::initialize();
			}

			Weight::from_parts(0, 0)
		}

		fn on_runtime_upgrade() -> Weight {
			StorageV0::migrate_to_v1::<T>()
		}
	}

	/// The identifier for the parachain consensus update inherent.
	pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"paraismp";

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = sp_inherents::MakeFatalError<()>;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let data: ConsensusMessage =
				data.get_data(&Self::INHERENT_IDENTIFIER).ok().flatten()?;

			Some(Call::update_parachain_consensus { data })
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::update_parachain_consensus { .. })
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

			// insert the parachain ids
			for para in &self.parachains {
				Parachains::<T>::insert(para.id, para);
			}
		}
	}
}

impl<T: Config> Pallet<T> {
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
		let message = CreateConsensusState {
			// insert empty bytes
			consensus_state: vec![],
			unbonding_period: u64::MAX,
			challenge_period: 0,
			consensus_state_id: PARACHAIN_CONSENSUS_ID,
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
		RelayChainStateCommitments::<T>::get(height)
	}
}

/// Data provided when registering a parachain to be tracked by hyperbridge consensus client
#[derive(
	Debug,
	Clone,
	Copy,
	Encode,
	Decode,
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
	/// parachain slot duration type
	pub slot_duration: u64,
}
