//! `pallet-bandwidth`: per-app subscriptions, consumption and tier configuration.
//! Ported from `services/bandwidth.service.ts`. Every handler but `TierSet` re-reads the
//! app's `Allowance` afterwards, because the pallet drains and expires subscriptions silently.

use std::{collections::BTreeMap, str::FromStr};

use alloy::primitives::B256;
use anyhow::Context;
use async_trait::async_trait;
use codec::{Decode, Encode};
use ismp::host::StateMachine;
use sp_crypto_hashing::{blake2_128, twox_64};
use tokio_postgres::Transaction;

use super::{extrinsic, is_hyperbridge, timestamp_ms, utc_day};
use crate::{
	handlers::abi::pallets::Bandwidth,
	registry::{EventContext, EventHandler},
	store::{numeric, postgres_text},
};

/** `twox128("Bandwidth") ++ twox128("Allowance")`. */
const ALLOWANCE_PREFIX: &str = "862cd327b24d7fa7f8c07091385221e246e04418345ce96b38cb72145af0570e";

fn app_id(chain: &str, app_hex: &str) -> String {
	format!("{chain}-{app_hex}")
}

/** A new subscription, from `BandwidthCredited` or `ForceCredited`. */
pub struct Credit<'a> {
	pub chain: &'a str,
	/** Lowercase hex of the app key, no `0x`. */
	pub app_hex: &'a str,
	pub tier: u8,
	pub bytes: u128,
	pub expires_at_secs: u64,
	pub paid_from: &'a str,
	pub forced: bool,
	pub block_number: u64,
	pub log_index: u64,
	pub timestamp_ms: u64,
	pub extrinsic_hash: Option<B256>,
}

/** `recordCredit`: writes the subscription and bumps the app's and tier's lifetime counters. */
pub async fn record_credit(db: &Transaction<'_>, credit: &Credit<'_>) -> anyhow::Result<()> {
	// Event strings anyone can craft, and Postgres text can't hold a NUL.
	let (chain, paid_from) = (postgres_text(credit.chain), postgres_text(credit.paid_from));
	let (chain, paid_from): (&str, &str) = (&chain, &paid_from);
	let app = app_id(chain, credit.app_hex);
	let bytes = numeric(credit.bytes);
	let ms = numeric(credit.timestamp_ms);
	db.execute(
		"INSERT INTO app.bandwidth_apps AS t
		   (id, chain, app, first_seen_at, last_activity_at, lifetime_subscriptions, lifetime_bytes_credited,
		    lifetime_bytes_consumed, lifetime_bytes_evicted, active_subscriptions)
		 VALUES ($1, $2, $3, $4, $4, 1, $5, 0, 0, 0)
		 ON CONFLICT (id) DO UPDATE SET
		   last_activity_at = EXCLUDED.last_activity_at,
		   lifetime_subscriptions = t.lifetime_subscriptions + 1,
		   lifetime_bytes_credited = t.lifetime_bytes_credited + EXCLUDED.lifetime_bytes_credited",
		&[&app, &chain, &credit.app_hex, &ms, &bytes],
	)
	.await?;
	db.execute(
		"INSERT INTO app.bandwidth_app_tier_stats AS t
		   (id, app_id, tier, lifetime_subscriptions, lifetime_bytes_credited, lifetime_bytes_evicted,
		    active_subscriptions, last_purchased_at)
		 VALUES ($1, $2, $3, 1, $4, 0, 0, $5)
		 ON CONFLICT (id) DO UPDATE SET
		   lifetime_subscriptions = t.lifetime_subscriptions + 1,
		   lifetime_bytes_credited = t.lifetime_bytes_credited + EXCLUDED.lifetime_bytes_credited,
		   last_purchased_at = EXCLUDED.last_purchased_at",
		&[&format!("{app}-{}", credit.tier), &app, &i32::from(credit.tier), &bytes, &ms],
	)
	.await?;
	// Re-processing the same id clears any eviction marks, as SubQuery's create().save() did.
	db.execute(
		"INSERT INTO app.bandwidth_subscriptions
		   (id, app_id, tier, bytes, expires_at, purchased_at, paid_from, forced, evicted_at,
		    evicted_lost_bytes, block_number, extrinsic_hash)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, NULL, $9, $10)
		 ON CONFLICT (id) DO UPDATE SET
		   app_id = EXCLUDED.app_id, tier = EXCLUDED.tier, bytes = EXCLUDED.bytes,
		   expires_at = EXCLUDED.expires_at, purchased_at = EXCLUDED.purchased_at,
		   paid_from = EXCLUDED.paid_from, forced = EXCLUDED.forced, evicted_at = NULL,
		   evicted_lost_bytes = NULL, block_number = EXCLUDED.block_number,
		   extrinsic_hash = EXCLUDED.extrinsic_hash",
		&[
			&format!("{app}-{}-{}", credit.block_number, credit.log_index),
			&app,
			&i32::from(credit.tier),
			&bytes,
			&numeric(credit.expires_at_secs),
			&ms,
			&paid_from,
			&credit.forced,
			&numeric(credit.block_number),
			&credit.extrinsic_hash.map(|hash| hash.to_string()),
		],
	)
	.await?;
	Ok(())
}

