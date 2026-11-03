//! Hyperbridge's own pallets: relayer and collator rewards, the treasury, relayer fees,
//! bandwidth subscriptions and the bridge token supply. Ported from SubQuery's
//! `handlers/events/{incentives,treasury,fees,relayer,collators,bandwidth,supply}`.
//!
//! SubQuery registered these only on `POLKADOT-3367` and `KUSAMA-4009`. A pallet source
//! resolves on every Substrate chain, so each handler checks the chain first.

pub mod bandwidth;
pub mod relayer;
pub mod rewards;
pub mod supply;

use std::sync::Arc;

use alloy::primitives::{keccak256, B256};
use blake2::{Blake2b512, Digest};
use chrono::{NaiveDateTime, NaiveTime};

use crate::{
	handlers::Handlers,
	registry::{LogMeta, Source, Subscription},
	store::timestamp_to_date,
};

pub fn register(handlers: &mut Handlers) {
	let subscriptions = [
		Subscription::new(Source::Pallet("consensusIncentives"), rewards::RelayerRewarded),
		Subscription::new(Source::Pallet("balances"), rewards::TreasuryTransfer),
		Subscription::new(Source::Pallet("collatorManager"), rewards::CollatorRewarded),
		Subscription::new(Source::Pallet("relayer"), relayer::AccumulateFees),
		Subscription::new(Source::Pallet("relayer"), relayer::Withdraw),
		Subscription::new(Source::Pallet("bandwidth"), bandwidth::BandwidthCredited),
		Subscription::new(Source::Pallet("bandwidth"), bandwidth::ForceCredited),
		Subscription::new(Source::Pallet("bandwidth"), bandwidth::BandwidthConsumed),
		Subscription::new(Source::Pallet("bandwidth"), bandwidth::SubscriptionEvicted),
		Subscription::new(Source::Pallet("bandwidth"), bandwidth::TierSet),
	];
	handlers.subscriptions.extend(subscriptions);
	handlers.hooks.push(Arc::new(supply::BridgeTokenSupply));
}

/** `isHyperbridgeChain` in SubQuery's chain-yaml generator. */
fn is_hyperbridge(state_machine_id: &str) -> bool {
	matches!(state_machine_id, "POLKADOT-3367" | "KUSAMA-4009")
}

/** `Timestamp.Now` in milliseconds. Hyperbridge's slot-aligned timestamps are whole seconds. */
fn timestamp_ms(log: &LogMeta) -> u64 {
	log.block_timestamp * 1000
}

/** UTC midnight of a millisecond timestamp, and its `YYYY-MM-DD` id. */
fn utc_day(ms: u64) -> (String, NaiveDateTime) {
	let midnight = timestamp_to_date(ms).date().and_time(NaiveTime::MIN);
	(midnight.format("%Y-%m-%d").to_string(), midnight)
}

/**
 * The log's extrinsic as SubQuery saw it: `(index, hash)`, or `None` for events from block
 * initialization or finalization, which substrate-eth-rpc gives synthetic hashes.
 */
fn extrinsic(log: &LogMeta) -> Option<(u64, B256)> {
	let synthetic = |tag: &[u8]| keccak256([log.block_hash.as_slice(), tag].concat());
	let hash = log.transaction_hash;
	(hash != synthetic(b"init") && hash != synthetic(b"final"))
		.then_some((log.transaction_index, hash))
}

/**
 * `encodeAddress` from `@polkadot/util-crypto`: SS58 for keys of 1, 2, 4, 8, 32 or 33 bytes.
 * Any other length threw in TypeScript, and is `None` here.
 */
fn encode_address(key: &[u8], prefix: u8) -> Option<String> {
	if ![1, 2, 4, 8, 32, 33].contains(&key.len()) || prefix >= 64 {
		return None;
	}
	let mut payload = Vec::with_capacity(key.len() + 3);
	payload.push(prefix);
	payload.extend_from_slice(key);
	let checksum = Blake2b512::new().chain_update(b"SS58PRE").chain_update(&payload).finalize();
	let checksum_len = if key.len() >= 32 { 2 } else { 1 };
	payload.extend_from_slice(&checksum[..checksum_len]);
	Some(bs58::encode(payload).into_string())
}

/** A little-endian `u128` at `offset`, or `None` if the bytes are too short. */
fn read_u128(bytes: &[u8], offset: usize) -> Option<u128> {
	let slice = bytes.get(offset..offset + 16)?;
	Some(u128::from_le_bytes(slice.try_into().ok()?))
}

/** `AccountInfo.data.free`: after four `u32` counters. */
fn account_free(account_info: &[u8]) -> Option<u128> {
	read_u128(account_info, 16)
}

#[cfg(test)]
mod tests {
	use alloy::primitives::Address;

	use super::*;
	use crate::substrate::ss58;

	const ALICE: &str = "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";

	#[test]
	fn encode_address_matches_ss58_for_32_byte_keys() {
		let alice: [u8; 32] = hex::decode(ALICE).unwrap().try_into().unwrap();
		for prefix in [0, 42] {
			assert_eq!(encode_address(&alice, prefix).unwrap(), ss58(&alice, prefix));
		}
	}

	#[test]
	fn encode_address_rejects_evm_addresses() {
		assert_eq!(encode_address(&[0u8; 20], 42), None);
		assert_eq!(encode_address(&[], 42), None);
		// Reference values from @polkadot/util-crypto's `encodeAddress`.
		assert_eq!(encode_address(&[1], 42).unwrap(), "F7NZ");
		assert_eq!(encode_address(&[1, 2], 42).unwrap(), "25GpW4");
		assert_eq!(encode_address(&[1, 2, 3, 4], 42).unwrap(), "MvAtmUea");
		assert_eq!(encode_address(&[1, 2, 3, 4, 5, 6, 7, 8], 0).unwrap(), "1kA3B2yGe32v");
		assert_eq!(
			encode_address(&[1u8; 33], 42).unwrap(),
			"KVwAJq4MvNXEo5cvx4H8GDKgSeDbidnN8UfrxTCJp6ng8ifY1"
		);
	}

	#[test]
	fn the_treasury_account_is_13uv() {
		let treasury: [u8; 32] =
			hex::decode("6d6f646c68622f74727372790000000000000000000000000000000000000000")
				.unwrap()
				.try_into()
				.unwrap();
		assert_eq!(ss58(&treasury, 0), "13UVJyLkyUpEiXBx5p776dHQoBuuk3Y5PYp5Aa89rYWePWA3");
	}

	#[test]
	fn day_buckets_are_utc() {
		let (id, midnight) = utc_day(1_793_711_196_000);
		assert_eq!(id, "2026-11-03");
		assert_eq!(midnight.to_string(), "2026-11-03 00:00:00");
	}

	#[test]
	fn synthetic_hashes_have_no_extrinsic() {
		let block_hash = B256::repeat_byte(7);
		let mut log = LogMeta {
			address: Address::ZERO,
			block_number: 1,
			block_hash,
			block_timestamp: 6,
			transaction_hash: keccak256([block_hash.as_slice(), b"init"].concat()),
			transaction_index: 3,
			log_index: 0,
		};
		assert_eq!(extrinsic(&log), None);
		log.transaction_hash = keccak256([block_hash.as_slice(), b"final"].concat());
		assert_eq!(extrinsic(&log), None);
		log.transaction_hash = B256::repeat_byte(1);
		assert_eq!(extrinsic(&log), Some((3, B256::repeat_byte(1))));
	}
}
