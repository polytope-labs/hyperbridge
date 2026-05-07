// Copyright (c) 2025 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

//! Integration tests for `pallet-bandwidth` against the testsuite mock
//! runtime — same harness as `pallet_hyperbridge.rs`.

use polkadot_sdk::*;

use alloy_sol_types::{sol_data, SolType};
use ismp::{host::StateMachine, module::IsmpModule, router::PostRequest};
use sp_core::{H160, U256};

use pallet_bandwidth::{
	abi::PurchaseMessage,
	pallet::{Allowance, BandwidthMarkets, Mode, Tiers, PALLET_BANDWIDTH},
	AppKey, BandwidthGate, EnforcementMode, ForceCreditParams, GateError, TierConfig, TierIndex,
};

use crate::runtime::{new_test_ext, set_timestamp, Bandwidth, RuntimeOrigin, Test};

/// `APP_CHAIN` is where the BaseIG-style app being funded lives.
/// `PAYER_CHAIN` is the chain whose `BandwidthManager` dispatches the
/// purchase. They differ in the cross-chain-sponsorship case.
const APP_CHAIN: StateMachine = StateMachine::Evm(8453); // Base
const PAYER_CHAIN: StateMachine = StateMachine::Evm(137); // Polygon

const MARKET: H160 = H160([0xAA; 20]);
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

fn register_market(chain: StateMachine) {
	Bandwidth::set_market(RuntimeOrigin::root(), chain, MARKET).unwrap();
}

fn configure_tier(tier: TierIndex, bytes: u128, duration_secs: u64) {
	Bandwidth::set_tier(RuntimeOrigin::root(), tier, Some(TierConfig { bytes, duration_secs }))
		.unwrap();
}

fn enforce() {
	Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Enforce).unwrap();
}

fn observe() {
	Bandwidth::set_enforcement_mode(RuntimeOrigin::root(), EnforcementMode::Observe).unwrap();
}

fn consume(bytes: u32) -> Result<(), GateError> {
	<Bandwidth as BandwidthGate>::try_consume(&APP_CHAIN, &APP.0, bytes)
}

/// Read the per-tier slot out of the per-`(chain, app)` BTreeMap.
fn bucket(chain: StateMachine, tier: TierIndex) -> Option<pallet_bandwidth::AllowanceState> {
	Allowance::<Test>::get(chain, app_key()).get(&tier).cloned()
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
		(&PurchaseMessage { app: APP.0.to_vec(), tier: tier_raw, months, app_chain }).into();

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
	dispatch(purchase_request(APP_CHAIN, MARKET, tier, APP_CHAIN))
}

fn buy_months(tier: TierIndex, months: u32) -> Result<(), anyhow::Error> {
	dispatch(purchase_request_with_months(APP_CHAIN, MARKET, tier, months, APP_CHAIN))
}

// ---------- tests ----------

#[test]
fn purchase_credits_allowance_with_expiry() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		buy(TIER1).unwrap();

		let state = bucket(APP_CHAIN, TIER1).expect("bucket must exist");
		assert_eq!(state.remaining_bytes, TIER1_BYTES);
		assert_eq!(state.expires_at, T0 + MONTH_SECS);
	});
}

/// Buy from Polygon for an app on Base — the credit lands on Base.
/// This is the case the per-`request.source` design got wrong.
#[test]
fn cross_chain_purchase_credits_app_chain_not_payer_chain() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(PAYER_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		dispatch(purchase_request(PAYER_CHAIN, MARKET, TIER1, APP_CHAIN)).unwrap();

		let state = bucket(APP_CHAIN, TIER1).expect("bucket must exist");
		assert_eq!(state.remaining_bytes, TIER1_BYTES);
		assert!(bucket(PAYER_CHAIN, TIER1).is_none());

		enforce();
		assert_eq!(consume(100), Ok(()));
	});
}

