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

//! # pallet-bandwidth
//!
//! Prepaid `(chain, app)` byte balances credited by tier purchases
//! from `BandwidthManager.sol`. Each purchase carries its own
//! `app_chain`, so any deployment can sponsor any app on any chain.
//!
//! [`BandwidthGate`] is the hook the runtime's ISMP router consults
//! for every message; insufficient balance → rejected.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use polkadot_sdk::frame_support::traits::UnixTime;
use polkadot_sdk::*;

pub mod abi;
pub mod types;

pub use pallet::*;
pub use types::{
	AllowanceState, AppKey, BandwidthBytes, BandwidthGate, EnforcementMode, GateError, TierIndex,
};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::abi::PurchaseMessage;
	use alloc::format;
	use frame_support::{pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;
	use ismp::{
		host::StateMachine,
		module::IsmpModule,
		router::{PostRequest, Response, Timeout},
	};
	use primitive_types::H160;
	use sp_runtime::Weight;

	/// `to` field on purchase messages; also the sovereign `PalletId`.
	pub const PALLET_BANDWIDTH: PalletId = PalletId(*b"BWMARKET");

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<RuntimeEvent: From<Event<Self>>> + pallet_ismp::Config
	{
	}

	/// Authorised purchase contract per source chain. A purchase whose
	/// `request.from` doesn't match this is rejected.
	#[pallet::storage]
	pub type BandwidthMarkets<T: Config> =
		StorageMap<_, Twox64Concat, StateMachine, H160, OptionQuery>;

	/// Keyed by `app_chain` from the purchase message, not by
	/// `request.source` — that decoupling is what enables sponsorship.
	#[pallet::storage]
	pub type Allowance<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		StateMachine,
		Blake2_128Concat,
		AppKey,
		AllowanceState,
		OptionQuery,
	>;

	/// Apps that bypass the gate. Used during phased rollout for
	/// protocol-sponsored apps that haven't migrated.
	#[pallet::storage]
	pub type Allowlist<T: Config> =
		StorageDoubleMap<_, Twox64Concat, StateMachine, Blake2_128Concat, AppKey, (), OptionQuery>;

	#[pallet::storage]
	pub type Tiers<T: Config> = StorageMap<_, Twox64Concat, TierIndex, BandwidthBytes, OptionQuery>;

	#[pallet::storage]
	pub type Mode<T: Config> = StorageValue<_, EnforcementMode, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MarketRegistered {
			source: StateMachine,
			market: H160,
		},
		TierSet {
			tier: TierIndex,
			bytes: BandwidthBytes,
		},
		BandwidthCredited {
			app_chain: StateMachine,
			app: AppKey,
			/// Chain that paid; differs from `app_chain` on sponsorship.
			paid_from: StateMachine,
			tier: TierIndex,
			bytes_credited: BandwidthBytes,
			new_balance: BandwidthBytes,
		},
		BandwidthConsumed {
			source: StateMachine,
			app: AppKey,
			bytes: u128,
			remaining: u128,
		},
		/// Would-have-rejected event under Observe mode.
		WouldReject {
			source: StateMachine,
			app: AppKey,
			required: u128,
			remaining: u128,
		},
		ModeChanged {
			mode: EnforcementMode,
		},
		AllowlistChanged {
			source: StateMachine,
			app: AppKey,
			on: bool,
		},
		ForceCredited {
			app_chain: StateMachine,
			app: AppKey,
			bytes: BandwidthBytes,
			new_balance: BandwidthBytes,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		UnknownMarket,
		UnauthorizedMarket,
		InvalidPurchaseBody,
		UnknownTier,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_market(
			origin: OriginFor<T>,
			source: StateMachine,
			market: H160,
		) -> DispatchResult {
			<T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
			BandwidthMarkets::<T>::insert(source, market);
			Self::deposit_event(Event::MarketRegistered { source, market });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_enforcement_mode(origin: OriginFor<T>, mode: EnforcementMode) -> DispatchResult {
			<T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
			Mode::<T>::put(mode);
			Self::deposit_event(Event::ModeChanged { mode });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_allowlist(
			origin: OriginFor<T>,
			source: StateMachine,
			app: AppKey,
			on: bool,
		) -> DispatchResult {
			<T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
			if on {
				Allowlist::<T>::insert(&source, &app, ());
			} else {
				Allowlist::<T>::remove(&source, &app);
			}
			Self::deposit_event(Event::AllowlistChanged { source, app, on });
			Ok(())
		}

		/// Admin-only out-of-band credit (migrations, refunds).
		#[pallet::call_index(3)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn force_credit(
			origin: OriginFor<T>,
			app_chain: StateMachine,
			app: AppKey,
			bytes: BandwidthBytes,
		) -> DispatchResult {
			<T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
			let new_balance = Self::credit_inner(&app_chain, &app, bytes);
			Self::deposit_event(Event::ForceCredited { app_chain, app, bytes, new_balance });
			Ok(())
		}

		/// Pass `bytes = 0` to revoke a tier.
		#[pallet::call_index(4)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_tier(
			origin: OriginFor<T>,
			tier: TierIndex,
			bytes: BandwidthBytes,
		) -> DispatchResult {
			<T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
			if bytes == 0 {
				Tiers::<T>::remove(tier);
			} else {
				Tiers::<T>::insert(tier, bytes);
			}
			Self::deposit_event(Event::TierSet { tier, bytes });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn allowance(app_chain: &StateMachine, app: &[u8]) -> Option<AllowanceState> {
			Allowance::<T>::get(app_chain, AppKey::truncate_from(app.to_vec()))
		}

		pub fn remaining(app_chain: &StateMachine, app: &[u8]) -> u128 {
			Self::allowance(app_chain, app).map(|s| s.remaining_bytes).unwrap_or_default()
		}

		fn credit_inner(
			app_chain: &StateMachine,
			app: &AppKey,
			bytes: BandwidthBytes,
		) -> BandwidthBytes {
			Allowance::<T>::mutate(app_chain, app, |slot| match slot.as_mut() {
				Some(s) => {
					s.remaining_bytes = s.remaining_bytes.saturating_add(bytes);
					s.purchased_lifetime = s.purchased_lifetime.saturating_add(bytes);
					s.remaining_bytes
				},
				None => {
					*slot = Some(AllowanceState {
						remaining_bytes: bytes,
						purchased_lifetime: bytes,
						last_consumed_at: 0,
					});
					bytes
				},
			})
		}

		/// The router uses this to skip the gate on purchases —
		/// otherwise a depleted app couldn't recharge.
		pub fn is_purchase_message(request: &PostRequest) -> bool {
			BandwidthMarkets::<T>::get(&request.source)
				.map(|m| request.from == m.0.to_vec())
				.unwrap_or(false)
		}
	}

	impl<T: Config> Default for Pallet<T> {
		fn default() -> Self {
			Self(PhantomData)
		}
	}

	impl<T: Config> IsmpModule for Pallet<T> {
		fn on_accept(&self, request: PostRequest) -> Result<Weight, anyhow::Error> {
			let market = BandwidthMarkets::<T>::get(&request.source).ok_or_else(|| {
				anyhow::anyhow!(format!("no bandwidth market registered for {:?}", request.source))
			})?;

			if request.from != market.0.to_vec() {
				return Err(anyhow::anyhow!(format!(
					"purchase from unauthorised sender on {:?}: expected {:x?}, got {:x?}",
					request.source, market.0, request.from
				)));
			}

			let msg = PurchaseMessage::try_from(request.body.as_slice())?;
			let bytes_for_tier = Tiers::<T>::get(msg.tier)
				.ok_or_else(|| anyhow::anyhow!(format!("unknown tier {}", msg.tier)))?;

			let key = AppKey::truncate_from(msg.app);
			let new_balance = Self::credit_inner(&msg.app_chain, &key, bytes_for_tier);

			Self::deposit_event(Event::BandwidthCredited {
				app_chain: msg.app_chain,
				app: key,
				paid_from: request.source,
				tier: msg.tier,
				bytes_credited: bytes_for_tier,
				new_balance,
			});

			Ok(Weight::zero())
		}

		fn on_response(&self, _response: Response) -> Result<Weight, anyhow::Error> {
			Err(ismp::Error::CannotHandleMessage.into())
		}

		/// Purchases dispatch with `timeout = 0`; swallow if it fires.
		fn on_timeout(&self, _timeout: Timeout) -> Result<Weight, anyhow::Error> {
			Ok(Weight::zero())
		}
	}
}

impl<T: Config> BandwidthGate for Pallet<T> {
	fn try_consume(
		source: &ismp::host::StateMachine,
		app: &[u8],
		bytes: u32,
	) -> Result<(), GateError> {
		let mode = Mode::<T>::get();
		if matches!(mode, EnforcementMode::Disabled) {
			return Ok(());
		}

		let key = AppKey::truncate_from(app.to_vec());
		if Allowlist::<T>::contains_key(source, &key) {
			return Ok(());
		}

		let need: u128 = bytes.into();
		let now_secs = <T as pallet_ismp::Config>::TimestampProvider::now().as_secs();

		// Only deduct on Enforce success: an Observe shortfall must not
		// silently drain the balance.
		let (decision, remaining_after): (Result<(), GateError>, u128) =
			Allowance::<T>::mutate(source, &key, |slot| -> (Result<(), GateError>, u128) {
				match slot.as_mut() {
					None => (Err(GateError::NoAllowance), 0),
					Some(state) => {
						if state.remaining_bytes < need {
							(
								Err(GateError::Insufficient {
									remaining: state.remaining_bytes,
									required: need,
								}),
								state.remaining_bytes,
							)
						} else {
							if matches!(mode, EnforcementMode::Enforce) {
								state.remaining_bytes -= need;
								state.last_consumed_at = now_secs;
							}
							(Ok(()), state.remaining_bytes)
						}
					},
				}
			});

		match (mode, &decision) {
			(EnforcementMode::Observe, Err(err)) => {
				let (required, remaining) = match err {
					GateError::NoAllowance => (need, 0u128),
					GateError::Insufficient { remaining, required } => (*required, *remaining),
				};
				Self::deposit_event(Event::WouldReject {
					source: *source,
					app: key,
					required,
					remaining,
				});
				Ok(())
			},
			(EnforcementMode::Enforce, Ok(())) => {
				Self::deposit_event(Event::BandwidthConsumed {
					source: *source,
					app: key,
					bytes: need,
					remaining: remaining_after,
				});
				Ok(())
			},
			_ => decision,
		}
	}
}
