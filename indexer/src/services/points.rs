//! `PointsService`: reward points per earner, and a log row per awarding transaction.

use tokio_postgres::Transaction;

use crate::store::{numeric, timestamp_to_date, PointsActivityType, ProtocolParticipantType};

pub struct Award<'a> {
	/** The earner as each caller formats it: lowercase for users, checksummed for fillers. */
	pub address: &'a str,
	pub chain: &'a str,
	pub points: u64,
	pub earner: ProtocolParticipantType,
	pub activity: PointsActivityType,
	pub transaction_hash: &'a str,
	pub description: &'a str,
	/** Seconds or milliseconds, as the caller's TypeScript passed it. */
	pub timestamp: u64,
}

/**
 * `awardPoints`: adds to `rewards` and writes the activity log. The log id has no event index,
 * so two awards to one earner in one transaction keep only the second log row, as before.
 */
pub async fn award_points(db: &Transaction<'_>, award: Award<'_>) -> anyhow::Result<()> {
	let earner = award.earner.as_str();
	let points = numeric(award.points);
	db.execute(
		"INSERT INTO app.rewards (id, address, chain, points, earner_type) VALUES ($1, $2, $3, $4, $5)
		 ON CONFLICT (id) DO UPDATE SET points = rewards.points + EXCLUDED.points",
		&[
			&format!("{}-{}-{earner}", award.address, award.chain),
			&award.address,
			&award.chain,
			&points,
			&award.earner,
		],
	)
	.await?;
	db.execute(
		"INSERT INTO app.rewards_activity_logs
		   (id, chain, points, transaction_hash, earner_address, earner_type, activity_type, description, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
		 ON CONFLICT (id) DO UPDATE SET chain = EXCLUDED.chain, points = EXCLUDED.points,
		   transaction_hash = EXCLUDED.transaction_hash, earner_address = EXCLUDED.earner_address,
		   earner_type = EXCLUDED.earner_type, activity_type = EXCLUDED.activity_type,
		   description = EXCLUDED.description, created_at = EXCLUDED.created_at",
		&[
			&format!("{}-{earner}-{}", award.address, award.transaction_hash),
			&award.chain,
			&points,
			&award.transaction_hash,
			&award.address,
			&award.earner,
			&award.activity,
			&award.description,
			&timestamp_to_date(award.timestamp),
		],
	)
	.await?;
	Ok(())
}
