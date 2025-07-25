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
	use alloc::vec::Vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::{consensus::ConsensusStateId, host::IsmpHost};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_ismp::Config {
		/// Origin allowed to add or remove parachains in Consensus State
		type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// IsmpHost implementation
		type IsmpHost: IsmpHost + Default;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error storing consensus state
		ErrorStoringConsensusState,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New epoch length set
		NewEpochLength { epoch_length: u64 },
	}

	/// BSC Epoch length
	#[pallet::storage]
	#[pallet::getter(fn epoch_length)]
	pub type EpochLength<T: Config> = StorageValue<_, u64, OptionQuery>;

	#[derive(
		Clone,
		codec::Encode,
		codec::Decode,
		DecodeWithMemTracking,
		scale_info::TypeInfo,
		PartialEq,
		Eq,
		RuntimeDebug,
	)]
	pub struct UpdateParams {
		pub epoch_length: u64,
		pub consensus_state: Option<Vec<u8>>,
		pub consensus_state_id: Option<ConsensusStateId>,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Sets the new BSC epoch length and resets the consensus state
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 3))]
		pub fn set_epoch_length(origin: OriginFor<T>, params: UpdateParams) -> DispatchResult {
			<T as Config>::AdminOrigin::ensure_origin(origin)?;
			let host = <T as Config>::IsmpHost::default();
			EpochLength::<T>::put(params.epoch_length);
			if let Some((consensus_state_id, consensus_state)) = params
				.consensus_state_id
				.and_then(|id| params.consensus_state.map(|state| (id, state)))
			{
				host.store_consensus_state(consensus_state_id, consensus_state)
					.map_err(|_| Error::<T>::ErrorStoringConsensusState)?;
			}

			Self::deposit_event(Event::<T>::NewEpochLength { epoch_length: params.epoch_length });

			Ok(())
		}
	}
}
