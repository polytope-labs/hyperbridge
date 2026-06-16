//! Cross-language ABI parity tests for app-level payloads dispatched
//! between Rust pallets and Solidity contracts:
//!
//! - HFT `Message` (`pallets/hyper-fungible-token` ↔ `apps/HyperFungibleToken.sol`)
//! - `BandwidthPurchaseMsg` (`apps/BandwidthManager.sol` → `pallets/bandwidth`)
//! - `Tier[]` (`pallets/bandwidth` → `apps/BandwidthManager.sol` SetTiers)
//! - `Withdrawal` (`pallets/bandwidth` → `apps/BandwidthManager.sol` Withdraw)
//!
//! Each test mirrors the production encode/decode methods on both
//! sides — the Rust assertions are intentionally what the pallets do
//! today, so any divergence from Solidity's `abi.encode(struct)`
//! surfaces here.

use super::utils::*;
use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{SolCall, SolValue};

alloy_sol_macro::sol! {
	// ── Mirror of production sol! types ─────────────────────────────

	/// Matches `Message` in `sdk/packages/core/contracts/apps/HyperFungibleToken.sol:82-91`
	/// and the alloy declaration in `pallets/hyper-fungible-token/src/types.rs`.
	struct HftMessage {
		bytes from;
		bytes to;
		uint256 amount;
		bytes data;
	}

	/// Matches `BandwidthPurchaseMsg` in `evm/src/apps/BandwidthManager.sol:32-41`.
	struct BandwidthPurchaseMsg {
		bytes app;
		uint256 tier;
		uint256 months;
		bytes chain;
	}

	/// Matches `Tier` in `evm/src/apps/BandwidthManager.sol:44-49`.
	struct Tier {
		uint256 tier;
		uint256 price;
	}

	/// Matches `Withdrawal` in `evm/src/apps/BandwidthManager.sol:54-58`.
	struct Withdrawal {
		address token;
		address beneficiary;
		uint256 amount;
	}

	// ── AbiAppsCodec function selectors ─────────────────────────────

	function encodeHftMessage(HftMessage m) external pure returns (bytes);
	function decodeHftMessage(bytes data) external pure returns (HftMessage);

	function encodeBandwidthPurchase(BandwidthPurchaseMsg m) external pure returns (bytes);
	function decodeBandwidthPurchase(bytes data) external pure returns (BandwidthPurchaseMsg);

	function encodeTiers(Tier[] tiers) external pure returns (bytes);
	function decodeTiers(bytes data) external pure returns (Tier[]);

	function encodeWithdrawal(Withdrawal w) external pure returns (bytes);
	function decodeWithdrawal(bytes data) external pure returns (Withdrawal);
}

fn deploy_codec(env: &mut TestEnv) -> Address {
	let out_dir = env.evm_out_dir_public();
	env.deploy_named(&out_dir, "AbiAppsCodec")
}

fn sample_hft_message() -> HftMessage {
	HftMessage {
		from: Bytes::from(hex::decode("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef").unwrap()),
		to: Bytes::from(hex::decode("cafebabecafebabecafebabecafebabecafebabe").unwrap()),
		amount: U256::from(1_000_000_000_000_000_000u128),
		data: Bytes::from(hex::decode("1234").unwrap()),
	}
}

fn sample_bandwidth_purchase() -> BandwidthPurchaseMsg {
	BandwidthPurchaseMsg {
		app: Bytes::from(b"my-app".to_vec()),
		tier: U256::from(2u64),
		months: U256::from(3u64),
		chain: Bytes::from(b"EVM-8453".to_vec()),
	}
}

fn sample_tiers() -> Vec<Tier> {
	vec![
		Tier { tier: U256::from(0u64), price: U256::from(10_000_000_000_000_000u128) },
		Tier { tier: U256::from(1u64), price: U256::from(50_000_000_000_000_000u128) },
		Tier { tier: U256::from(2u64), price: U256::from(250_000_000_000_000_000u128) },
	]
}

fn sample_withdrawal() -> Withdrawal {
	Withdrawal {
		token: Address::repeat_byte(0xab),
		beneficiary: Address::repeat_byte(0xcd),
		amount: U256::from(123_456_789u64),
	}
}

/// Encoding parity for the HFT `Message`. Mirrors the production path
/// in `pallets/hyper-fungible-token/src/lib.rs:296` and `module.rs:59,215`,
/// which use `abi_encode` / `abi_decode` to match Solidity's
/// `abi.encode(struct)` / `abi.decode(data, (Message))`.
#[test]
fn test_hft_message_encoding_parity() {
	let mut env = TestEnv::new();
	let codec = deploy_codec(&mut env);
	let msg = sample_hft_message();

	// Rust encodes — exactly as `pallets/hyper-fungible-token/src/lib.rs:296` does.
	let rust_encoded = HftMessage::abi_encode(&msg);

	// Solidity encodes — exactly as `HyperFungibleToken.sol:242` does.
	let result = env.call(codec, encodeHftMessageCall { m: msg.clone() }.abi_encode());
	let sol_encoded = Bytes::abi_decode(&result).unwrap().to_vec();

	assert_eq!(rust_encoded, sol_encoded, "HFT Message encoding mismatch");

	// Solidity must be able to decode Rust-produced bytes.
	let _ = env.call(
		codec,
		decodeHftMessageCall { data: Bytes::from(rust_encoded.clone()) }.abi_encode(),
	);

	// Rust must be able to decode Solidity-produced bytes — mirrors
	// `pallets/hyper-fungible-token/src/module.rs:59,215`.
	let decoded = HftMessage::abi_decode(&sol_encoded)
		.expect("Rust `abi_decode` failed on Solidity-encoded HFT Message");
	assert_eq!(decoded.from, msg.from);
	assert_eq!(decoded.to, msg.to);
	assert_eq!(decoded.amount, msg.amount);
	assert_eq!(decoded.data, msg.data);
}