/// Same tier bought twice 5 days apart: bytes stack and the second
/// expiry starts where the first ended (David's "rollover"). Buying 6
/// months upfront falls out of this rule.
#[test]
fn same_tier_repurchase_stacks_bytes_and_extends_expiry() {
	new_test_ext().execute_with(|| {
		let five_days = 5 * 24 * 60 * 60;
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		buy(TIER1).unwrap();
		let first_expiry = bucket(APP_CHAIN, TIER1).unwrap().expires_at;
		assert_eq!(first_expiry, T0 + MONTH_SECS);

		jump_to(T0 + five_days);
		buy(TIER1).unwrap();

		let state = bucket(APP_CHAIN, TIER1).unwrap();
		assert_eq!(state.remaining_bytes, 2 * TIER1_BYTES, "bytes stack");
		assert_eq!(
			state.expires_at,
			first_expiry + MONTH_SECS,
			"second window starts when the first ended, not when bought",
		);
	});
}

/// Different tiers live in independent BTreeMap entries with
/// independent expiries.
#[test]
fn different_tier_purchases_create_separate_buckets() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
		configure_tier(TIER2, TIER2_BYTES, QUARTER_SECS);

		buy(TIER1).unwrap();
		buy(TIER2).unwrap();

		let b1 = bucket(APP_CHAIN, TIER1).unwrap();
		let b2 = bucket(APP_CHAIN, TIER2).unwrap();
		assert_eq!(b1.remaining_bytes, TIER1_BYTES);
		assert_eq!(b1.expires_at, T0 + MONTH_SECS);
		assert_eq!(b2.remaining_bytes, TIER2_BYTES);
		assert_eq!(b2.expires_at, T0 + QUARTER_SECS);
		assert_eq!(Bandwidth::remaining(&APP_CHAIN, &APP.0), TIER1_BYTES + TIER2_BYTES);
	});
}

/// FIFO-by-expiry: the bucket about to expire drains first, even when
/// it was bought after the longer-lived bucket.
#[test]
fn gate_consumes_earliest_expiry_bucket_first() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, QUARTER_SECS);
		configure_tier(TIER2, TIER2_BYTES, MONTH_SECS);
		buy(TIER1).unwrap();
		buy(TIER2).unwrap();
		enforce();

		assert_eq!(consume(200), Ok(()));
		assert_eq!(bucket(APP_CHAIN, TIER1).unwrap().remaining_bytes, TIER1_BYTES);
		assert_eq!(bucket(APP_CHAIN, TIER2).unwrap().remaining_bytes, TIER2_BYTES - 200);
	});
}

#[test]
fn gate_spills_into_next_bucket_when_first_drained() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, QUARTER_SECS);
		configure_tier(TIER2, 300, MONTH_SECS);
		buy(TIER1).unwrap();
		buy(TIER2).unwrap();
		enforce();

		// 500 = drain all 300 of tier 2, then 200 from tier 1.
		assert_eq!(consume(500), Ok(()));
		assert!(bucket(APP_CHAIN, TIER2).is_none(), "drained bucket gets removed");
		assert_eq!(bucket(APP_CHAIN, TIER1).unwrap().remaining_bytes, TIER1_BYTES - 200);
	});
}

#[test]
fn expired_buckets_are_skipped_and_swept() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
		buy(TIER1).unwrap();
		enforce();

		jump_to(T0 + MONTH_SECS + 1);
		assert_eq!(consume(1), Err(GateError::NoAllowance));
		assert!(bucket(APP_CHAIN, TIER1).is_none(), "expired bucket swept by the gate");
	});
}

#[test]
fn purchase_after_expiry_resets_bucket() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
		buy(TIER1).unwrap();

		let later = T0 + MONTH_SECS + 100;
		jump_to(later);
		buy(TIER1).unwrap();

		let state = bucket(APP_CHAIN, TIER1).unwrap();
		assert_eq!(state.remaining_bytes, TIER1_BYTES, "expired bucket resets, no stacking");
		assert_eq!(state.expires_at, later + MONTH_SECS);
	});
}