/** `recordConsumption`: the app's lifetime bytes consumed and its UTC day's bucket. */
pub async fn record_consumption(
	db: &Transaction<'_>,
	chain: &str,
	app_hex: &str,
	bytes: u128,
	timestamp_ms: u64,
) -> anyhow::Result<()> {
	let chain = postgres_text(chain);
	let chain: &str = &chain;
	let app = app_id(chain, app_hex);
	let bytes = numeric(bytes);
	db.execute(
		"INSERT INTO app.bandwidth_apps AS t
		   (id, chain, app, first_seen_at, last_activity_at, lifetime_subscriptions, lifetime_bytes_credited,
		    lifetime_bytes_consumed, lifetime_bytes_evicted, active_subscriptions)
		 VALUES ($1, $2, $3, $4, $4, 0, 0, $5, 0, 0)
		 ON CONFLICT (id) DO UPDATE SET
		   last_activity_at = EXCLUDED.last_activity_at,
		   lifetime_bytes_consumed = t.lifetime_bytes_consumed + EXCLUDED.lifetime_bytes_consumed",
		&[&app, &chain, &app_hex, &numeric(timestamp_ms), &bytes],
	)
	.await?;
	let (day, midnight) = utc_day(timestamp_ms);
	db.execute(
		"INSERT INTO app.bandwidth_app_daily_consumptions AS t (id, app_id, date, bytes_consumed)
		 VALUES ($1, $2, $3, $4)
		 ON CONFLICT (id) DO UPDATE SET bytes_consumed = t.bytes_consumed + EXCLUDED.bytes_consumed",
		&[&format!("{app}-{day}"), &app, &midnight, &bytes],
	)
	.await?;
	Ok(())
}

/**
 * `recordEviction`: the app's and tier's lifetime bytes evicted, and an eviction mark on the
 * oldest unevicted subscription of the tier. Like SubQuery, it looks only at the first 100 of
 * the app's subscriptions in that tier by id, and takes the lowest `purchased_at` among them.
 */
