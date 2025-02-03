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

pub use pallet::*;
use polkadot_sdk::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::types::{ConsensusState, L2Consensus};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::{
		consensus::StateMachineId,
		host::{IsmpHost, StateMachine},
	};

	use sp_core::H256;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config<I: 'static = ()>:
		polkadot_sdk::frame_system::Config + pallet_ismp::Config
	{
		/// Origin allowed to add or remove parachains in Consensus State
		type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + Default;
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Contract Address Already Exists
		ContractAddressAlreadyExists,
		/// Error fetching consensus state
		ErrorFetchingConsensusState,
		/// Error decoding consensus state
		ErrorDecodingConsensusState,
		/// Error storing consensus state
		ErrorStoringConsensusState,
	}

	/// Additional L2s added after the consensus client has been initialized
	#[pallet::storage]
	#[pallet::getter(fn layer_twos)]
	pub type SupportedStatemachines<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Twox64Concat, StateMachine, bool, OptionQuery>;

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I>
	where
		<T as frame_system::Config>::Hash: From<H256>,
	{
		/// Add a new l2 consensus to the sync committee consensus state
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn add_l2_consensus(
			origin: OriginFor<T>,
			state_machine_id: StateMachineId,
			l2_consensus: L2Consensus,
		) -> DispatchResult {
			<T as Config<I>>::AdminOrigin::ensure_origin(origin)?;

			let host = <T as Config<I>>::IsmpHost::default();
			let StateMachineId { consensus_state_id, state_id: state_machine } = state_machine_id;

			let encoded_consensus_state = host
				.consensus_state(consensus_state_id)
				.map_err(|_| Error::<T, I>::ErrorFetchingConsensusState)?;
			let mut consensus_state: ConsensusState =
				codec::Decode::decode(&mut &encoded_consensus_state[..])
					.map_err(|_| Error::<T, I>::ErrorDecodingConsensusState)?;

			consensus_state.l2_consensus.insert(state_machine, l2_consensus);
			SupportedStatemachines::<T, I>::insert(state_machine_id.state_id, true);

			let encoded_consensus_state = consensus_state.encode();
			host.store_consensus_state(consensus_state_id, encoded_consensus_state)
				.map_err(|_| Error::<T, I>::ErrorStoringConsensusState)?;
			Ok(())
		}

		/// Add a new state machine
		#[pallet::call_index(1)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads(1))]
		pub fn add_state_machine(
			origin: OriginFor<T>,
			state_machine_id: StateMachineId,
		) -> DispatchResult {
			<T as Config<I>>::AdminOrigin::ensure_origin(origin)?;
			SupportedStatemachines::<T, I>::insert(state_machine_id.state_id, true);
			Ok(())
		}
	}
}
