// Copyright (c) 2025 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

//! Integration tests for `pallet-bandwidth` against the testsuite mock
//! runtime — same harness as `pallet_hyperbridge.rs`.

use polkadot_sdk::*;

use ismp::{host::StateMachine, module::IsmpModule, router::PostRequest};
use sp_core::H160;

use pallet_bandwidth::{
    abi::PurchaseMessage,
    pallet::{Allowance, BandwidthMarkets, Mode, Tiers, PALLET_BANDWIDTH},
    AppKey, BandwidthGate, EnforcementMode, GateError, TierConfig, TierIndex,
};

use crate::runtime::{new_test_ext, set_timestamp, Bandwidth, RuntimeOrigin, Test};

/// `APP_CHAIN` is where the BaseIG-style app being funded lives.
/// `PAYER_CHAIN` is the chain whose `BandwidthManager` dispatches the
/// purchase. They differ in the cross-chain-sponsorship case.
const APP_CHAIN: StateMachine = StateMachine::Evm(8453); // Base
const PAYER_CHAIN: StateMachine = StateMachine::Evm(137); // Polygon
const MARKET: H160 = H160([0xAA; 20]);
const APP: H160 = H160([0xBB; 20]);

const TIER1: TierIndex = 1;
const TIER2: TierIndex = 2;
const TIER1_BYTES: u128 = 1_000;
const TIER2_BYTES: u128 = 5_000;
/// Mock 28d window; chosen large enough that `t0 + DURATION` fits
/// comfortably in the test harness's `u64` clock.
const MONTH_SECS: u64 = 28 * 24 * 60 * 60;
const QUARTER_SECS: u64 = 3 * MONTH_SECS;

fn app_key() -> AppKey {
    AppKey::truncate_from(APP.0.to_vec())
}

/// `set_timestamp` takes milliseconds; tests think in seconds.
fn jump_to(secs: u64) {
    set_timestamp::<Test>(secs.saturating_mul(1_000));
}

/// Mirrors what `EvmHost.dispatch` on the payer chain would emit. The
/// `payer_chain` is the chain hosting the `BandwidthManager` that
/// dispatched the purchase; `app_chain` is the chain whose allowance
/// gets credited.
fn purchase_request(
    payer_chain: StateMachine,
    sender: H160,
    tier: TierIndex,
    app_chain: StateMachine,
) -> PostRequest {
    let body: Vec<u8> = (&PurchaseMessage { app: APP.0.to_vec(), tier, app_chain }).into();

    PostRequest {
        source: payer_chain,
        dest: StateMachine::Polkadot(100),
        nonce: 0,
        from: sender.0.to_vec(),
        to: PALLET_BANDWIDTH.0.to_vec(),
        timeout_timestamp: 0,
        body,
    }
}

fn register_market(chain: StateMachine) {
    Bandwidth::set_market(RuntimeOrigin::root(), chain, MARKET).unwrap();
}

fn configure_tier(tier: TierIndex, bytes: u128, duration_secs: u64) {
    Bandwidth::set_tier(RuntimeOrigin::root(), tier, bytes, duration_secs).unwrap();
}

fn bucket(chain: StateMachine, tier: TierIndex) -> Option<pallet_bandwidth::AllowanceState> {
    Allowance::<Test>::get((chain, app_key(), tier))
}

#[test]
fn purchase_credits_allowance_with_expiry() {
    new_test_ext().execute_with(|| {
        jump_to(1_000);
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();

        let state = bucket(APP_CHAIN, TIER1).expect("bucket must exist");
        assert_eq!(state.remaining_bytes, TIER1_BYTES);
        assert_eq!(state.expires_at, 1_000 + MONTH_SECS);
    });
}

/// Buy from Polygon for an app on Base — the credit lands on Base.
/// This is the case the per-`request.source` design got wrong.
#[test]
fn cross_chain_purchase_credits_app_chain_not_payer_chain() {
    new_test_ext().execute_with(|| {
        jump_to(1_000);
        register_market(PAYER_CHAIN);
        configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

        Bandwidth::default()
            .on_accept(purchase_request(PAYER_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();

        // Credit lands on the app's home chain ...
        let state = bucket(APP_CHAIN, TIER1).expect("bucket must exist");
        assert_eq!(state.remaining_bytes, TIER1_BYTES);

        // ... NOT under the payer's chain.
        assert!(bucket(PAYER_CHAIN, TIER1).is_none());

        // And the gate (which keys on `request.source` — the chain the
        // app lives on) finds the balance.
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 100),
            Ok(())
        );
    });
}

/// Same tier bought twice 5 days apart: bytes stack and the expiry of
/// the second window starts where the first one ended (David's
/// "rollover"). Buying 6 months upfront falls out of this rule.
#[test]
fn same_tier_repurchase_stacks_bytes_and_extends_expiry() {
    new_test_ext().execute_with(|| {
        let t0 = 1_000_000_u64;
        let five_days = 5 * 24 * 60 * 60;
        jump_to(t0);
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        let first_expiry = bucket(APP_CHAIN, TIER1).unwrap().expires_at;
        assert_eq!(first_expiry, t0 + MONTH_SECS);

        jump_to(t0 + five_days);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();

        let state = bucket(APP_CHAIN, TIER1).unwrap();
        assert_eq!(state.remaining_bytes, 2 * TIER1_BYTES, "bytes stack");
        assert_eq!(
            state.expires_at,
            first_expiry + MONTH_SECS,
            "second window starts when the first ended, not when bought",
        );
    });
}

