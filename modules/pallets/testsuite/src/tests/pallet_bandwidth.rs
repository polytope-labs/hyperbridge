// Copyright (c) 2025 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

//! Integration tests for `pallet-bandwidth` against the testsuite mock
//! runtime.

use polkadot_sdk::*;

use alloy_sol_types::SolValue;
use ismp::{host::StateMachine, module::IsmpModule, router::PostRequest};
use sp_core::{H160, U256};

use pallet_bandwidth::{
	abi::{PurchaseMessage, TierAbi, WithdrawalAbi},
	pallet::{Allowance, BandwidthManager, Tiers, PALLET_BANDWIDTH},
	AppKey, BandwidthGate, ForceCreditParams, GateError, Subscription, TierConfig, TierIndex,
	MAX_SUBSCRIPTIONS,
};

use crate::runtime::{new_test_ext, set_timestamp, Bandwidth, RuntimeOrigin, Test};

/// `APP_CHAIN` is where the BaseIG-style app being funded lives.
/// `PAYER_CHAIN` is the chain whose `BandwidthManager` dispatches the
/// purchase. They differ in the cross-chain-sponsorship case.
const APP_CHAIN: StateMachine = StateMachine::Evm(8453); // Base
const PAYER_CHAIN: StateMachine = StateMachine::Evm(137); // Polygon

const MANAGER: H160 = H160([0xAA; 20]);
const APP: H160 = H160([0xBB; 20]);
const IMPOSTER: H160 = H160([0xCC; 20]);

const TIER1: TierIndex = TierIndex::TierOne;
const TIER2: TierIndex = TierIndex::TierTwo;
const TIER1_BYTES: u128 = 1_000;
const TIER2_BYTES: u128 = 5_000;

/// Mock 28d window; chosen large enough that `t0 + DURATION` fits
/// comfortably in the test harness's `u64` clock.
const MONTH_SECS: u64 = 28 * 24 * 60 * 60;
const QUARTER_SECS: u64 = 3 * MONTH_SECS;

/// Default test epoch — non-zero so `expires_at` is distinguishable
/// from the unset/zero default in storage.
const T0: u64 = 1_000_000;

// ---------- harness helpers ----------

fn app_key() -> AppKey {
	AppKey::truncate_from(APP.0.to_vec())
}

/// `set_timestamp` takes milliseconds; tests think in seconds.
fn jump_to(secs: u64) {
	set_timestamp::<Test>(secs.saturating_mul(1_000));
}

fn register_manager(chain: StateMachine) {
	Bandwidth::set_manager(RuntimeOrigin::root(), chain, MANAGER).unwrap();
}

fn configure_tier(tier: TierIndex, bytes: u128, duration_secs: u64) {
	Bandwidth::set_tier(RuntimeOrigin::root(), tier, Some(TierConfig { bytes, duration_secs }))
		.unwrap();
}

fn consume(bytes: u32) -> Result<(), GateError> {
	<Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, bytes)
}

/// Look up a subscription at FIFO position `idx` (0 = oldest).
fn sub_at(chain: StateMachine, idx: usize) -> Option<Subscription> {
	Allowance::<Test>::get(chain, app_key()).get(idx).cloned()
}

/// Length of the subscription list for `(chain, APP)`.
fn sub_count(chain: StateMachine) -> usize {
	Allowance::<Test>::get(chain, app_key()).len()
}

// ---------- request builders ----------

fn purchase_request(
	payer_chain: StateMachine,
	sender: H160,
	tier: TierIndex,
	app_chain: StateMachine,
) -> PostRequest {
	purchase_request_with_months(payer_chain, sender, tier, 1, app_chain)
}

fn purchase_request_with_months(
	payer_chain: StateMachine,
	sender: H160,
	tier: TierIndex,
	months: u32,
	app_chain: StateMachine,
) -> PostRequest {
	purchase_request_raw(payer_chain, sender, tier.into(), months, app_chain)
}

