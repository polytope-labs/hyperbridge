//! Relayer and per-chain message statistics: `relayer_v2s`, `relayer_stats_per_chain_v2s`,
//! `relayer_activities` and `hyper_bridge_stats`.

use tokio_postgres::Transaction;

use crate::store::{numeric, Numeric};

/** `RelayerService.findOrCreate`. */
pub async fn ensure_relayer(db: &Transaction<'_>, relayer: &str) -> anyhow::Result<()> {
	db.execute("INSERT INTO app.relayer_v2s (id) VALUES ($1) ON CONFLICT DO NOTHING", &[&relayer])
		.await?;
	Ok(())
}

/** What to add to a relayer's stats on one chain. Every field defaults to 0. */
#[derive(Debug, Clone)]
pub struct RelayerStatsDelta {
	pub messages_delivered: Numeric,
	pub gas_used: Numeric,
	pub gas_fee: Numeric,
	pub usd_gas_fee: Numeric,
	pub fees_earned: Numeric,
	pub withdrawn: Numeric,
}

impl Default for RelayerStatsDelta {
	fn default() -> Self {
		let zero = numeric(0);
		Self {
			messages_delivered: zero.clone(),
			gas_used: zero.clone(),
			gas_fee: zero.clone(),
			usd_gas_fee: zero.clone(),
			fees_earned: zero.clone(),
			withdrawn: zero,
		}
	}
}

/**
 * Adds to `relayer_stats_per_chain_v2s["<relayer>-<chain>"]`, creating the relayer and the row
 * at zero first, as `RelayerStatsPerChainV2Service.findOrCreate` did.
 */
pub async fn add_relayer_stats(
	db: &Transaction<'_>,
	relayer: &str,
	chain: &str,
	delta: &RelayerStatsDelta,
) -> anyhow::Result<()> {
	ensure_relayer(db, relayer).await?;
	db.execute(
		"INSERT INTO app.relayer_stats_per_chain_v2s
		   (id, relayer_id, chain, number_of_successful_messages_delivered, gas_used_for_successful_messages,
		    gas_fee_for_successful_messages, usd_gas_fee_for_successful_messages, fees_earned,
		    cumulative_withdrawn_amount)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
		 ON CONFLICT (id) DO UPDATE SET
		   number_of_successful_messages_delivered = relayer_stats_per_chain_v2s.number_of_successful_messages_delivered + EXCLUDED.number_of_successful_messages_delivered,
		   gas_used_for_successful_messages = relayer_stats_per_chain_v2s.gas_used_for_successful_messages + EXCLUDED.gas_used_for_successful_messages,
		   gas_fee_for_successful_messages = relayer_stats_per_chain_v2s.gas_fee_for_successful_messages + EXCLUDED.gas_fee_for_successful_messages,
		   usd_gas_fee_for_successful_messages = relayer_stats_per_chain_v2s.usd_gas_fee_for_successful_messages + EXCLUDED.usd_gas_fee_for_successful_messages,
		   fees_earned = relayer_stats_per_chain_v2s.fees_earned + EXCLUDED.fees_earned,
		   cumulative_withdrawn_amount = COALESCE(relayer_stats_per_chain_v2s.cumulative_withdrawn_amount, 0) + EXCLUDED.cumulative_withdrawn_amount",
		&[
			&format!("{relayer}-{chain}"),
			&relayer,
			&chain,
			&delta.messages_delivered,
			&delta.gas_used,
			&delta.gas_fee,
			&delta.usd_gas_fee,
			&delta.fees_earned,
			&delta.withdrawn,
		],
	)
	.await?;
	Ok(())
}

/** `updateRelayerActivity`: the relayer's last activity, as the raw timestamp. */
pub async fn touch_relayer_activity(
	db: &Transaction<'_>,
	relayer: &str,
	timestamp: u64,
) -> anyhow::Result<()> {
	ensure_relayer(db, relayer).await?;
	db.execute(
		"INSERT INTO app.relayer_activities (id, relayer_id, last_updated_at) VALUES ($1, $1, $2)
		 ON CONFLICT (id) DO UPDATE SET last_updated_at = EXCLUDED.last_updated_at",
		&[&relayer, &numeric(timestamp)],
	)
	.await?;
	Ok(())
}

/** What to add to `hyper_bridge_stats[chain]`. Every field defaults to 0. */
#[derive(Debug, Clone)]
pub struct ChainStatsDelta {
	pub messages_sent: Numeric,
	pub delivered_messages: Numeric,
	pub timed_out_messages: Numeric,
	pub transfers_in: Numeric,
	pub fees_paid_to_relayers: Numeric,
}

impl Default for ChainStatsDelta {
	fn default() -> Self {
		let zero = numeric(0);
		Self {
			messages_sent: zero.clone(),
			delivered_messages: zero.clone(),
			timed_out_messages: zero.clone(),
			transfers_in: zero.clone(),
			fees_paid_to_relayers: zero,
		}
	}
}

/**
 * Adds to a chain's `hyper_bridge_stats` row, creating it at zero first.
 * `number_of_failed_deliveries` was never incremented, and stays 0.
 */
pub async fn add_chain_stats(
	db: &Transaction<'_>,
	chain: &str,
	delta: &ChainStatsDelta,
) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO app.hyper_bridge_stats
		   (id, number_of_messages_sent, number_of_delivered_messages, number_of_failed_deliveries,
		    number_of_timed_out_messages, total_transfers_in, fees_payed_out_to_relayers)
		 VALUES ($1, $2, $3, 0, $4, $5, $6)
		 ON CONFLICT (id) DO UPDATE SET
		   number_of_messages_sent = hyper_bridge_stats.number_of_messages_sent + EXCLUDED.number_of_messages_sent,
		   number_of_delivered_messages = hyper_bridge_stats.number_of_delivered_messages + EXCLUDED.number_of_delivered_messages,
		   number_of_timed_out_messages = hyper_bridge_stats.number_of_timed_out_messages + EXCLUDED.number_of_timed_out_messages,
		   total_transfers_in = hyper_bridge_stats.total_transfers_in + EXCLUDED.total_transfers_in,
		   fees_payed_out_to_relayers = hyper_bridge_stats.fees_payed_out_to_relayers + EXCLUDED.fees_payed_out_to_relayers",
		&[
			&chain,
			&delta.messages_sent,
			&delta.delivered_messages,
			&delta.timed_out_messages,
			&delta.transfers_in,
			&delta.fees_paid_to_relayers,
		],
	)
	.await?;
	Ok(())
}