pub async fn record_eviction(
	db: &Transaction<'_>,
	chain: &str,
	app_hex: &str,
	tier: u8,
	lost_bytes: u128,
	timestamp_ms: u64,
) -> anyhow::Result<()> {
	let chain = postgres_text(chain);
	let chain: &str = &chain;
	let app = app_id(chain, app_hex);
	let lost = numeric(lost_bytes);
	let ms = numeric(timestamp_ms);
	db.execute(
		"INSERT INTO app.bandwidth_apps AS t
		   (id, chain, app, first_seen_at, last_activity_at, lifetime_subscriptions, lifetime_bytes_credited,
		    lifetime_bytes_consumed, lifetime_bytes_evicted, active_subscriptions)
		 VALUES ($1, $2, $3, $4, $4, 0, 0, 0, $5, 0)
		 ON CONFLICT (id) DO UPDATE SET
		   last_activity_at = EXCLUDED.last_activity_at,
		   lifetime_bytes_evicted = t.lifetime_bytes_evicted + EXCLUDED.lifetime_bytes_evicted",
		&[&app, &chain, &app_hex, &ms, &lost],
	)
	.await?;
	db.execute(
		"INSERT INTO app.bandwidth_app_tier_stats AS t
		   (id, app_id, tier, lifetime_subscriptions, lifetime_bytes_credited, lifetime_bytes_evicted,
		    active_subscriptions, last_purchased_at)
		 VALUES ($1, $2, $3, 0, 0, $4, 0, $5)
		 ON CONFLICT (id) DO UPDATE SET
		   lifetime_bytes_evicted = t.lifetime_bytes_evicted + EXCLUDED.lifetime_bytes_evicted",
		&[&format!("{app}-{tier}"), &app, &i32::from(tier), &lost, &ms],
	)
	.await?;
	let marked = db
		.execute(
			"UPDATE app.bandwidth_subscriptions SET evicted_at = $3, evicted_lost_bytes = $4
			 WHERE id = (
			   SELECT id FROM (
			     SELECT id, purchased_at, evicted_at FROM app.bandwidth_subscriptions
			     WHERE app_id = $1 AND tier = $2 ORDER BY id LIMIT 100
			   ) window_rows
			   WHERE evicted_at IS NULL ORDER BY purchased_at, id LIMIT 1
			 )",
			&[&app, &i32::from(tier), &ms, &lost],
		)
		.await?;
	if marked == 0 {
		tracing::warn!(app, tier, "SubscriptionEvicted: no unevicted subscription to mark");
	}
	Ok(())
}

/** `applyTierSet`: `None` deactivates the tier and clears its limits. */
pub async fn apply_tier_set(
	db: &Transaction<'_>,
	tier: u8,
	config: Option<(u128, u64)>,
	timestamp_ms: u64,
) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO app.bandwidth_tiers (id, tier, bytes, duration_secs, is_active, last_updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6)
		 ON CONFLICT (id) DO UPDATE SET
		   bytes = EXCLUDED.bytes, duration_secs = EXCLUDED.duration_secs,
		   is_active = EXCLUDED.is_active, last_updated_at = EXCLUDED.last_updated_at",
		&[
			&tier.to_string(),
			&i32::from(tier),
			&config.map(|(bytes, _)| numeric(bytes)),
			&config.map(|(_, duration)| numeric(duration)),
			&config.is_some(),
			&numeric(timestamp_ms),
		],
	)
	.await?;
	Ok(())
}

/**
 * The tail of `syncActiveCounts`: overwrites the app's and each tier's active subscriptions
 * with the counts in `Allowance`, given as the tier of each stored subscription.
 */
pub async fn set_active_counts(
	db: &Transaction<'_>,
	chain: &str,
	app_hex: &str,
	tiers: &[u8],
	timestamp_ms: u64,
) -> anyhow::Result<()> {
	let chain = postgres_text(chain);
	let chain: &str = &chain;
	let app = app_id(chain, app_hex);
	let ms = numeric(timestamp_ms);
	let total = i32::try_from(tiers.len())?;
	db.execute(
		"INSERT INTO app.bandwidth_apps
		   (id, chain, app, first_seen_at, last_activity_at, lifetime_subscriptions, lifetime_bytes_credited,
		    lifetime_bytes_consumed, lifetime_bytes_evicted, active_subscriptions)
		 VALUES ($1, $2, $3, $4, $4, 0, 0, 0, 0, $5)
		 ON CONFLICT (id) DO UPDATE SET active_subscriptions = EXCLUDED.active_subscriptions",
		&[&app, &chain, &app_hex, &ms, &total],
	)
	.await?;
	db.execute(
		"UPDATE app.bandwidth_app_tier_stats SET active_subscriptions = 0 WHERE app_id = $1",
		&[&app],
	)
	.await?;
	let mut counts = BTreeMap::<u8, i32>::new();
	for tier in tiers {
		*counts.entry(*tier).or_default() += 1;
	}
	for (tier, count) in counts {
		db.execute(
			"INSERT INTO app.bandwidth_app_tier_stats
			   (id, app_id, tier, lifetime_subscriptions, lifetime_bytes_credited, lifetime_bytes_evicted,
			    active_subscriptions, last_purchased_at)
			 VALUES ($1, $2, $3, 0, 0, 0, $4, $5)
			 ON CONFLICT (id) DO UPDATE SET active_subscriptions = EXCLUDED.active_subscriptions",
			&[&format!("{app}-{tier}"), &app, &i32::from(tier), &count, &ms],
		)
		.await?;
	}
	Ok(())
}