/// Lets unknown-discriminant cases construct a `PurchaseMessage` with
/// a `tier: u32` that no `TierIndex` variant matches.
fn purchase_request_raw(
	payer_chain: StateMachine,
	sender: H160,
	tier_raw: u32,
	months: u32,
	app_chain: StateMachine,
) -> PostRequest {
	let body: Vec<u8> =
		(&PurchaseMessage { app: APP.0.to_vec(), tier: tier_raw, months, chain: app_chain }).into();

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

/// Drives `IsmpModule::on_accept` with a built request. Discards the
/// returned `Weight` so callers can chain `.unwrap()` / `.expect_err`.
fn dispatch(req: PostRequest) -> Result<(), anyhow::Error> {
	Bandwidth::default().on_accept(req).map(|_| ())
}

/// One-line happy-path purchase: APP_CHAIN as both payer and credit chain.
fn buy(tier: TierIndex) -> Result<(), anyhow::Error> {
	dispatch(purchase_request(APP_CHAIN, MANAGER, tier, APP_CHAIN))
}

fn buy_months(tier: TierIndex, months: u32) -> Result<(), anyhow::Error> {
	dispatch(purchase_request_with_months(APP_CHAIN, MANAGER, tier, months, APP_CHAIN))
}

// ---------- tests ----------

#[test]
fn purchase_creates_subscription_with_fixed_expiry() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		buy(TIER1).unwrap();

		let sub = sub_at(APP_CHAIN, 0).expect("subscription must exist");
		assert_eq!(sub.tier, TIER1);
		assert_eq!(sub.remaining_bytes, TIER1_BYTES);
		assert_eq!(sub.expires_at, T0 + MONTH_SECS);
		assert_eq!(sub.purchased_at, T0);
		assert_eq!(sub_count(APP_CHAIN), 1);
	});
}

/// Buy from Polygon for an app on Base — the credit lands on Base.
/// This is the case the per-`request.source` design got wrong.
#[test]
fn cross_chain_purchase_credits_app_chain_not_payer_chain() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(PAYER_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		dispatch(purchase_request(PAYER_CHAIN, MANAGER, TIER1, APP_CHAIN)).unwrap();

		let sub = sub_at(APP_CHAIN, 0).expect("subscription must exist on app chain");
		assert_eq!(sub.remaining_bytes, TIER1_BYTES);
		assert_eq!(sub_count(PAYER_CHAIN), 0, "payer chain has no credit");

		assert_eq!(consume(100), Ok(()));
	});
}

/// Same tier bought twice 5 days apart: each becomes its own
/// subscription with its own fixed expiry — no stacking, no rollover.
#[test]
fn same_tier_repurchase_creates_independent_subscriptions() {
	new_test_ext().execute_with(|| {
		let five_days = 5 * 24 * 60 * 60;
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		buy(TIER1).unwrap();
		jump_to(T0 + five_days);
		buy(TIER1).unwrap();

		assert_eq!(sub_count(APP_CHAIN), 2, "each purchase is a separate sub");

		let first = sub_at(APP_CHAIN, 0).unwrap();
		let second = sub_at(APP_CHAIN, 1).unwrap();
		assert_eq!(first.remaining_bytes, TIER1_BYTES);
		assert_eq!(first.expires_at, T0 + MONTH_SECS, "first expiry never extends");
		assert_eq!(second.remaining_bytes, TIER1_BYTES, "no stacking");
		assert_eq!(
			second.expires_at,
			T0 + five_days + MONTH_SECS,
			"second expiry is fixed at purchase time + duration",
		);
	});
}

