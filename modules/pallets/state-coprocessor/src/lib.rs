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
use polkadot_sdk::*;
pub mod impls;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::{vec, vec::Vec};

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use impls::GetRequestsWithProof;
	use ismp::{host::IsmpHost, messaging::hash_request, router::Request};
	use pallet_ismp::offchain::{Leaf, OffchainDBProvider};
	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config + pallet_ismp::Config + pallet_ismp_relayer::Config
	{
		/// The underlying [`IsmpHost`] implementation
		type IsmpHost: IsmpHost + Default;

		/// Merkle mountain range overlay tree implementation.
		///
		/// Verified GetResponse(s) are stored in the mmr
		type Mmr: OffchainDBProvider<Leaf = Leaf>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// An error occured, check the node logs.
		HandlingError,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: AsRef<[u8]>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		<T as pallet_ismp::Config>::Balance: Into<u128>,
	{
		/// This is an extension of the ISMP protocol to allow Hyperbridge perform state proof
		/// verification on behalf of its applications and provides the verified values in the
		/// overlay tree.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn handle_unsigned(
			origin: OriginFor<T>,
			message: GetRequestsWithProof,
		) -> DispatchResult {
			ensure_none(origin)?;

			Self::handle_get_requests(message).map_err(|err| {
				log::error!(target: "ismp", "pallet-coprocessor: {:?}", err);
				Error::<T>::HandlingError
			})?;

			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T>
	where
		T::AccountId: AsRef<[u8]>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
		<T as pallet_ismp::Config>::Balance: Into<u128>,
	{
		type Call = Call<T>;

		// empty pre-dispatch so we don't modify storage
		fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
			Ok(())
		}

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			let Call::handle_unsigned { message } = call else {
				return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
			};

			if let Err(err) = Self::handle_get_requests(message.clone()) {
				log::error!(target: "ismp", "{:?}", err);
				return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
			}

			let mut messages = message
				.requests
				.iter()
				.map(|get| hash_request::<<T as Config>::IsmpHost>(&Request::Get(get.clone())))
				.collect::<Vec<_>>();
			messages.sort();

			// this is so we can reject duplicate batches at the mempool level
			let msg_hash = sp_io::hashing::keccak_256(&messages.encode()).to_vec();

			Ok(ValidTransaction {
				priority: 100,
				requires: vec![],
				provides: vec![msg_hash],
				longevity: 25,
				propagate: true,
			})
		}
	}
}