#[test]
fn unauthorised_market_rejected() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		dispatch(purchase_request(APP_CHAIN, IMPOSTER, TIER1, APP_CHAIN))
			.expect_err("imposter must be rejected");
		assert!(bucket(APP_CHAIN, TIER1).is_none());
	});
}

#[test]
fn unknown_payer_chain_rejected() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
		dispatch(purchase_request(PAYER_CHAIN, MARKET, TIER1, APP_CHAIN))
			.expect_err("missing market registration must reject");
	});
}

/// 99 doesn't map to any `TierIndex` variant.
#[test]
fn unknown_tier_discriminant_rejected() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		dispatch(purchase_request_raw(APP_CHAIN, MARKET, 99, 1, APP_CHAIN))
			.expect_err("unknown tier discriminant must reject");
	});
}

/// Discriminant valid but no `TierConfig` set.
#[test]
fn unconfigured_tier_rejected() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		buy(TIER1).expect_err("purchases against unconfigured tiers must reject");
	});
}

#[test]
fn gate_disabled_short_circuits() {
	new_test_ext().execute_with(|| {
		assert_eq!(consume(9_999), Ok(()));
	});
}

#[test]
fn gate_enforce_no_allowance_rejects() {
	new_test_ext().execute_with(|| {
		enforce();
		assert_eq!(consume(100), Err(GateError::NoAllowance));
	});
}

#[test]
fn gate_enforce_insufficient_across_buckets_does_not_deduct() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, 100, MONTH_SECS);
		configure_tier(TIER2, 50, QUARTER_SECS);
		buy(TIER1).unwrap();
		buy(TIER2).unwrap();
		enforce();

		assert_eq!(consume(200), Err(GateError::Insufficient { remaining: 150, required: 200 }),);
		assert_eq!(bucket(APP_CHAIN, TIER1).unwrap().remaining_bytes, 100);
		assert_eq!(bucket(APP_CHAIN, TIER2).unwrap().remaining_bytes, 50);
	});
}

/// Critical: Observe mode must surface what would happen without
/// affecting state, so flipping to Enforce later is non-destructive.
#[test]
fn gate_observe_does_not_mutate_on_shortfall() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, 100, MONTH_SECS);
		buy(TIER1).unwrap();
		observe();

		assert_eq!(consume(200), Ok(()));
		assert_eq!(bucket(APP_CHAIN, TIER1).unwrap().remaining_bytes, 100);
	});
}

#[test]
fn allowlist_bypasses_gate() {
	new_test_ext().execute_with(|| {
		enforce();
		Bandwidth::set_allowlist(RuntimeOrigin::root(), APP_CHAIN, app_key(), true).unwrap();

		assert_eq!(consume(99_999), Ok(()));
	});
}

#[test]
fn is_purchase_message_recognises_authorised_sender() {
	new_test_ext().execute_with(|| {
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);
		assert!(Bandwidth::is_purchase_message(&purchase_request(
			APP_CHAIN, MARKET, TIER1, APP_CHAIN,
		)));
		assert!(!Bandwidth::is_purchase_message(&purchase_request(
			APP_CHAIN, IMPOSTER, TIER1, APP_CHAIN,
		)));
	});
}