/// Different tiers each appear as their own subscription, in
/// insertion order.
#[test]
fn different_tier_purchases_create_separate_subscriptions() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
		configure_tier(TIER2, TIER2_BYTES, QUARTER_SECS);

		buy(TIER1).unwrap();
		buy(TIER2).unwrap();

		let s1 = sub_at(APP_CHAIN, 0).unwrap();
		let s2 = sub_at(APP_CHAIN, 1).unwrap();
		assert_eq!(s1.tier, TIER1);
		assert_eq!(s1.remaining_bytes, TIER1_BYTES);
		assert_eq!(s1.expires_at, T0 + MONTH_SECS);
		assert_eq!(s2.tier, TIER2);
		assert_eq!(s2.remaining_bytes, TIER2_BYTES);
		assert_eq!(s2.expires_at, T0 + QUARTER_SECS);
		assert_eq!(Bandwidth::remaining(&APP_CHAIN, &APP.0), TIER1_BYTES + TIER2_BYTES);
	});
}

/// FIFO by insertion: the first sub bought drains first, even when a
/// later sub will expire sooner. Diagnostic shape — TIER1 has the
/// longer duration but is bought first; an expiry-FIFO gate would
/// drain TIER2 instead.
#[test]
fn gate_consumes_oldest_subscription_first() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, QUARTER_SECS);
		configure_tier(TIER2, TIER2_BYTES, MONTH_SECS);
		buy(TIER1).unwrap();
		buy(TIER2).unwrap();

		assert_eq!(consume(200), Ok(()));
		assert_eq!(
			sub_at(APP_CHAIN, 0).unwrap().remaining_bytes,
			TIER1_BYTES - 200,
			"oldest sub drains first",
		);
		assert_eq!(
			sub_at(APP_CHAIN, 1).unwrap().remaining_bytes,
			TIER2_BYTES,
			"younger sub is untouched",
		);
	});
}

#[test]
fn gate_spills_into_next_subscription_when_first_drained() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, 300, MONTH_SECS);
		configure_tier(TIER2, TIER2_BYTES, QUARTER_SECS);
		buy(TIER1).unwrap();
		buy(TIER2).unwrap();

		// 500 = drain all 300 of sub#0 (TIER1), then 200 from sub#1 (TIER2).
		assert_eq!(consume(500), Ok(()));
		assert_eq!(sub_count(APP_CHAIN), 1, "fully-drained sub gets removed");
		assert_eq!(sub_at(APP_CHAIN, 0).unwrap().tier, TIER2);
		assert_eq!(sub_at(APP_CHAIN, 0).unwrap().remaining_bytes, TIER2_BYTES - 200);
	});
}

#[test]
fn expired_subscriptions_are_swept() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
		buy(TIER1).unwrap();

		jump_to(T0 + MONTH_SECS + 1);
		assert_eq!(consume(1), Err(GateError::NoAllowance));
		assert_eq!(sub_count(APP_CHAIN), 0, "expired sub swept by the gate");
	});
}

/// After expiry, a new purchase pushes a fresh subscription — the
/// expired one is gone, no stacking with a phantom previous state.
#[test]
fn purchase_after_expiry_pushes_new_subscription() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
		buy(TIER1).unwrap();

		let later = T0 + MONTH_SECS + 100;
		jump_to(later);
		buy(TIER1).unwrap();

		// Old one expired (still in the list — sweep happens on gate
		// activity, not purchase), new one appended at the back.
		let new_sub = sub_at(APP_CHAIN, sub_count(APP_CHAIN) - 1).unwrap();
		assert_eq!(new_sub.remaining_bytes, TIER1_BYTES);
		assert_eq!(new_sub.expires_at, later + MONTH_SECS);
		assert_eq!(new_sub.purchased_at, later);
	});
}

/// Critical ISP-style property: a sub whose drain never starts (because
/// older subs are still being consumed) still expires on its own clock.
/// What you don't reach in time, you lose.
#[test]
fn unused_subscription_expires_independently() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		// sub#0 is fat + long: drain stays inside it the whole test.
		configure_tier(TIER1, 1_000_000, QUARTER_SECS);
		// sub#1 is thin + short: nominally available, but never reached.
		configure_tier(TIER2, 500, MONTH_SECS);
		buy(TIER1).unwrap();
		buy(TIER2).unwrap();

		// Light usage stays inside sub#0.
		assert_eq!(consume(100), Ok(()));
		assert_eq!(sub_count(APP_CHAIN), 2);

		// Cross sub#1's expiry; first gate hit after that prunes it.
		jump_to(T0 + MONTH_SECS + 1);
		assert_eq!(consume(100), Ok(()));
		assert_eq!(sub_count(APP_CHAIN), 1, "unused sub#1 expired and was swept");
		assert_eq!(sub_at(APP_CHAIN, 0).unwrap().tier, TIER1);
	});
}

