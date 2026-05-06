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
//! Prepaid `(chain, app, tier)` byte balances credited by tier
//! purchases from `BandwidthManager.sol`. Each purchase carries its
//! own `app_chain`, so any deployment can sponsor any app on any
//! chain.
//!
//! Per-tier rows let same-tier re-buys stack their bytes and expiry,
//! while different tiers keep independent expiries. The gate consumes
//! FIFO by `expires_at` so users burn what's about to expire first.
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
	AllowanceState, AppKey, BandwidthBytes, BandwidthGate, EnforcementMode, GateError, TierConfig,
	TierIndex,
};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::abi::PurchaseMessage;
	use alloc::{format, vec::Vec};
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

	/// Keyed by `(app_chain, app, tier)`. `app_chain` comes from the
	/// purchase message — not `request.source` — so a payer chain can
	/// sponsor an app that lives elsewhere.
	#[pallet::storage]
	pub type Allowance<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, StateMachine>,
			NMapKey<Blake2_128Concat, AppKey>,
			NMapKey<Twox64Concat, TierIndex>,
		),
		AllowanceState,
		OptionQuery,
	>;

	/// Apps that bypass the gate. Used during phased rollout for
	/// protocol-sponsored apps that haven't migrated.
	#[pallet::storage]
	pub type Allowlist<T: Config> =
		StorageDoubleMap<_, Twox64Concat, StateMachine, Blake2_128Concat, AppKey, (), OptionQuery>;

	#[pallet::storage]
	pub type Tiers<T: Config> = StorageMap<_, Twox64Concat, TierIndex, TierConfig, OptionQuery>;

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
			duration_secs: u64,
		},
		BandwidthCredited {
			app_chain: StateMachine,
			app: AppKey,
			/// Chain that paid; differs from `app_chain` on sponsorship.
			paid_from: StateMachine,
			tier: TierIndex,
			bytes_credited: BandwidthBytes,
			tier_balance: BandwidthBytes,
			expires_at: u64,
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
			tier: TierIndex,
			bytes: BandwidthBytes,
			tier_balance: BandwidthBytes,
			expires_at: u64,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		UnknownMarket,
		UnauthorizedMarket,
		InvalidPurchaseBody,
		UnknownTier,
		InvalidTierConfig,
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

		/// Admin-only out-of-band credit (migrations, refunds). Targets
		/// a specific tier bucket so it follows the same stack/reset
		/// rules as a real purchase.
		#[pallet::call_index(3)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn force_credit(
			origin: OriginFor<T>,
			app_chain: StateMachine,
			app: AppKey,
			tier: TierIndex,
			bytes: BandwidthBytes,
			duration_secs: u64,
		) -> DispatchResult {
			<T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
			let (tier_balance, expires_at) =
				Self::credit_bucket(&app_chain, &app, tier, bytes, duration_secs);
			Self::deposit_event(Event::ForceCredited {
				app_chain,
				app,
				tier,
				bytes,
				tier_balance,
				expires_at,
			});
			Ok(())
		}

		/// Pass `bytes = 0` to revoke a tier. `duration_secs` must be
		/// non-zero on creation so purchases produce a real expiry.
		#[pallet::call_index(4)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_tier(
			origin: OriginFor<T>,
			tier: TierIndex,
			bytes: BandwidthBytes,
			duration_secs: u64,
		) -> DispatchResult {
			<T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
			if bytes == 0 {
				Tiers::<T>::remove(tier);
			} else {
				ensure!(duration_secs > 0, Error::<T>::InvalidTierConfig);
				Tiers::<T>::insert(tier, TierConfig { bytes, duration_secs });
			}
			Self::deposit_event(Event::TierSet { tier, bytes, duration_secs });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// All non-expired buckets for `(app_chain, app)`, sorted FIFO
		/// by expiry. Lazy: callers should treat returned snapshot as a
		/// read-only view.
		pub fn allowances(
			app_chain: &StateMachine,
			app: &[u8],
		) -> Vec<(TierIndex, AllowanceState)> {
			let key = AppKey::truncate_from(app.to_vec());
			let now = <T as pallet_ismp::Config>::TimestampProvider::now().as_secs();
			let mut buckets: Vec<(TierIndex, AllowanceState)> =
				Allowance::<T>::iter_prefix((app_chain, &key))
					.filter(|(_, s)| s.expires_at > now)
					.collect();
			buckets.sort_by_key(|(_, s)| s.expires_at);
			buckets
		}

		/// Sum of all live buckets — what the gate would charge against.
		pub fn remaining(app_chain: &StateMachine, app: &[u8]) -> u128 {
			Self::allowances(app_chain, app).into_iter().map(|(_, s)| s.remaining_bytes).sum()
		}

		/// Stack-or-reset: if the bucket is still live, add bytes and
		/// push expiry forward by `duration_secs`. If expired or
		/// missing, start a fresh window from `now`.
		fn credit_bucket(
			app_chain: &StateMachine,
			app: &AppKey,
			tier: TierIndex,
			bytes: BandwidthBytes,
			duration_secs: u64,
		) -> (BandwidthBytes, u64) {
			let now = <T as pallet_ismp::Config>::TimestampProvider::now().as_secs();
			Allowance::<T>::mutate((app_chain, app, tier), |slot| {
				let next = match slot.as_ref() {
					Some(s) if s.expires_at > now => AllowanceState {
						remaining_bytes: s.remaining_bytes.saturating_add(bytes),
						expires_at: s.expires_at.saturating_add(duration_secs),
					},
					_ => AllowanceState {
						remaining_bytes: bytes,
						expires_at: now.saturating_add(duration_secs),
					},
				};
				let result = (next.remaining_bytes, next.expires_at);
				*slot = Some(next);
				result
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
			let cfg = Tiers::<T>::get(msg.tier)
				.ok_or_else(|| anyhow::anyhow!(format!("unknown tier {}", msg.tier)))?;

			let key = AppKey::truncate_from(msg.app);
			let (tier_balance, expires_at) =
				Self::credit_bucket(&msg.app_chain, &key, msg.tier, cfg.bytes, cfg.duration_secs);

			Self::deposit_event(Event::BandwidthCredited {
				app_chain: msg.app_chain,
				app: key,
				paid_from: request.source,
				tier: msg.tier,
				bytes_credited: cfg.bytes,
				tier_balance,
				expires_at,
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
		use alloc::vec::Vec;

		let mode = Mode::<T>::get();
		if matches!(mode, EnforcementMode::Disabled) {
			return Ok(());
		}

		let key = AppKey::truncate_from(app.to_vec());
		if Allowlist::<T>::contains_key(source, &key) {
			return Ok(());
		}

		let need: u128 = bytes.into();
		let now = <T as pallet_ismp::Config>::TimestampProvider::now().as_secs();

		// Materialize so we can sort, then sweep expired rows. We do
		// the sweep unconditionally (even on shortfall) so storage
		// can't grow without bound from abandoned tiers.
		let mut live: Vec<(TierIndex, AllowanceState)> = Vec::new();
		for (tier, state) in Allowance::<T>::iter_prefix((source, &key)) {
			if state.expires_at <= now {
				Allowance::<T>::remove((source, &key, tier));
			} else {
				live.push((tier, state));
			}
		}

		if live.is_empty() {
			return finalize_shortfall::<T>(mode, source, &key, need, 0, GateError::NoAllowance);
		}

		live.sort_by_key(|(_, s)| s.expires_at);
		let total: u128 = live.iter().map(|(_, s)| s.remaining_bytes).sum();

		if total < need {
			return finalize_shortfall::<T>(
				mode,
				source,
				&key,
				need,
				total,
				GateError::Insufficient { remaining: total, required: need },
			);
		}

		// Sufficient. Observe is a noop on success — nothing to write,
		// no event needed.
		if matches!(mode, EnforcementMode::Observe) {
			return Ok(());
		}

		let mut left = need;
		for (tier, mut state) in live {
			if left == 0 {
				break;
			}
			let take = state.remaining_bytes.min(left);
			state.remaining_bytes -= take;
			left -= take;
			if state.remaining_bytes == 0 {
				Allowance::<T>::remove((source, &key, tier));
			} else {
				Allowance::<T>::insert((source, &key, tier), state);
			}
		}

		Self::deposit_event(Event::BandwidthConsumed {
			source: *source,
			app: key,
			bytes: need,
			remaining: total - need,
		});

		Ok(())
	}
}

/// Shared shortfall path for both Observe (emit + Ok) and Enforce
/// (return Err) — keeps the gate body free of nested matches.
fn finalize_shortfall<T: Config>(
	mode: EnforcementMode,
	source: &ismp::host::StateMachine,
	key: &AppKey,
	required: u128,
	remaining: u128,
	err: GateError,
) -> Result<(), GateError> {
	match mode {
		EnforcementMode::Observe => {
			Pallet::<T>::deposit_event(Event::WouldReject {
				source: *source,
				app: key.clone(),
				required,
				remaining,
			});
			Ok(())
		},
		_ => Err(err),
	}
}