/// Different tiers live in independent buckets with independent expiries.
#[test]
fn different_tier_purchases_create_separate_buckets() {
    new_test_ext().execute_with(|| {
        let t0 = 1_000_000_u64;
        jump_to(t0);
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
        configure_tier(TIER2, TIER2_BYTES, QUARTER_SECS);

        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER2, APP_CHAIN))
            .unwrap();

        let b1 = bucket(APP_CHAIN, TIER1).unwrap();
        let b2 = bucket(APP_CHAIN, TIER2).unwrap();
        assert_eq!(b1.remaining_bytes, TIER1_BYTES);
        assert_eq!(b1.expires_at, t0 + MONTH_SECS);
        assert_eq!(b2.remaining_bytes, TIER2_BYTES);
        assert_eq!(b2.expires_at, t0 + QUARTER_SECS);
        assert_eq!(Bandwidth::remaining(&APP_CHAIN, &APP.0), TIER1_BYTES + TIER2_BYTES);
    });
}

/// FIFO-by-expiry: the bucket about to expire drains first, even when
/// it was bought after the longer-lived bucket.
#[test]
fn gate_consumes_earliest_expiry_bucket_first() {
    new_test_ext().execute_with(|| {
        let t0 = 1_000_000_u64;
        jump_to(t0);
        register_market(APP_CHAIN);
        // Buy the longer-lived tier first — proving the gate orders
        // by `expires_at`, not by purchase order or tier index.
        configure_tier(TIER1, TIER1_BYTES, QUARTER_SECS); // expires later
        configure_tier(TIER2, TIER2_BYTES, MONTH_SECS); // expires sooner
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER2, APP_CHAIN))
            .unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        // Spend less than tier 2's balance — only tier 2 should move.
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 200),
            Ok(())
        );
        assert_eq!(bucket(APP_CHAIN, TIER1).unwrap().remaining_bytes, TIER1_BYTES);
        assert_eq!(bucket(APP_CHAIN, TIER2).unwrap().remaining_bytes, TIER2_BYTES - 200);
    });
}

/// Spend that exceeds the earliest-expiry bucket spills into the next.
/// The drained row is removed from storage.
#[test]
fn gate_spills_into_next_bucket_when_first_drained() {
    new_test_ext().execute_with(|| {
        let t0 = 1_000_000_u64;
        jump_to(t0);
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES, QUARTER_SECS); // 1000, expires later
        configure_tier(TIER2, 300, MONTH_SECS); // 300, expires sooner
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER2, APP_CHAIN))
            .unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        // 500 = drain all 300 of tier 2, then 200 from tier 1.
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 500),
            Ok(())
        );
        assert!(bucket(APP_CHAIN, TIER2).is_none(), "drained bucket gets removed");
        assert_eq!(bucket(APP_CHAIN, TIER1).unwrap().remaining_bytes, TIER1_BYTES - 200);
    });
}

/// Expired buckets don't count toward the gate's available total and
/// are swept from storage on the first gate call that touches them.
#[test]
fn expired_buckets_are_skipped_and_swept() {
    new_test_ext().execute_with(|| {
        let t0 = 1_000_000_u64;
        jump_to(t0);
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        // Fast-forward past expiry. Bucket exists in storage but is dead.
        jump_to(t0 + MONTH_SECS + 1);
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 1),
            Err(GateError::NoAllowance),
            "all expired ⇒ NoAllowance, not Insufficient",
        );
        assert!(bucket(APP_CHAIN, TIER1).is_none(), "expired bucket swept by the gate");
    });
}

/// Buying after expiry resets the bucket cleanly — no phantom carryover.
#[test]
fn purchase_after_expiry_resets_bucket() {
    new_test_ext().execute_with(|| {
        let t0 = 1_000_000_u64;
        jump_to(t0);
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();

        let later = t0 + MONTH_SECS + 100;
        jump_to(later);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();

        let state = bucket(APP_CHAIN, TIER1).unwrap();
        assert_eq!(state.remaining_bytes, TIER1_BYTES, "expired bucket resets, no stacking");
        assert_eq!(state.expires_at, later + MONTH_SECS);
    });
}

#[test]
fn unauthorised_market_rejected() {
    new_test_ext().execute_with(|| {
        jump_to(1_000);
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

        let imposter = H160([0xCC; 20]);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, imposter, TIER1, APP_CHAIN))
            .expect_err("imposter must be rejected");
        assert!(bucket(APP_CHAIN, TIER1).is_none());
    });
}

