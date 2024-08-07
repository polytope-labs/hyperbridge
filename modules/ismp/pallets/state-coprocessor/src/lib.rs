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

//! The state coprocessor performs all the neeeded state proof verification needed to certify a
//! GetResponse on behalf of connected chains.

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;

mod impls;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use ismp::host::IsmpHost;
	use mmr_primitives::MerkleMountainRangeTree;
	use pallet_ismp::mmr::Leaf;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_ismp::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + Default;

		/// Merkle mountain range overlay tree implementation.
		///
		/// Verified GetResponse(s) are stored in the mmr
		type Mmr: MerkleMountainRangeTree<Leaf = Leaf>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Some error happened
		SomeError,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account `account` has been added to the fishermen set.
		Added { account: T::AccountId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: AsRef<[u8]>,
	{
		/// Adds a new fisherman to the set
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn add(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			Ok(())
		}
	}
}
