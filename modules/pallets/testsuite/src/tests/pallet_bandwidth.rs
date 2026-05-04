// Copyright (c) 2025 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

//! Integration tests for `pallet-bandwidth` against the testsuite mock
//! runtime — same harness as `pallet_hyperbridge.rs`.

use polkadot_sdk::*;

use ismp::{host::StateMachine, module::IsmpModule, router::PostRequest};
use sp_core::H160;

use pallet_bandwidth::{
    abi::{encode_purchase_msg, PurchaseMessage},
    pallet::{Allowance, BandwidthMarkets, Mode, PALLET_BANDWIDTH_ID},
    AppKey, BandwidthGate, EnforcementMode, GateError,
};

use crate::runtime::{new_test_ext, Bandwidth, RuntimeOrigin, Test};

const SOURCE: StateMachine = StateMachine::Evm(1);
const MARKET: H160 = H160([0xAA; 20]);
const APP: H160 = H160([0xBB; 20]);

fn app_key() -> AppKey {
    AppKey::truncate_from(APP.0.to_vec())
}

/// Mirrors what `EvmHost.dispatch` on the source chain would emit.
fn purchase_request(sender: H160, bytes_purchased: u128, amount_paid_18d: u128) -> PostRequest {
    let body = encode_purchase_msg(&PurchaseMessage {
        app: APP,
        bytes_purchased,
        amount_paid_18d,
    });

    PostRequest {
        source: SOURCE,
        dest: StateMachine::Polkadot(100),
        nonce: 0,
        from: sender.0.to_vec(),
        to: PALLET_BANDWIDTH_ID.to_vec(),
        timeout_timestamp: 0,
        body,
    }
}

fn register_market() {
    Bandwidth::set_market(RuntimeOrigin::root(), SOURCE, MARKET).unwrap();
}

#[test]
fn purchase_credits_allowance() {
    new_test_ext().execute_with(|| {
        register_market();

        Bandwidth::default()
            .on_accept(purchase_request(MARKET, 1_000, 1_000_000))
            .unwrap();

        let state = Allowance::<Test>::get(SOURCE, app_key()).expect("allowance must exist");
        assert_eq!(state.remaining_bytes, 1_000);
        assert_eq!(state.purchased_lifetime, 1_000);
    });
}

#[test]
fn recharge_after_partial_consumption_accumulates() {
    new_test_ext().execute_with(|| {
        register_market();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        Bandwidth::default().on_accept(purchase_request(MARKET, 1_000, 1)).unwrap();
        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&SOURCE, &APP.0, 800), Ok(()));
        Bandwidth::default().on_accept(purchase_request(MARKET, 500, 1)).unwrap();

        let state = Allowance::<Test>::get(SOURCE, app_key()).unwrap();
        assert_eq!(state.remaining_bytes, 700, "200 leftover + 500 recharge");
        assert_eq!(state.purchased_lifetime, 1_500, "purchased lifetime is monotonic");
    });
}

#[test]
fn recharge_after_full_depletion_works() {
    new_test_ext().execute_with(|| {
        register_market();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        Bandwidth::default().on_accept(purchase_request(MARKET, 100, 1)).unwrap();
        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&SOURCE, &APP.0, 100), Ok(()));
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&SOURCE, &APP.0, 1),
            Err(GateError::Insufficient { remaining: 0, required: 1 })
        );

        Bandwidth::default().on_accept(purchase_request(MARKET, 50, 1)).unwrap();
        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&SOURCE, &APP.0, 50), Ok(()));

        let state = Allowance::<Test>::get(SOURCE, app_key()).unwrap();
        assert_eq!(state.remaining_bytes, 0);
        assert_eq!(state.purchased_lifetime, 150);
    });
}

#[test]
fn unauthorised_market_rejected() {
    new_test_ext().execute_with(|| {
        register_market();

        let imposter = H160([0xCC; 20]);
        Bandwidth::default()
            .on_accept(purchase_request(imposter, 1_000, 1))
            .expect_err("imposter must be rejected");
        assert!(Allowance::<Test>::get(SOURCE, app_key()).is_none());
    });
}

#[test]
fn unknown_source_chain_rejected() {
    new_test_ext().execute_with(|| {
        Bandwidth::default()
            .on_accept(purchase_request(MARKET, 1_000, 1))
            .expect_err("missing market registration must reject");
    });
}

#[test]
fn gate_disabled_short_circuits() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&SOURCE, &APP.0, 9999),
            Ok(())
        );
    });
}

#[test]
fn gate_enforce_no_allowance_rejects() {
    new_test_ext().execute_with(|| {
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();
        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&SOURCE, &APP.0, 100),
            Err(GateError::NoAllowance)
        );
    });
}

#[test]
fn gate_enforce_deducts_on_success() {
    new_test_ext().execute_with(|| {
        register_market();
        Bandwidth::default().on_accept(purchase_request(MARKET, 1_000, 1)).unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&SOURCE, &APP.0, 200), Ok(()));

        let state = Allowance::<Test>::get(SOURCE, app_key()).unwrap();
        assert_eq!(state.remaining_bytes, 800);
    });
}

#[test]
fn gate_enforce_insufficient_does_not_deduct() {
    new_test_ext().execute_with(|| {
        register_market();
        Bandwidth::default().on_accept(purchase_request(MARKET, 100, 1)).unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();

        assert_eq!(
            <Bandwidth as BandwidthGate>::try_consume(&SOURCE, &APP.0, 200),
            Err(GateError::Insufficient { remaining: 100, required: 200 })
        );
        assert_eq!(Allowance::<Test>::get(SOURCE, app_key()).unwrap().remaining_bytes, 100);
    });
}

/// Critical: Observe mode must surface what would happen without
/// affecting state, so flipping to Enforce later is non-destructive.
#[test]
fn gate_observe_does_not_mutate_on_shortfall() {
    new_test_ext().execute_with(|| {
        register_market();
        Bandwidth::default().on_accept(purchase_request(MARKET, 100, 1)).unwrap();
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Observe).unwrap();

        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&SOURCE, &APP.0, 200), Ok(()));
        assert_eq!(Allowance::<Test>::get(SOURCE, app_key()).unwrap().remaining_bytes, 100);
    });
}

#[test]
fn allowlist_bypasses_gate() {
    new_test_ext().execute_with(|| {
        Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();
        Bandwidth::set_allowlist(RuntimeOrigin::root(), SOURCE, app_key(), true).unwrap();

        assert_eq!(<Bandwidth as BandwidthGate>::try_consume(&SOURCE, &APP.0, 99_999), Ok(()));
    });
}

#[test]
fn is_purchase_message_recognises_authorised_sender() {
    new_test_ext().execute_with(|| {
        register_market();
        assert!(Bandwidth::is_purchase_message(&purchase_request(MARKET, 1, 1)));
        assert!(!Bandwidth::is_purchase_message(&purchase_request(H160([0xCC; 20]), 1, 1)));
    });
}

#[test]
fn force_credit_creates_allowance() {
    new_test_ext().execute_with(|| {
        Bandwidth::force_credit(RuntimeOrigin::root(), SOURCE, app_key(), 7_777).unwrap();
        assert_eq!(Allowance::<Test>::get(SOURCE, app_key()).unwrap().remaining_bytes, 7_777);
        assert!(BandwidthMarkets::<Test>::get(SOURCE).is_none());
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
