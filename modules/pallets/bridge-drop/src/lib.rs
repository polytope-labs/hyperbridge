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

//! Airdrop for Bridge Tokens

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
pub use pallet::*;
use polkadot_sdk::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use alloc::{vec, vec::Vec};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use polkadot_sdk::{
		frame_system::ensure_none,
		sp_core::{H160, H256},
	};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The config trait
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config + polkadot_sdk::pallet_balances::Config
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
	}

	/// Set of accounts that have claimed the airdrop
	#[pallet::storage]
	#[pallet::getter(fn claimed)]
	pub type Claimed<T: Config> = StorageMap<_, Blake2_128Concat, H160, T::AccountId, OptionQuery>;

	/// Merkle root
	#[pallet::storage]
	#[pallet::getter(fn merkle_root)]
	pub type MerkleRoot<T: Config> = StorageValue<_, H256, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Account has claimed the airdrop already
		AlreadyClaimed,
		/// Invalid claim merkle proof
		InvalidMerkleProof,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Airdrop claimed
		Claimed { account: T::AccountId, amount: <T as pallet_balances::Config>::Balance },
	}

	#[derive(
		Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct Proof<T: Config> {
		/// Account Eligible for the claim
		who: H160,
		/// Receiving account on Hyperbridge
		dest: T::AccountId,
		/// Signature that validates the receiving address
		signature: Vec<u8>,
		/// Merkle proof of eligibility
		proof: Vec<H256>,
		/// Amount to claim
		amount: <T as pallet_balances::Config>::Balance,
	}

	struct Claim<T: Config> {
		account: T::AccountId,
		amount: u128,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as pallet_balances::Config>::Balance: From<u128>,
		<T as frame_system::Config>::Hash: From<H256>,
	{
		/// Claim bridge tokens
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 2))]
		pub fn claim_tokens(origin: OriginFor<T>, proof: Proof<T>) -> DispatchResult {
			ensure_none(origin)?;

			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T>
	where
		<T as frame_system::Config>::Hash: From<H256>,
		<T as pallet_balances::Config>::Balance: From<u128>,
	{
		type Call = Call<T>;

		// empty pre-dispatch so we don't modify storage
		fn pre_dispatch(_call: &Self::Call) -> Result<(), TransactionValidityError> {
			Ok(())
		}

		fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			let Call::claim_tokens { proof } = call else {
				return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
			};

			Ok(ValidTransaction {
				priority: 100,
				requires: vec![],
				provides: vec![],
				longevity: 25,
				propagate: true,
			})
		}
	}

	impl<T: Config> Pallet<T> {
		fn validate_proof(proof: Proof<T>) -> Result<(), Error<T>> {
			Ok(())
		}
	}
}