#[test]
fn force_credit_creates_bucket_with_expiry() {
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

		let state = bucket(APP_CHAIN, TIER1).unwrap();
		assert_eq!(state.remaining_bytes, 7_777);
		assert_eq!(state.expires_at, T0 + MONTH_SECS);
		assert!(BandwidthMarkets::<Test>::get(APP_CHAIN).is_none());
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

#[test]
fn bulk_purchase_credits_proportional_bytes_and_extends_expiry() {
	new_test_ext().execute_with(|| {
		let months = 6_u32;
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		buy_months(TIER1, months).unwrap();

		let state = bucket(APP_CHAIN, TIER1).unwrap();
		assert_eq!(state.remaining_bytes, TIER1_BYTES * months as u128);
		assert_eq!(state.expires_at, T0 + MONTH_SECS * months as u64);
	});
}

#[test]
fn purchase_rejects_zero_months() {
	new_test_ext().execute_with(|| {
		jump_to(T0);
		register_market(APP_CHAIN);
		configure_tier(TIER1, TIER1_BYTES, MONTH_SECS);

		dispatch(purchase_request_raw(APP_CHAIN, MARKET, TIER1.into(), 0, APP_CHAIN))
			.expect_err("zero months must be rejected at decode time");
		assert!(bucket(APP_CHAIN, TIER1).is_none());
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
	type Abi = (sol_data::Array<sol_data::Uint<256>>, sol_data::Array<sol_data::Uint<256>>);
	let (tiers, prices) = Abi::abi_decode_params(&body[1..]).unwrap();
	(
		tiers.into_iter().map(|t| t.try_into().unwrap()).collect(),
		prices.into_iter().map(|p| p.try_into().unwrap()).collect(),
	)
}

fn decode_withdraw(body: &[u8]) -> (H160, H160, u128) {
	assert_eq!(body[0], ACTION_WITHDRAW);
	type Abi = (sol_data::Address, sol_data::Address, sol_data::Uint<256>);
	let (token, beneficiary, amount) = Abi::abi_decode_params(&body[1..]).unwrap();
	(
		H160::from_slice(token.as_slice()),
		H160::from_slice(beneficiary.as_slice()),
		amount.try_into().unwrap(),
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
		register_market(APP_CHAIN);
		Bandwidth::dispatch_set_tiers(RuntimeOrigin::root(), APP_CHAIN, vec![])
			.expect_err("empty batch must be rejected");
	});
}

#[test]
fn dispatch_set_tiers_admin_only() {
	new_test_ext().execute_with(|| {
		register_market(APP_CHAIN);
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
	// format we send matches the contract's `abi.decode((uint256[],
	// uint256[]))` shape.
	use alloy_sol_types::SolType;
	type Abi = (sol_data::Array<sol_data::Uint<256>>, sol_data::Array<sol_data::Uint<256>>);
	let tiers: Vec<alloy_primitives::U256> = vec![1u128, 2u128, 3u128]
		.into_iter()
		.map(alloy_primitives::U256::from)
		.collect();
	let prices: Vec<alloy_primitives::U256> = vec![5u128, 50u128, 500u128]
		.into_iter()
		.map(alloy_primitives::U256::from)
		.collect();
	let body = Abi::abi_encode_params(&(tiers.clone(), prices.clone()));
	let (tiers_back, prices_back) = Abi::abi_decode_params(&body).unwrap();
	assert_eq!(tiers, tiers_back);
	assert_eq!(prices, prices_back);

	// And our pallet helper builds the same body.
	let (decoded_tiers, decoded_prices) =
		decode_set_tiers(&[&[ACTION_SET_TIERS][..], &body[..]].concat());
	assert_eq!(decoded_tiers, vec![1, 2, 3]);
	assert_eq!(decoded_prices, vec![5, 50, 500]);
}

#[test]
fn withdraw_body_round_trips() {
	let body = {
		type Abi = (sol_data::Address, sol_data::Address, sol_data::Uint<256>);
		Abi::abi_encode_params(&(
			alloy_primitives::Address::from([0x10u8; 20]),
			alloy_primitives::Address::from([0x20u8; 20]),
			alloy_primitives::U256::from(42u128),
		))
	};
	let (token, beneficiary, amount) =
		decode_withdraw(&[&[ACTION_WITHDRAW][..], &body[..]].concat());
	assert_eq!(token, H160([0x10; 20]));
	assert_eq!(beneficiary, H160([0x20; 20]));
	assert_eq!(amount, 42);
}

#[test]
fn mode_storage_round_trips() {
	new_test_ext().execute_with(|| {
		assert_eq!(Mode::<Test>::get(), EnforcementMode::Disabled);
		observe();
		assert_eq!(Mode::<Test>::get(), EnforcementMode::Observe);
		enforce();
		assert_eq!(Mode::<Test>::get(), EnforcementMode::Enforce);
	});
}