#[test]
fn unknown_payer_chain_rejected() {
    new_test_ext().execute_with(|| {
        jump_to(1_000);
        configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
        Bandwidth::default()
            .on_accept(purchase_request(PAYER_CHAIN, MARKET, TIER1, APP_CHAIN))
            .expect_err("missing market registration must reject");
    });
}

#[test]
fn unknown_tier_rejected() {
    new_test_ext().execute_with(|| {
        jump_to(1_000);
        register_market(APP_CHAIN);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, 99, APP_CHAIN))
            .expect_err("purchases against unconfigured tiers must reject");
    });
}

#[test]
fn gate_disabled_short_circuits() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 9_999),
            Ok(())
        );
    });
}

#[test]
fn gate_enforce_no_allowance_rejects() {
    new_test_ext().execute_with(|| {
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 100),
            Err(GateError::NoAllowance)
        );
    });
}

#[test]
fn gate_enforce_insufficient_across_buckets_does_not_deduct() {
    new_test_ext().execute_with(|| {
        let t0 = 1_000_000_u64;
        jump_to(t0);
        register_market(APP_CHAIN);
        configure_tier(TIER1, 100, MONTH_SECS);
        configure_tier(TIER2, 50, QUARTER_SECS);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER2, APP_CHAIN))
            .unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        // Total across buckets is 150; ask for 200.
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 200),
            Err(GateError::Insufficient { remaining: 150, required: 200 })
        );
        assert_eq!(bucket(APP_CHAIN, TIER1).unwrap().remaining_bytes, 100);
        assert_eq!(bucket(APP_CHAIN, TIER2).unwrap().remaining_bytes, 50);
    });
}

/// Critical: Observe mode must surface what would happen without
/// affecting state, so flipping to Enforce later is non-destructive.
#[test]
fn gate_observe_does_not_mutate_on_shortfall() {
    new_test_ext().execute_with(|| {
        let t0 = 1_000_000_u64;
        jump_to(t0);
        register_market(APP_CHAIN);
        configure_tier(TIER1, 100, MONTH_SECS);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Observe).unwrap();

        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 200),
            Ok(())
        );
        assert_eq!(bucket(APP_CHAIN, TIER1).unwrap().remaining_bytes, 100);
    });
}

#[test]
fn allowlist_bypasses_gate() {
    new_test_ext().execute_with(|| {
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();
        Bandwidth::set_allowlist(RuntimeOrigin::root(), APP_CHAIN, app_key(), true).unwrap();

        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 99_999),
            Ok(())
        );
    });
}

#[test]
fn is_purchase_message_recognises_authorised_sender() {
    new_test_ext().execute_with(|| {
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
        assert!(Bandwidth::is_purchase_message(&purchase_request(
            APP_CHAIN, MARKET, TIER1, APP_CHAIN
        )));
        assert!(!Bandwidth::is_purchase_message(&purchase_request(
            APP_CHAIN,
            H160([0xCC; 20]),
            TIER1,
            APP_CHAIN
        )));
    });
}

#[test]
fn force_credit_creates_bucket_with_expiry() {
    new_test_ext().execute_with(|| {
        let t0 = 1_000_u64;
        jump_to(t0);
        Bandwidth::force_credit(
            RuntimeOrigin::root(),
            APP_CHAIN,
            app_key(),
            TIER1,
            7_777,
            MONTH_SECS,
        )
        .unwrap();

        let state = bucket(APP_CHAIN, TIER1).unwrap();
        assert_eq!(state.remaining_bytes, 7_777);
        assert_eq!(state.expires_at, t0 + MONTH_SECS);
        // No market registration required for force_credit.
        assert!(BandwidthMarkets::<Test>::get(APP_CHAIN).is_none());
    });
}

#[test]
fn set_tier_round_trips_and_revokes() {
    new_test_ext().execute_with(|| {
        Bandwidth::set_tier(RuntimeOrigin::root(), 7, 4_096, MONTH_SECS).unwrap();
        assert_eq!(Tiers::<Test>::get(7), Some(TierConfig { bytes: 4_096, duration_secs: MONTH_SECS }));

        // Setting bytes to zero must remove — `purchase` then rejects with UnknownTier.
        Bandwidth::set_tier(RuntimeOrigin::root(), 7, 0, 0).unwrap();
        assert!(Tiers::<Test>::get(7).is_none());
    });
}

/// Non-zero bytes with zero duration is nonsensical (bucket would
/// expire on creation) — reject at config time.
#[test]
fn set_tier_rejects_zero_duration_with_nonzero_bytes() {
    new_test_ext().execute_with(|| {
        Bandwidth::set_tier(RuntimeOrigin::root(), 7, 4_096, 0)
            .expect_err("tier with bytes but no duration must be rejected");
    });
}

#[test]
fn mode_storage_round_trips() {
    new_test_ext().execute_with(|| {
        assert_eq!(Mode::<Test>::get(), EnforcementMode::Disabled);
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Observe).unwrap();
        assert_eq!(Mode::<Test>::get(), EnforcementMode::Observe);
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();
        assert_eq!(Mode::<Test>::get(), EnforcementMode::Enforce);
    });
}
