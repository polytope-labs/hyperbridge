//! `VolumeService`: cumulative and daily USD volume, at 18-decimal fixed point.

use chrono::NaiveTime;
use tokio_postgres::Transaction;

use crate::{
	services::decimal::to_scaled_usd,
	store::{numeric, timestamp_to_date},
};

/**
 * `updateVolume`: adds `usd` (a decimal string) to `<base>.<state machine>` and to its row for
 * the UTC day. `timestamp` is stored as given (seconds on EVM, milliseconds on Substrate).
 */
pub async fn update_volume(
	db: &Transaction<'_>,
	base: &str,
	state_machine: &str,
	usd: &str,
	timestamp: u64,
) -> anyhow::Result<()> {
	let id = format!("{base}.{state_machine}");
	update_cumulative_volume(db, &id, usd, timestamp).await?;
	update_daily_volume(db, &id, usd, timestamp).await
}

/**
 * `updateCumulativeVolume`. A second update with the same timestamp, i.e. from the same block,
 * is dropped: the TypeScript compared `lastUpdatedAt` before adding, and its tests pin that.
 */
pub async fn update_cumulative_volume(
	db: &Transaction<'_>,
	id: &str,
	usd: &str,
	timestamp: u64,
) -> anyhow::Result<()> {
	db.execute(
		"INSERT INTO app.cumulative_volume_u_s_ds (id, last_updated_at, volume_u_s_d) VALUES ($1, $2, $3)
		 ON CONFLICT (id) DO UPDATE SET
		   volume_u_s_d = CASE WHEN cumulative_volume_u_s_ds.last_updated_at <> EXCLUDED.last_updated_at
		     THEN cumulative_volume_u_s_ds.volume_u_s_d + EXCLUDED.volume_u_s_d
		     ELSE cumulative_volume_u_s_ds.volume_u_s_d END,
		   last_updated_at = EXCLUDED.last_updated_at",
		&[&id, &numeric(timestamp), &to_scaled_usd(usd)?],
	)
	.await?;
	Ok(())
}

/** `updateDailyVolume`: the row for the UTC day of `timestamp`, always added to. */
pub async fn update_daily_volume(
	db: &Transaction<'_>,
	id: &str,
	usd: &str,
	timestamp: u64,
) -> anyhow::Result<()> {
	let day = timestamp_to_date(timestamp).date().and_time(NaiveTime::MIN);
	db.execute(
		"INSERT INTO app.daily_volume_u_s_ds (id, last_updated_at, created_at, last24_hours_volume_u_s_d)
		 VALUES ($1, $2, $3, $4)
		 ON CONFLICT (id) DO UPDATE SET
		   last24_hours_volume_u_s_d = daily_volume_u_s_ds.last24_hours_volume_u_s_d
		     + EXCLUDED.last24_hours_volume_u_s_d,
		   last_updated_at = EXCLUDED.last_updated_at",
		&[&format!("{id}.{}", day.format("%Y-%m-%d")), &numeric(timestamp), &day, &to_scaled_usd(usd)?],
	)
	.await?;
	Ok(())
}