#[test]
fn unauthorised_market_rejected() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		dispatch(purchase_request(APP_CHAIN, IMPOSTER, TIER1, APP_CHAIN))
			.expect_err("imposter must be rejected");
		assert_eq!(sub_count(APP_CHAIN), 0);
	});
}

#[test]
fn unknown_payer_chain_rejected() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
		dispatch(purchase_request(PAYER_CHAIN, MANAGER, TIER1, APP_CHAIN))
			.expect_err("missing market registration must reject");
	});
}

/// 99 doesn't map to any `TierIndex` variant.
#[test]
fn unknown_tier_discriminant_rejected() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		dispatch(purchase_request_raw(APP_CHAIN, MANAGER, 99, 1, APP_CHAIN))
			.expect_err("unknown tier discriminant must reject");
	});
}

/// Discriminant valid but no `TierConfig` set.
#[test]
fn unconfigured_tier_rejected() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		buy(TIER1).expect_err("purchases against unconfigured tiers must reject");
	});
}

#[test]
fn gate_no_allowance_rejects() {
	new_test_ext().execute_with(|| {
		assert_eq!(consume(100), Err(GateError::NoAllowance));
	});
}

#[test]
fn gate_insufficient_across_subscriptions_does_not_deduct() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, 100, MONTH_SECS);
		configure_tier(TIER2, 50, QUARTER_SECS);
		buy(TIER1).unwrap();
		buy(TIER2).unwrap();

		assert_eq!(consume(200), Err(GateError::Insufficient { remaining: 150, required: 200 }),);
		assert_eq!(sub_at(APP_CHAIN, 0).unwrap().remaining_bytes, 100);
		assert_eq!(sub_at(APP_CHAIN, 1).unwrap().remaining_bytes, 50);
	});
}

#[test]
fn allowlist_bypasses_gate() {
	new_test_ext().execute_with(|| {
		Bandwidth::set_allowlist(RuntimeOrigin::root(), APP_CHAIN, app_key(), true).unwrap();

		assert_eq!(consume(99_999), Ok(()));
	});
}

#[test]
fn is_purchase_message_recognises_authorised_sender() {
	new_test_ext().execute_with(|| {
		register_manager(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
		assert!(Bandwidth::is_purchase_message(&purchase_request(
			APP_CHAIN, MANAGER, TIER1, APP_CHAIN,
		)));
		assert!(!Bandwidth::is_purchase_message(&purchase_request(
			APP_CHAIN, IMPOSTER, TIER1, APP_CHAIN,
		)));
	});
}

#[test]
fn force_credit_pushes_subscription() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		Bandwidth::force_credit(
			RuntimeOrigin::root(),
			ForceCreditParams {
				app_chain: APP_CHAIN,
				app: app_key(),
				tier: TIER1,
				bytes: 7_777,
				duration_secs: MONTH_SECS,
			},
		)
		.unwrap();

		let sub = sub_at(APP_CHAIN, 0).unwrap();
		assert_eq!(sub.tier, TIER1);
		assert_eq!(sub.remaining_bytes, 7_777);
		assert_eq!(sub.expires_at, T0 + MONTH_SECS);
		assert!(BandwidthManager::<Test>::get(APP_CHAIN).is_none());
	});
}

#[test]
fn set_tier_round_trips_and_revokes() {
	new_test_ext().execute_with(|| {
		let cfg = TierConfig { bytes: 4_096, duration_secs: MONTH_SECS };
		Bandwidth::set_tier(RuntimeOrigin::root(), TIER2, Some(cfg)).unwrap();
		assert_eq!(Tiers::<Test>::get(TIER2), Some(cfg));

		Bandwidth::set_tier(RuntimeOrigin::root(), TIER2, None).unwrap();
		assert!(Tiers::<Test>::get(TIER2).is_none());
	});
}

