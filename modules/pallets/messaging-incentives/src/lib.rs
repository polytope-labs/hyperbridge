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

//! # pallet-messaging-incentives
//!
//! Mints reputation tokens to the relayer that delivered each
//! message, scaled by message size. The per-byte mint rate is set by
//! governance via [`Pallet::set_mint_per_byte`]; a rate of zero
//! disables minting without uninstalling the pallet.
//!
//! [`pallet-collator-manager`] consumes the [`IncentivesManager`]
//! trait so it stays exported here, but the canonical impl is a noop
//! since this version doesn't accumulate per-session state.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use codec::{Decode, Encode};
use frame_support::{
	dispatch::{DispatchResultWithPostInfo, Pays, PostDispatchInfo},
	pallet_prelude::*,
	traits::fungible::{self, Mutate},
};
use frame_system::pallet_prelude::*;
use polkadot_sdk::{
	sp_io,
	sp_runtime::traits::{SaturatedConversion, Saturating, Zero},
	*,
};

use crypto_utils::verification::Signature;
use ismp::{
	events::Event as IsmpEvent,
	messaging::{Message, MessageWithWeight},
	router::{RequestResponse, GetResponse},
};
use pallet_ismp::fee_handler::FeeHandler;

pub use pallet::*;

/// Trait kept for `pallet-collator-manager`'s Config bound. The
/// reputation-mint flow doesn't accumulate per-session state, so the
/// canonical impl on [`Pallet`] is a noop.
pub trait IncentivesManager {
	fn reset_incentives();
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::StorageVersion;

	pub type BalanceOf<T> = <<T as Config>::ReputationAsset as fungible::Inspect<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config<RuntimeEvent: From<Event<Self>>> {
		/// The fungible asset minted to relayers.
		type ReputationAsset: fungible::Mutate<Self::AccountId>;
		/// Origin allowed to update the per-byte mint rate.
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	/// Reputation tokens minted per byte of delivered payload. Zero
	/// disables minting; non-zero applies to every message executed
	/// after it is set.
	#[pallet::storage]
	pub type MintPerByte<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		ReputationMintFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MintRateUpdated { amount: BalanceOf<T> },
		ReputationMinted { relayer: T::AccountId, bytes: u32, amount: BalanceOf<T> },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Update the per-byte mint rate. Pass zero to disable minting.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000_000, 0).saturating_add(T::DbWeight::get().writes(1)))]
		pub fn set_mint_per_byte(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			MintPerByte::<T>::put(amount);
			Self::deposit_event(Event::MintRateUpdated { amount });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T>
where
	T::AccountId: From<[u8; 32]>,
{
	/// Same minimum-byte rule as the bandwidth gate (`max(body, 32)`)
	/// so undersized payloads can't game the mint by being charged 0.
	fn message_bytes(message: &Message) -> u32 {
		let raw = match message {
			Message::Request(req) => req.requests.iter().map(|p| p.body.len()).sum::<usize>(),
			Message::Response(res) => match &res.datagram {
				RequestResponse::Response(responses) => responses
					.iter()
					.map(|r| {
						r.values.iter().filter_map(|v| v.value.as_ref()).map(|b| b.len()).sum::<usize>()
					})
					.sum::<usize>(),
				RequestResponse::Request(_) => 0,
			},
			_ => 0,
		};
		core::cmp::max(raw as u32, 32)
	}

	/// Recover the relayer's account from the sr25519 signature on a
	/// `Message`'s `signer` field. Returns `None` if the message has
	/// no signer (e.g. consensus messages) or the signature is bad.
	fn relayer_for(message: &Message) -> Option<T::AccountId> {
		let (signer, signed) = match message {
			Message::Request(msg) =>
				(&msg.signer, sp_io::hashing::keccak_256(&msg.requests.encode())),
			Message::Response(msg) =>
				(&msg.signer, sp_io::hashing::keccak_256(&msg.datagram.encode())),
			_ => return None,
		};
		Signature::decode(&mut &signer[..])
			.ok()?
			.verify_and_get_sr25519_pubkey(&signed, None)
			.ok()
			.map(T::AccountId::from)
	}
}

impl<T: Config> FeeHandler for Pallet<T>
where
	T::AccountId: From<[u8; 32]>,
{
	fn on_executed(
		messages: Vec<MessageWithWeight>,
		_events: Vec<IsmpEvent>,
	) -> DispatchResultWithPostInfo {
		let rate = MintPerByte::<T>::get();
		if !rate.is_zero() {
			for mw in &messages {
				let bytes = Self::message_bytes(&mw.message);
				let bytes_balance: BalanceOf<T> = (bytes as u128).saturated_into();
				let amount = rate.saturating_mul(bytes_balance);
				if amount.is_zero() {
					continue;
				}
				if let Some(relayer) = Self::relayer_for(&mw.message) {
					match T::ReputationAsset::mint_into(&relayer, amount) {
						Ok(_) =>
							Self::deposit_event(Event::ReputationMinted { relayer, bytes, amount }),
						Err(err) => log::warn!(
							target: "messaging-incentives",
							"reputation mint failed for {bytes}b: {err:?}",
						),
					}
				}
			}
		}
		Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
	}
}

impl<T: Config> IncentivesManager for Pallet<T> {
	fn reset_incentives() {}
}
