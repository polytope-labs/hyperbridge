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
//! Per-`(source_chain, app)` prepaid bandwidth balances credited via
//! purchase messages from `BandwidthMarket.sol`. Exposes a
//! [`BandwidthGate`] the runtime's ISMP router consults for every
//! cross-chain message; messages from apps without sufficient balance
//! are rejected before processing.
//!
//! Bandwidth has no expiry — apps recharge by purchasing more. See
//! `miniIssue.md` for the end-to-end design.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use polkadot_sdk::frame_support::traits::UnixTime;
use polkadot_sdk::*;

pub mod abi;
pub mod types;

pub use pallet::*;
pub use types::{AllowanceState, AppKey, BandwidthGate, EnforcementMode, GateError};

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use crate::abi::{decode_purchase_msg, PurchaseMessage};
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

    /// Used by `BandwidthMarket.sol` as the `to` field of purchase
    /// messages and as the [`PalletId`] for this pallet's sovereign
    /// account.
    pub const PALLET_BANDWIDTH_ID: &[u8] = b"BWMARKET";
    pub const PALLET_BANDWIDTH: PalletId = PalletId(*b"BWMARKET");

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Reuses `pallet_ismp::Config::{AdminOrigin, TimestampProvider}` to
    /// avoid duplicating already-root-controlled state.
    #[pallet::config]
    pub trait Config:
        polkadot_sdk::frame_system::Config<RuntimeEvent: From<Event<Self>>> + pallet_ismp::Config
    {
    }

    /// Authorised purchase contract per source chain — purchase messages
    /// whose `request.from` doesn't match are rejected.
    #[pallet::storage]
    pub type BandwidthMarkets<T: Config> =
        StorageMap<_, Twox64Concat, StateMachine, H160, OptionQuery>;

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

    /// Apps that bypass the gate entirely — used during phased rollout
    /// for protocol-sponsored applications that haven't migrated yet.
    #[pallet::storage]
    pub type Allowlist<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        StateMachine,
        Blake2_128Concat,
        AppKey,
        (),
        OptionQuery,
    >;

    #[pallet::storage]
    pub type Mode<T: Config> = StorageValue<_, EnforcementMode, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        MarketRegistered { source: StateMachine, market: H160 },
        BandwidthCredited {
            source: StateMachine,
            app: AppKey,
            bytes_purchased: u128,
            amount_paid_18d: u128,
            new_balance: u128,
        },
        BandwidthConsumed { source: StateMachine, app: AppKey, bytes: u128, remaining: u128 },
        /// Observe mode would have rejected this in Enforce mode.
        WouldReject { source: StateMachine, app: AppKey, required: u128, remaining: u128 },
        ModeChanged { mode: EnforcementMode },
        AllowlistChanged { source: StateMachine, app: AppKey, on: bool },
        /// Credited out-of-band via [`Call::force_credit`].
        ForceCredited { source: StateMachine, app: AppKey, bytes: u128, new_balance: u128 },
    }

    #[pallet::error]
    pub enum Error<T> {
        UnknownMarket,
        UnauthorizedMarket,
        InvalidPurchaseBody,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register the authoritative `BandwidthMarket` contract for a
        /// source chain — overwrites any prior registration.
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
        pub fn set_enforcement_mode(
            origin: OriginFor<T>,
            mode: EnforcementMode,
        ) -> DispatchResult {
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

        /// Out-of-band credit for migrations / refunds.
        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn force_credit(
            origin: OriginFor<T>,
            source: StateMachine,
            app: AppKey,
            bytes: u128,
        ) -> DispatchResult {
            <T as pallet_ismp::Config>::AdminOrigin::ensure_origin(origin)?;
            let new_balance = Self::credit_inner(&source, &app, bytes, 0);
            Self::deposit_event(Event::ForceCredited { source, app, bytes, new_balance });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn allowance(source: &StateMachine, app: &[u8]) -> Option<AllowanceState> {
            Allowance::<T>::get(source, AppKey::truncate_from(app.to_vec()))
        }

        pub fn remaining(source: &StateMachine, app: &[u8]) -> u128 {
            Self::allowance(source, app).map(|s| s.remaining_bytes).unwrap_or_default()
        }

        /// Returns the new balance — both the IsmpModule path and
        /// `force_credit` need it for their respective events.
        fn credit_inner(
            source: &StateMachine,
            app: &AppKey,
            bytes: u128,
            amount_paid_18d: u128,
        ) -> u128 {
            let new_balance = Allowance::<T>::mutate(source, app, |slot| match slot.as_mut() {
                Some(s) => {
                    s.remaining_bytes = s.remaining_bytes.saturating_add(bytes);
                    s.purchased_lifetime = s.purchased_lifetime.saturating_add(bytes);
                    s.remaining_bytes
                }
                None => {
                    *slot = Some(AllowanceState {
                        remaining_bytes: bytes,
                        purchased_lifetime: bytes,
                        last_consumed_at: 0,
                    });
                    bytes
                }
            });

            Self::deposit_event(Event::BandwidthCredited {
                source: *source,
                app: app.clone(),
                bytes_purchased: bytes,
                amount_paid_18d,
                new_balance,
            });

            new_balance
        }

        /// Lets the runtime's ISMP router skip the gate for purchases —
        /// otherwise a freshly-deployed app couldn't recharge after going
        /// to zero.
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
                anyhow::anyhow!(format!(
                    "no bandwidth market registered for {:?}",
                    request.source
                ))
            })?;

            // Sole identity check — must be exact.
            if request.from != market.0.to_vec() {
                return Err(anyhow::anyhow!(format!(
                    "purchase from unauthorised sender on {:?}: expected {:x?}, got {:x?}",
                    request.source, market.0, request.from
                )));
            }

            let PurchaseMessage { app, bytes_purchased, amount_paid_18d } =
                decode_purchase_msg(&request.body)?;
            let key = AppKey::truncate_from(app.0.to_vec());
            Self::credit_inner(&request.source, &key, bytes_purchased, amount_paid_18d);

            Ok(Weight::zero())
        }

        fn on_response(&self, _response: Response) -> Result<Weight, anyhow::Error> {
            Err(ismp::Error::CannotHandleMessage.into())
        }

        /// Purchase messages dispatch with `timeout = 0`, so this should
        /// never fire. If it does, swallow it — refund is off-chain.
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

        // Only mutate on a successful Enforce consume — an Observe-mode
        // shortfall must not silently drain the balance.
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
                    }
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
            }
            (EnforcementMode::Enforce, Ok(())) => {
                Self::deposit_event(Event::BandwidthConsumed {
                    source: *source,
                    app: key,
                    bytes: need,
                    remaining: remaining_after,
                });
                Ok(())
            }
            // Observe+Ok, Disabled, Enforce+Err: propagate as-is.
            _ => decision,
        }
    }
}