/// Either knob being zero produces an unusable tier — reject early.
#[test]
fn set_tier_rejects_invalid_config() {
	new_test_ext().execute_with(|| {
		Bandwidth::set_tier(
			RuntimeOrigin::root(),
			TIER1,
			Some(TierConfig { bytes: 4_096, duration_secs: 0 }),
		)
		.expect_err("zero duration must be rejected");

		Bandwidth::set_tier(
			RuntimeOrigin::root(),
			TIER1,
			Some(TierConfig { bytes: 0, duration_secs: MONTH_SECS }),
		)
		.expect_err("zero bytes via Some must be rejected (use None to revoke)");
	});
}

/// `months > 1` produces a single scaled subscription, not N
/// back-to-back monthly ones — by design (matches "one purchase = one
/// subscription").
#[test]
fn bulk_purchase_creates_one_scaled_subscription() {
	new_test_ext().execute_with(|| {
		let months = 6_u32;
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		buy_months(TIER1, months).unwrap();

		assert_eq!(sub_count(APP_CHAIN), 1);
		let sub = sub_at(APP_CHAIN, 0).unwrap();
		assert_eq!(sub.remaining_bytes, TIER1_BYTES * months as u128);
		assert_eq!(sub.expires_at, T0 + MONTH_SECS * months as u64);
	});
}

#[test]
fn purchase_rejects_zero_months() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_manager(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		dispatch(purchase_request_raw(APP_CHAIN, MANAGER, TIER1.into(), 0, APP_CHAIN))
			.expect_err("zero months must be rejected at decode time");
		assert_eq!(sub_count(APP_CHAIN), 0);
	});
}

/// The 1024-sub cap evicts the oldest entry. force_credit reuses the
/// same push path as purchase, so this also covers the purchase cap.
#[test]
fn subscription_cap_evicts_oldest() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		let cap = MAX_SUBSCRIPTIONS as u128;

		// Fill the list to exactly the cap. `bytes` encodes the index
		// so we can prove which one got evicted.
		for i in 0..cap {
			Bandwidth::force_credit(
				RuntimeOrigin::root(),
				ForceCreditParams {
					app_chain: APP_CHAIN,
					app: app_key(),
					tier: TIER1,
					bytes: i + 1,
					duration_secs: MONTH_SECS,
				},
			)
			.unwrap();
		}
		assert_eq!(sub_count(APP_CHAIN), cap as usize);
		assert_eq!(sub_at(APP_CHAIN, 0).unwrap().remaining_bytes, 1, "oldest is index 1");

		// One more push: evicts the oldest, appends the new one.
		Bandwidth::force_credit(
			RuntimeOrigin::root(),
			ForceCreditParams {
				app_chain: APP_CHAIN,
				app: app_key(),
				tier: TIER1,
				bytes: cap + 1,
				duration_secs: MONTH_SECS,
			},
		)
		.unwrap();

		assert_eq!(sub_count(APP_CHAIN), cap as usize, "still capped");
		assert_eq!(
			sub_at(APP_CHAIN, 0).unwrap().remaining_bytes,
			2,
			"former second-oldest is now front",
		);
		assert_eq!(
			sub_at(APP_CHAIN, (cap - 1) as usize).unwrap().remaining_bytes,
			cap + 1,
			"new sub is at the back",
		);
	});
}

// ---------- outbound governance dispatch ----------

const ACTION_SET_TIERS: u8 = 0;
const ACTION_WITHDRAW: u8 = 1;

