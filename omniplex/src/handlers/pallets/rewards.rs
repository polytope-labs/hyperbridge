//! Relayer rewards, collator rewards and treasury transfers.

use alloy::primitives::B256;
use async_trait::async_trait;
use tokio_postgres::Transaction;

use super::{account_free, extrinsic, is_hyperbridge, timestamp_ms, utc_day};
use crate::{
	handlers::abi::pallets::{Balances, CollatorManager, ConsensusIncentives},
	registry::{EventContext, EventHandler},
	store::{numeric, timestamp_to_date, RelayerRewardType},
	substrate::ss58,
};

/** SS58 prefix of AccountId32 fields, as polkadot.js printed them on Nexus and Gargantua. */
pub const ACCOUNT_PREFIX: u8 = 0;

/** `"modl" ++ "hb/trsry"`, the treasury's PalletId account. */
pub const TREASURY: [u8; 32] = *b"modlhb/trsry\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

/** `System.Account(TREASURY)`. */
const TREASURY_ACCOUNT_KEY: &str = "26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9cd6b28636b7a34035711b0be6e52614c6d6f646c68622f74727372790000000000000000000000000000000000000000";

/** `handleRelayerRewardedEvent`. */
pub struct RelayerRewarded;

#[async_trait]
impl EventHandler for RelayerRewarded {
	type Event = ConsensusIncentives::RelayerRewarded;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !is_hyperbridge(ctx.chain.state_machine_id()) {
			return Ok(());
		}
		let reward = RelayerReward {
			relayer: ss58(&event.relayer.0, ACCOUNT_PREFIX),
			amount: event.amount,
			chain: ctx.chain.state_machine_id(),
			block_number: ctx.log.block_number,
			timestamp_ms: timestamp_ms(ctx.log),
			extrinsic: extrinsic(ctx.log),
		};
		record_relayer_reward(ctx.db, &reward).await
	}
}

/** A `RelayerRewarded` event, decoded. */
pub struct RelayerReward<'a> {
	/** SS58, prefix 0. */
	pub relayer: String,
	pub amount: u128,
	/** Hyperbridge's own state machine id. */
	pub chain: &'a str,
	pub block_number: u64,
	pub timestamp_ms: u64,
	/** `(index, hash)`, or `None` outside an extrinsic. */
	pub extrinsic: Option<(u64, B256)>,
}

/**
 * Adds a consensus reward to the relayer's totals and the day's, and writes the reward
 * transaction. SubQuery gave every reward to one relayer in one extrinsic the same id, so only
 * the last survived. The last still takes that id, and each earlier one moves to `<id>-<n>`, in
 * the order they came.
 */
pub async fn record_relayer_reward(
	db: &Transaction<'_>,
	reward: &RelayerReward<'_>,
) -> anyhow::Result<()> {
	let amount = numeric(reward.amount);
	db.execute(
		"INSERT INTO app.hyperbridge_relayer_rewards AS t
		   (id, total_reward_amount, total_consensus_reward_amount)
		 VALUES ($1, $2, $2)
		 ON CONFLICT (id) DO UPDATE SET
		   total_consensus_reward_amount = COALESCE(t.total_consensus_reward_amount, 0) + EXCLUDED.total_consensus_reward_amount,
		   total_reward_amount = COALESCE(t.total_reward_amount, 0) + EXCLUDED.total_reward_amount",
		&[&reward.relayer, &amount],
	)
	.await?;

	let (day, midnight) = utc_day(reward.timestamp_ms);
	db.execute(
		"INSERT INTO app.daily_treasury_relayer_rewards AS t (id, date, daily_reward_amount)
		 VALUES ($1, $2, $3)
		 ON CONFLICT (id) DO UPDATE SET daily_reward_amount = t.daily_reward_amount + EXCLUDED.daily_reward_amount",
		&[&day, &midnight, &amount],
	)
	.await?;

	let index = reward.extrinsic.map_or(0, |(index, _)| index);
	let hash = reward.extrinsic.map(|(_, hash)| hash.to_string());
	let id = format!("{}-{}-{index}", reward.relayer, reward.block_number);
	// The rows sharing the id's prefix are all of this block, whose index keeps the count short.
	db.execute(
		"UPDATE app.relayer_reward_transactions SET id = $1 || '-' || (1 + (
		   SELECT count(*) FROM app.relayer_reward_transactions
		   WHERE block_number = $2 AND starts_with(id, $1 || '-')))
		 WHERE id = $1",
		&[&id, &numeric(reward.block_number)],
	)
	.await?;
	db.execute(
		"INSERT INTO app.relayer_reward_transactions
		   (id, relayer, chain, amount, reward_type, block_number, block_timestamp, extrinsic_hash, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
		 ON CONFLICT (id) DO UPDATE SET
		   relayer = EXCLUDED.relayer, chain = EXCLUDED.chain, amount = EXCLUDED.amount,
		   reward_type = EXCLUDED.reward_type, block_number = EXCLUDED.block_number,
		   block_timestamp = EXCLUDED.block_timestamp, extrinsic_hash = EXCLUDED.extrinsic_hash,
		   created_at = EXCLUDED.created_at",
		&[
			&id,
			&reward.relayer,
			&reward.chain,
			&amount,
			&RelayerRewardType::ConsensusReward,
			&numeric(reward.block_number),
			&numeric(reward.timestamp_ms),
			&hash,
			&timestamp_to_date(reward.timestamp_ms),
		],
	)
	.await?;
	Ok(())
}