/** `Bandwidth.Allowance(app_chain, app)`: `Twox64Concat` then `Blake2_128Concat`. */
pub fn allowance_key(state_machine: &StateMachine, app: &[u8]) -> Vec<u8> {
	let chain = state_machine.encode();
	let app = app.to_vec().encode();
	let mut key = hex::decode(ALLOWANCE_PREFIX).expect("constant is hex");
	key.extend_from_slice(&twox_64(&chain));
	key.extend_from_slice(&chain);
	key.extend_from_slice(&blake2_128(&app));
	key.extend_from_slice(&app);
	key
}

/** A stored `Subscription`: tier, remaining bytes, expiry and purchase time in seconds. */
#[derive(Decode)]
struct Subscription {
	tier: u8,
	_remaining_bytes: u128,
	_expires_at: u64,
	_purchased_at: u64,
}

/**
 * The tier of every subscription in an `Allowance` value, expired ones included: the pallet
 * only sweeps those inside `try_consume`. An absent key is an empty list.
 */
pub fn allowance_tiers(value: &[u8]) -> anyhow::Result<Vec<u8>> {
	if value.is_empty() {
		return Ok(vec![]);
	}
	let list = Vec::<Subscription>::decode(&mut &value[..]).context("decoding Allowance")?;
	Ok(list.into_iter().map(|sub| sub.tier).collect())
}

/** `syncActiveCounts`, reading `Allowance` at the event's block. */
async fn sync_active_counts(
	ctx: &EventContext<'_>,
	chain: &str,
	app: &[u8],
	ms: u64,
) -> anyhow::Result<()> {
	let Ok(state_machine) = StateMachine::from_str(chain) else {
		tracing::warn!(chain, "bandwidth: can't rebuild the state machine; counts left unchanged");
		return Ok(());
	};
	let value = ctx
		.chain
		.storage(&allowance_key(&state_machine, app), ctx.log.block_number)
		.await?;
	// A decode failure repeats on every retry, so it must not fail the batch. SubQuery left the
	// counts unchanged too.
	let tiers = match allowance_tiers(&value) {
		Ok(tiers) => tiers,
		Err(err) => {
			tracing::warn!(chain, "bandwidth: {err:#}; counts left unchanged");
			return Ok(());
		},
	};
	set_active_counts(ctx.db, chain, &hex::encode(app), &tiers, ms).await
}

/** `handleBandwidthCreditedEvent`. */
pub struct BandwidthCredited;

#[async_trait]
impl EventHandler for BandwidthCredited {
	type Event = Bandwidth::BandwidthCredited;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !is_hyperbridge(ctx.chain.state_machine_id()) {
			return Ok(());
		}
		let ms = timestamp_ms(ctx.log);
		let app_hex = hex::encode(&event.app);
		let credit = Credit {
			chain: &event.appChain,
			app_hex: &app_hex,
			tier: event.tier,
			bytes: event.byteCount,
			expires_at_secs: event.expiresAt,
			paid_from: &event.paidFrom,
			forced: false,
			block_number: ctx.log.block_number,
			log_index: ctx.log.log_index,
			timestamp_ms: ms,
			extrinsic_hash: extrinsic(ctx.log).map(|(_, hash)| hash),
		};
		record_credit(ctx.db, &credit).await?;
		sync_active_counts(ctx, &event.appChain, &event.app, ms).await
	}
}

/** `handleForceCreditedEvent`: an admin credit, paid from the app's own chain. */
pub struct ForceCredited;