/// Decode a `dispatch_set_tiers` body back into `(tiers, prices)` so
/// we can assert the wire matches what `BandwidthManager.onAccept`
/// would parse.
fn decode_set_tiers(body: &[u8]) -> (Vec<u128>, Vec<u128>) {
	assert_eq!(body[0], ACTION_SET_TIERS, "first byte must be SetTiers discriminant");
	let rows: Vec<TierAbi> = <Vec<TierAbi>>::abi_decode(&body[1..]).unwrap();
	(
		rows.iter().map(|r| r.tier.try_into().unwrap()).collect(),
		rows.iter().map(|r| r.price.try_into().unwrap()).collect(),
	)
}

fn decode_withdraw(body: &[u8]) -> (H160, H160, u128) {
	assert_eq!(body[0], ACTION_WITHDRAW);
	let w = WithdrawalAbi::abi_decode(&body[1..]).unwrap();
	(
		H160::from_slice(w.token.as_slice()),
		H160::from_slice(w.beneficiary.as_slice()),
		w.amount.try_into().unwrap(),
	)
}

#[test]
fn dispatch_set_tiers_rejects_unknown_market() {
	new_test_ext().execute_with(|| {
		Bandwidth::dispatch_set_tiers(
			RuntimeOrigin::root(),
			APP_CHAIN,
			vec![(TIER1, U256::from(1_000_000_000_000_000_000u128))],
		)
		.expect_err("no market registered for APP_CHAIN");
	});
}

#[test]
fn dispatch_set_tiers_rejects_empty_batch() {
	new_test_ext().execute_with(|| {
		register_manager(APP_CHAIN);
		Bandwidth::dispatch_set_tiers(RuntimeOrigin::root(), APP_CHAIN, vec![])
			.expect_err("empty batch must be rejected");
	});
}

#[test]
fn dispatch_set_tiers_admin_only() {
	new_test_ext().execute_with(|| {
		register_manager(APP_CHAIN);
		let alice: sp_core::crypto::AccountId32 = sp_core::crypto::AccountId32::new([1u8; 32]);
		Bandwidth::dispatch_set_tiers(
			RuntimeOrigin::signed(alice),
			APP_CHAIN,
			vec![(TIER1, U256::from(5u128))],
		)
		.expect_err("non-admin must be rejected");
	});
}

#[test]
fn dispatch_withdraw_rejects_unknown_market() {
	new_test_ext().execute_with(|| {
		let token = H160([0x10; 20]);
		let beneficiary = H160([0x20; 20]);
		Bandwidth::dispatch_withdraw(
			RuntimeOrigin::root(),
			APP_CHAIN,
			token,
			beneficiary,
			U256::from(42u128),
		)
		.expect_err("no market registered");
	});
}

#[test]
fn set_tiers_body_round_trips() {
	// Pure body-encoding test, doesn't dispatch — proves the wire
	// format we send matches the contract's `abi.decode((Tier[]))` shape.
	let rows: Vec<TierAbi> = vec![(1u128, 5u128), (2u128, 50u128), (3u128, 500u128)]
		.into_iter()
		.map(|(t, p)| TierAbi {
			tier: alloy_primitives::U256::from(t),
			price: alloy_primitives::U256::from(p),
		})
		.collect();
	let body = rows.abi_encode();

	let (decoded_tiers, decoded_prices) =
		decode_set_tiers(&[&[ACTION_SET_TIERS][..], &body[..]].concat());
	assert_eq!(decoded_tiers, vec![1, 2, 3]);
	assert_eq!(decoded_prices, vec![5, 50, 500]);
}

#[test]
fn withdraw_body_round_trips() {
	let body = WithdrawalAbi {
		token: alloy_primitives::Address::from([0x10u8; 20]),
		beneficiary: alloy_primitives::Address::from([0x20u8; 20]),
		amount: alloy_primitives::U256::from(42u128),
	}
	.abi_encode();
	let (token, beneficiary, amount) =
		decode_withdraw(&[&[ACTION_WITHDRAW][..], &body[..]].concat());
	assert_eq!(token, H160([0x10; 20]));
	assert_eq!(beneficiary, H160([0x20; 20]));
	assert_eq!(amount, 42);
}