/** `handleCollatorRewardedEvent`. */
pub struct CollatorRewarded;

#[async_trait]
impl EventHandler for CollatorRewarded {
	type Event = CollatorManager::CollatorRewarded;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !is_hyperbridge(ctx.chain.state_machine_id()) {
			return Ok(());
		}
		let collator = ss58(&event.collator.0, ACCOUNT_PREFIX);
		record_collator_reward(
			ctx.db,
			&collator,
			event.amount,
			ctx.log.block_number,
			timestamp_ms(ctx.log),
		)
		.await
	}
}

/** Adds a block reward to the collator's totals and the day's, and writes the transaction. */
pub async fn record_collator_reward(
	db: &Transaction<'_>,
	collator: &str,
	amount: u128,
	block_number: u64,
	timestamp_ms: u64,
) -> anyhow::Result<()> {
	let amount = numeric(amount);
	let date = timestamp_to_date(timestamp_ms);
	db.execute(
		"INSERT INTO app.hyperbridge_collator_rewards AS t
		   (id, total_reward_amount, total_blocks_authored, last_rewarded_at)
		 VALUES ($1, $2, 1, $3)
		 ON CONFLICT (id) DO UPDATE SET
		   total_reward_amount = COALESCE(t.total_reward_amount, 0) + EXCLUDED.total_reward_amount,
		   total_blocks_authored = COALESCE(t.total_blocks_authored, 0) + 1,
		   last_rewarded_at = EXCLUDED.last_rewarded_at",
		&[&collator, &amount, &date],
	)
	.await?;

	let (day, midnight) = utc_day(timestamp_ms);
	db.execute(
		"INSERT INTO app.daily_treasury_collator_rewards AS t (id, date, daily_reward_amount)
		 VALUES ($1, $2, $3)
		 ON CONFLICT (id) DO UPDATE SET daily_reward_amount = t.daily_reward_amount + EXCLUDED.daily_reward_amount",
		&[&day, &midnight, &amount],
	)
	.await?;

	db.execute(
		"INSERT INTO app.hyperbridge_collator_reward_transactions
		   (id, collator, amount, block_number, block_timestamp, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6)
		 ON CONFLICT (id) DO UPDATE SET
		   collator = EXCLUDED.collator, amount = EXCLUDED.amount, block_number = EXCLUDED.block_number,
		   block_timestamp = EXCLUDED.block_timestamp, created_at = EXCLUDED.created_at",
		&[
			&format!("{collator}-{block_number}"),
			&collator,
			&amount,
			&numeric(block_number),
			&numeric(timestamp_ms),
			&date,
		],
	)
	.await?;
	Ok(())
}

/** `handleTreasuryTransferEvent`: transfers into or out of the treasury. */
pub struct TreasuryTransfer;

#[async_trait]
impl EventHandler for TreasuryTransfer {
	type Event = Balances::Transfer;

	async fn handle(&self, ctx: &EventContext<'_>, event: Self::Event) -> anyhow::Result<()> {
		if !is_hyperbridge(ctx.chain.state_machine_id()) {
			return Ok(());
		}
		// A transfer from the treasury to itself counts as outgoing only.
		let outgoing = event.from.0 == TREASURY;
		if !outgoing && event.to.0 != TREASURY {
			return Ok(());
		}
		// SubQuery read the balance at the chain tip; this reads it at the transfer's block.
		let key = hex::decode(TREASURY_ACCOUNT_KEY)?;
		let account = ctx.chain.storage(&key, ctx.log.block_number).await?;
		let balance = account_free(&account).unwrap_or(0);
		record_treasury_transfer(ctx.db, outgoing, event.amount, balance, timestamp_ms(ctx.log))
			.await
	}
}

/** Adds a transfer to the treasury's totals and overwrites its balance. */
pub async fn record_treasury_transfer(
	db: &Transaction<'_>,
	outgoing: bool,
	amount: u128,
	balance: u128,
	timestamp_ms: u64,
) -> anyhow::Result<()> {
	let (transferred_in, transferred_out) = if outgoing { (0, amount) } else { (amount, 0) };
	db.execute(
		"INSERT INTO app.treasuries AS t
		   (id, total_amount_transferred_in, total_amount_transferred_out, total_balance, last_updated_at)
		 VALUES ($1, $2, $3, $4, $5)
		 ON CONFLICT (id) DO UPDATE SET
		   total_amount_transferred_in = t.total_amount_transferred_in + EXCLUDED.total_amount_transferred_in,
		   total_amount_transferred_out = t.total_amount_transferred_out + EXCLUDED.total_amount_transferred_out,
		   total_balance = EXCLUDED.total_balance,
		   last_updated_at = EXCLUDED.last_updated_at",
		&[
			&ss58(&TREASURY, ACCOUNT_PREFIX),
			&numeric(transferred_in),
			&numeric(transferred_out),
			&numeric(balance),
			&timestamp_to_date(timestamp_ms),
		],
	)
	.await?;
	Ok(())
}
