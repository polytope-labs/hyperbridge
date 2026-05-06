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
    AppKey, BandwidthGate, EnforcementMode, GateError, TierIndex,
};

use crate::runtime::{new_test_ext, Bandwidth, RuntimeOrigin, Test};

/// `APP_CHAIN` is where the BaseIG-style app being funded lives.
/// `PAYER_CHAIN` is the chain whose `BandwidthManager` dispatches the
/// purchase. They differ in the cross-chain-sponsorship case.
const APP_CHAIN: StateMachine = StateMachine::Evm(8453); // Base
const PAYER_CHAIN: StateMachine = StateMachine::Evm(137); // Polygon
const MARKET: H160 = H160([0xAA; 20]);
const APP: H160 = H160([0xBB; 20]);
const TIER1: TierIndex = 1;
const TIER1_BYTES: u128 = 1_000;

fn app_key() -> AppKey {
    AppKey::truncate_from(APP.0.to_vec())
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

fn configure_tier(tier: TierIndex, bytes: u128) {
    Bandwidth::set_tier(RuntimeOrigin::root(), tier, bytes).unwrap();
}

#[test]
fn purchase_credits_allowance() {
    new_test_ext().execute_with(|| {
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES);

        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();

        let state = Allowance::<Test>::get(APP_CHAIN, app_key()).expect("allowance must exist");
        assert_eq!(state.remaining_bytes, TIER1_BYTES);
        assert_eq!(state.purchased_lifetime, TIER1_BYTES);
    });
}

/// Buy from Polygon for an app on Base — the credit lands on Base.
/// This is the case the per-`request.source` design got wrong.
#[test]
fn cross_chain_purchase_credits_app_chain_not_payer_chain() {
    new_test_ext().execute_with(|| {
        register_market(PAYER_CHAIN);
        configure_tier(TIER1, TIER1_BYTES);

        Bandwidth::default()
            .on_accept(purchase_request(PAYER_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();

        // Credit lands on the app's home chain ...
        let state = Allowance::<Test>::get(APP_CHAIN, app_key()).expect("allowance must exist");
        assert_eq!(state.remaining_bytes, TIER1_BYTES);

        // ... NOT under the payer's chain.
        assert!(Allowance::<Test>::get(PAYER_CHAIN, app_key()).is_none());

        // And the gate (which keys on `request.source` — the chain the
        // app lives on) finds the balance.
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 100),
            Ok(())
        );
    });
}

#[test]
fn recharge_after_partial_consumption_accumulates() {
    new_test_ext().execute_with(|| {
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES);
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 800), Ok(()));
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();

        let state = Allowance::<Test>::get(APP_CHAIN, app_key()).unwrap();
        assert_eq!(state.remaining_bytes, 1_200, "200 leftover + 1000 recharge");
        assert_eq!(state.purchased_lifetime, 2_000, "purchased lifetime is monotonic");
    });
}

#[test]
fn recharge_after_full_depletion_works() {
    new_test_ext().execute_with(|| {
        register_market(APP_CHAIN);
        configure_tier(TIER1, 100);
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 100), Ok(()));
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 1),
            Err(GateError::Insufficient { remaining: 0, required: 1 })
        );

        configure_tier(TIER1, 50);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 50), Ok(()));

        let state = Allowance::<Test>::get(APP_CHAIN, app_key()).unwrap();
        assert_eq!(state.remaining_bytes, 0);
        assert_eq!(state.purchased_lifetime, 150);
    });
}

#[test]
fn unauthorised_market_rejected() {
    new_test_ext().execute_with(|| {
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES);

        let imposter = H160([0xCC; 20]);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, imposter, TIER1, APP_CHAIN))
            .expect_err("imposter must be rejected");
        assert!(Allowance::<Test>::get(APP_CHAIN, app_key()).is_none());
    });
}

#[test]
fn unknown_payer_chain_rejected() {
    new_test_ext().execute_with(|| {
        configure_tier(TIER1, TIER1_BYTES);
        // No `BandwidthMarkets` entry for PAYER_CHAIN.
        Bandwidth::default()
            .on_accept(purchase_request(PAYER_CHAIN, MARKET, TIER1, APP_CHAIN))
            .expect_err("missing market registration must reject");
    });
}

#[test]
fn unknown_tier_rejected() {
    new_test_ext().execute_with(|| {
        register_market(APP_CHAIN);
        // No tier configured.
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
fn gate_enforce_deducts_on_success() {
    new_test_ext().execute_with(|| {
        register_market(APP_CHAIN);
        configure_tier(TIER1, TIER1_BYTES);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 200), Ok(()));

        let state = Allowance::<Test>::get(APP_CHAIN, app_key()).unwrap();
        assert_eq!(state.remaining_bytes, 800);
    });
}

#[test]
fn gate_enforce_insufficient_does_not_deduct() {
    new_test_ext().execute_with(|| {
        register_market(APP_CHAIN);
        configure_tier(TIER1, 100);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 200),
            Err(GateError::Insufficient { remaining: 100, required: 200 })
        );
        assert_eq!(Allowance::<Test>::get(APP_CHAIN, app_key()).unwrap().remaining_bytes, 100);
    });
}

/// Critical: Observe mode must surface what would happen without
/// affecting state, so flipping to Enforce later is non-destructive.
#[test]
fn gate_observe_does_not_mutate_on_shortfall() {
    new_test_ext().execute_with(|| {
        register_market(APP_CHAIN);
        configure_tier(TIER1, 100);
        Bandwidth::default()
            .on_accept(purchase_request(APP_CHAIN, MARKET, TIER1, APP_CHAIN))
            .unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Observe).unwrap();

        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, 200), Ok(()));
        assert_eq!(Allowance::<Test>::get(APP_CHAIN, app_key()).unwrap().remaining_bytes, 100);
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
        configure_tier(TIER1, TIER1_BYTES);
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
fn force_credit_creates_allowance() {
    new_test_ext().execute_with(|| {
        Bandwidth::force_credit(RuntimeOrigin::root(), APP_CHAIN, app_key(), 7_777).unwrap();
        assert_eq!(
            Allowance::<Test>::get(APP_CHAIN, app_key()).unwrap().remaining_bytes,
            7_777
        );
        assert!(BandwidthMarkets::<Test>::get(APP_CHAIN).is_none());
    });
}

#[test]
fn set_tier_round_trips_and_revokes() {
    new_test_ext().execute_with(|| {
        Bandwidth::set_tier(RuntimeOrigin::root(), 7, 4_096).unwrap();
        assert_eq!(Tiers::<Test>::get(7), Some(4_096));

        // Setting to zero must remove — `purchase` then rejects with UnknownTier.
        Bandwidth::set_tier(RuntimeOrigin::root(), 7, 0).unwrap();
        assert!(Tiers::<Test>::get(7).is_none());
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
