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
//! Tiers are a closed enum, and each `(chain, app)` row holds a
//! `BoundedBTreeMap<TierIndex, AllowanceState>` — bounded by the
//! variant count, so the gate is one storage read instead of an
//! `iter_prefix`. Same-tier re-buys stack their bytes and expiry;
//! different tiers keep independent expiries. The gate consumes FIFO
//! by `expires_at` so users burn what's about to expire first.
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
	AllowanceState, AppKey, BandwidthBytes, BandwidthGate, EnforcementMode, ForceCreditParams,
	GateError, MaxTiers, TierConfig, TierIndex,
};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::abi::PurchaseMessage;
	use alloc::{format, vec::Vec};
	use frame_support::{pallet_prelude::*, BoundedBTreeMap, PalletId};
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

	pub type TierMap = BoundedBTreeMap<TierIndex, AllowanceState, MaxTiers>;

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

	/// Keyed by `app_chain` from the purchase message — *not*
	/// `request.source` — so a payer chain can sponsor an app that
	/// lives elsewhere. The inner `BoundedBTreeMap` is bounded by the
	/// `TierIndex` variant count, so the gate touches one storage row.
	#[pallet::storage]
	pub type Allowance<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		StateMachine,
		Blake2_128Concat,
		AppKey,
		TierMap,
		ValueQuery,
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
			config: Option<TierConfig>,
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

		/// Admin-only out-of-band credit (migrations, refunds). Same
		/// stack/reset rules as a real purchase.
		#[pallet::call_index(3)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn force_credit(origin: OriginFor<T>, params: ForceCreditParams) -> DispatchResult {
			<T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
			let (tier_balance, expires_at) = Self::credit_bucket(
				&params.app_chain,
				&params.app,
				params.tier,
				params.bytes,
				params.duration_secs,
			);
			Self::deposit_event(Event::ForceCredited {
				app_chain: params.app_chain,
				app: params.app,
				tier: params.tier,
				bytes: params.bytes,
				tier_balance,
				expires_at,
			});
			Ok(())
		}

		/// Pass `config: None` to revoke. Non-zero `bytes` requires a
		/// non-zero `duration_secs` so a purchase can't expire on
		/// creation.
		#[pallet::call_index(4)]
		#[pallet::weight(T::DbWeight::get().writes(1))]
		pub fn set_tier(
			origin: OriginFor<T>,
			tier: TierIndex,
			config: Option<TierConfig>,
		) -> DispatchResult {
			<T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
			match config {
				None => Tiers::<T>::remove(tier),
				Some(cfg) => {
					ensure!(
						cfg.bytes > 0 && cfg.duration_secs > 0,
						Error::<T>::InvalidTierConfig
					);
					Tiers::<T>::insert(tier, cfg);
				},
			}
			Self::deposit_event(Event::TierSet { tier, config });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// All non-expired buckets for `(app_chain, app)`, sorted FIFO
		/// by expiry. Returned snapshot is a read-only view.
		pub fn allowances(
			app_chain: &StateMachine,
			app: &[u8],
		) -> Vec<(TierIndex, AllowanceState)> {
			let key = AppKey::truncate_from(app.to_vec());
			let now = <T as pallet_ismp::Config>::TimestampProvider::now().as_secs();
			let map = Allowance::<T>::get(app_chain, &key);
			let mut buckets: Vec<(TierIndex, AllowanceState)> = map
				.into_iter()
				.filter(|(_, s)| s.expires_at > now)
				.collect();
			buckets.sort_by_key(|(_, s)| s.expires_at);
			buckets
		}

		/// Sum of all live buckets — what the gate would charge against.
		pub fn remaining(app_chain: &StateMachine, app: &[u8]) -> u128 {
			Self::allowances(app_chain, app).into_iter().map(|(_, s)| s.remaining_bytes).sum()
		}

		/// Stack-or-reset: live bucket → bytes add, expiry pushes out by
		/// `duration_secs`. Expired/missing → fresh window from now.
		fn credit_bucket(
			app_chain: &StateMachine,
			app: &AppKey,
			tier: TierIndex,
			bytes: BandwidthBytes,
			duration_secs: u64,
		) -> (BandwidthBytes, u64) {
			let now = <T as pallet_ismp::Config>::TimestampProvider::now().as_secs();
			Allowance::<T>::mutate(app_chain, app, |map| {
				let next = match map.get(&tier) {
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
				// Bounded by `MaxTiers` = `TierIndex` variant count, so
				// `try_insert` can never hit the bound.
				let _ = map.try_insert(tier, next);
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
			let tier = TierIndex::try_from(msg.tier)
				.map_err(|_| anyhow::anyhow!(format!("unknown tier discriminant {}", msg.tier)))?;
			let cfg = Tiers::<T>::get(tier)
				.ok_or_else(|| anyhow::anyhow!(format!("tier {:?} is not configured", tier)))?;

			let bytes = cfg.bytes.saturating_mul(msg.months as u128);
			let duration = cfg.duration_secs.saturating_mul(msg.months as u64);

			let key = AppKey::truncate_from(msg.app);
			let (tier_balance, expires_at) =
				Self::credit_bucket(&msg.app_chain, &key, tier, bytes, duration);

			Self::deposit_event(Event::BandwidthCredited {
				app_chain: msg.app_chain,
				app: key,
				paid_from: request.source,
				tier,
				bytes_credited: bytes,
				tier_balance,
				expires_at,
			});

			Ok(Weight::zero())
		}

		fn on_response(&self, _response: Response) -> Result<Weight, anyhow::Error> {
			Err(ismp::Error::CannotHandleMessage.into())
		}

		/// Purchases dispatch with `timeout = 0`. If `on_timeout` ever
		/// fires it's an invariant violation, not a noop.
		fn on_timeout(&self, _timeout: Timeout) -> Result<Weight, anyhow::Error> {
			Err(anyhow::anyhow!("pallet-bandwidth purchases are non-timeouting"))
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

		let outcome: Result<u128, GateError> = pallet::Allowance::<T>::mutate(source, &key, |map| {
			let expired: Vec<TierIndex> = map
				.iter()
				.filter_map(|(t, s)| (s.expires_at <= now).then_some(*t))
				.collect();
			for t in expired {
				map.remove(&t);
			}

			if map.is_empty() {
				return Err(GateError::NoAllowance);
			}

			let total: u128 = map.values().map(|s| s.remaining_bytes).sum();
			if total < need {
				return Err(GateError::Insufficient { remaining: total, required: need });
			}

			if matches!(mode, EnforcementMode::Enforce) {
				let mut order: Vec<(TierIndex, u64)> =
					map.iter().map(|(t, s)| (*t, s.expires_at)).collect();
				order.sort_by_key(|(_, e)| *e);

				let mut left = need;
				let mut drained: Vec<TierIndex> = Vec::new();
				for (t, _) in order {
					if left == 0 {
						break;
					}
					if let Some(state) = map.get_mut(&t) {
						let take = state.remaining_bytes.min(left);
						state.remaining_bytes = state.remaining_bytes.saturating_sub(take);
						left = left.saturating_sub(take);
						if state.remaining_bytes == 0 {
							drained.push(t);
						}
					}
				}
				for t in drained {
					map.remove(&t);
				}
			}

			Ok(total)
		});

		match (mode, outcome) {
			(EnforcementMode::Observe, Err(err)) => {
				let (required, remaining) = match err {
					GateError::NoAllowance => (need, 0u128),
					GateError::Insufficient { remaining, required } => (required, remaining),
				};
				Self::deposit_event(Event::WouldReject {
					source: *source,
					app: key,
					required,
					remaining,
				});
				Ok(())
			},
			(EnforcementMode::Enforce, Ok(total)) => {
				Self::deposit_event(Event::BandwidthConsumed {
					source: *source,
					app: key,
					bytes: need,
					remaining: total - need,
				});
				Ok(())
			},
			(_, result) => result.map(|_| ()),
		}
	}
}