/// Encoding parity for `BandwidthPurchaseMsg`. Solidity encodes via
/// `abi.encode(body)` (`BandwidthManager.sol:177`); Rust decodes via
/// `abi_decode` (`pallets/bandwidth/src/abi.rs:64`). Mirror struct
/// encoded with `abi_encode` to round-trip correctly.
#[test]
fn test_bandwidth_purchase_encoding_parity() {
	let mut env = TestEnv::new();
	let codec = deploy_codec(&mut env);
	let msg = sample_bandwidth_purchase();

	// Rust encodes — exactly as `pallets/bandwidth/src/abi.rs:90` does.
	let rust_encoded = BandwidthPurchaseMsg::abi_encode(&msg);

	// Solidity encodes — exactly as `BandwidthManager.sol:177` does.
	let result =
		env.call(codec, encodeBandwidthPurchaseCall { m: msg.clone() }.abi_encode());
	let sol_encoded = Bytes::abi_decode(&result).unwrap().to_vec();

	assert_eq!(rust_encoded, sol_encoded, "BandwidthPurchaseMsg encoding mismatch");

	// Rust must be able to decode Solidity-produced bytes — mirrors
	// `pallets/bandwidth/src/abi.rs:64`.
	let decoded = BandwidthPurchaseMsg::abi_decode(&sol_encoded)
		.expect("Rust `abi_decode` failed on Solidity-encoded BandwidthPurchaseMsg");
	assert_eq!(decoded.app, msg.app);
	assert_eq!(decoded.tier, msg.tier);
	assert_eq!(decoded.months, msg.months);
	assert_eq!(decoded.chain, msg.chain);
}

/// Encoding parity for the `Tier[]` governance payload. Rust uses
/// `rows.abi_encode_params()` (`pallets/bandwidth/src/lib.rs:317`);
/// Solidity uses `abi.decode(body[1:], (Tier[]))`. For a dynamic
/// array (non-tuple SolType), `abi_encode_params` wraps in a 1-tuple
/// and matches `abi.encode(arr)`, so this should pass.
#[test]
fn test_tiers_encoding_parity() {
	let mut env = TestEnv::new();
	let codec = deploy_codec(&mut env);
	let tiers = sample_tiers();

	// Rust encodes — exactly as `pallets/bandwidth/src/lib.rs:317` does.
	let rust_encoded = tiers.abi_encode_params();

	// Solidity encodes — exactly as `abi.decode(body[1:], (Tier[]))`
	// expects on the receive side.
	let result = env.call(codec, encodeTiersCall { tiers: tiers.clone() }.abi_encode());
	let sol_encoded = Bytes::abi_decode(&result).unwrap().to_vec();

	assert_eq!(rust_encoded, sol_encoded, "Tier[] encoding mismatch");

	// Solidity must be able to decode Rust-produced bytes.
	let result = env.call(
		codec,
		decodeTiersCall { data: Bytes::from(rust_encoded.clone()) }.abi_encode(),
	);
	let decoded = <Vec<Tier> as SolValue>::abi_decode(&result).unwrap();
	assert_eq!(decoded.len(), tiers.len());
	for (a, b) in decoded.iter().zip(tiers.iter()) {
		assert_eq!(a.tier, b.tier);
		assert_eq!(a.price, b.price);
	}
}

/// Encoding parity for the `Withdrawal` governance payload. Rust uses
/// `payload.abi_encode_params()` (`pallets/bandwidth/src/lib.rs:346`);
/// Solidity uses `abi.decode(body[1:], (Withdrawal))`. `Withdrawal`
/// is all-static (`address`, `address`, `uint256`), so tuple-wrap vs
/// fields-spread produce identical bytes — this should pass.
#[test]
fn test_withdrawal_encoding_parity() {
	let mut env = TestEnv::new();
	let codec = deploy_codec(&mut env);
	let w = sample_withdrawal();

	// Rust encodes — exactly as `pallets/bandwidth/src/lib.rs:346` does.
	let rust_encoded = Withdrawal::abi_encode_params(&w);

	// Solidity encodes — exactly as `abi.decode(body[1:], (Withdrawal))`
	// expects on the receive side.
	let result = env.call(codec, encodeWithdrawalCall { w: w.clone() }.abi_encode());
	let sol_encoded = Bytes::abi_decode(&result).unwrap().to_vec();

	assert_eq!(rust_encoded, sol_encoded, "Withdrawal encoding mismatch");

	// Solidity must be able to decode Rust-produced bytes.
	let result = env.call(
		codec,
		decodeWithdrawalCall { data: Bytes::from(rust_encoded) }.abi_encode(),
	);
	let decoded = Withdrawal::abi_decode(&result).unwrap();
	assert_eq!(decoded.token, w.token);
	assert_eq!(decoded.beneficiary, w.beneficiary);
	assert_eq!(decoded.amount, w.amount);
}