#[async_trait]
impl EventHandler for ForceCredited {
	type Event = Bandwidth::ForceCredited;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !is_hyperbridge(ctx.chain.state_machine_id()) {
			return Ok(());
		}
		let ms = timestamp_ms(ctx.log);
		let app_hex = hex::encode(&event.app);
		let credit = Credit {
			chain: &event.appChain,
			app_hex: &app_hex,
			tier: event.tier,
			bytes: event.byteCount,
			expires_at_secs: event.expiresAt,
			paid_from: &event.appChain,
			forced: true,
			block_number: ctx.log.block_number,
			log_index: ctx.log.log_index,
			timestamp_ms: ms,
			extrinsic_hash: extrinsic(ctx.log).map(|(_, hash)| hash),
		};
		record_credit(ctx.db, &credit).await?;
		sync_active_counts(ctx, &event.appChain, &event.app, ms).await
	}
}

/** `handleBandwidthConsumedEvent`. The request's source keys the allowance. */
pub struct BandwidthConsumed;

#[async_trait]
impl EventHandler for BandwidthConsumed {
	type Event = Bandwidth::BandwidthConsumed;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !is_hyperbridge(ctx.chain.state_machine_id()) {
			return Ok(());
		}
		let ms = timestamp_ms(ctx.log);
		let app_hex = hex::encode(&event.app);
		record_consumption(ctx.db, &event.source, &app_hex, event.byteCount, ms).await?;
		sync_active_counts(ctx, &event.source, &event.app, ms).await
	}
}

/** `handleSubscriptionEvictedEvent`. */
pub struct SubscriptionEvicted;

#[async_trait]
impl EventHandler for SubscriptionEvicted {
	type Event = Bandwidth::SubscriptionEvicted;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !is_hyperbridge(ctx.chain.state_machine_id()) {
			return Ok(());
		}
		let ms = timestamp_ms(ctx.log);
		let app_hex = hex::encode(&event.app);
		record_eviction(ctx.db, &event.appChain, &app_hex, event.tier, event.lostBytes, ms).await?;
		sync_active_counts(ctx, &event.appChain, &event.app, ms).await
	}
}

/** `handleTierSetEvent`. */
pub struct TierSet;

#[async_trait]
impl EventHandler for TierSet {
	type Event = Bandwidth::TierSet;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !is_hyperbridge(ctx.chain.state_machine_id()) {
			return Ok(());
		}
		let config = event
			.config
			.isSome
			.then_some((event.config.value.byteCount, event.config.value.durationSecs));
		apply_tier_set(ctx.db, event.tier, config, timestamp_ms(ctx.log)).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn allowance_key_follows_the_storage_hashers() {
		let app = hex::decode("5762e54a92f331226dec592a4cebcf35579ffdc2").unwrap();
		let key = allowance_key(&StateMachine::Evm(1), &app);
		let chain = hex::decode("0001000000").unwrap();
		let mut app_encoded = vec![20 << 2];
		app_encoded.extend_from_slice(&app);
		let expected = [
			hex::decode(ALLOWANCE_PREFIX).unwrap(),
			twox_64(&chain).to_vec(),
			chain,
			blake2_128(&app_encoded).to_vec(),
			app_encoded,
		]
		.concat();
		assert_eq!(key, expected);
	}

	#[test]
	fn allowance_counts_every_stored_subscription() {
		assert_eq!(allowance_tiers(&[]).unwrap(), Vec::<u8>::new());
		let mut value = vec![3 << 2];
		for tier in [1u8, 4, 1] {
			value.push(tier);
			value.extend_from_slice(&100u128.to_le_bytes());
			value.extend_from_slice(&1_789_650_030u64.to_le_bytes());
			value.extend_from_slice(&1_787_058_030u64.to_le_bytes());
		}
		assert_eq!(value.len(), 1 + 3 * 33);
		assert_eq!(allowance_tiers(&value).unwrap(), vec![1, 4, 1]);
		assert!(allowance_tiers(&value[..50]).is_err());
	}
}
