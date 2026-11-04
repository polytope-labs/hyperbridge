//! Statuses that arrive before their parent row. Another chain writes the parent, so a status can
//! be indexed first; it waits in `pending_status_metadata` until the parent exists.

use std::{sync::Arc, time::Duration};

use chrono::NaiveDateTime;
use tokio_postgres::Transaction;

use crate::{store::Numeric, Indexer};

/** The parent a pending status belongs to, as `pending_status_metadata.entity_type` names it. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingEntity {
	Request,
	GetRequest,
	Order,
}

impl PendingEntity {
	pub const ALL: [PendingEntity; 3] =
		[PendingEntity::Request, PendingEntity::GetRequest, PendingEntity::Order];

	pub fn as_str(&self) -> &'static str {
		match self {
			PendingEntity::Request => "RequestV2",
			PendingEntity::GetRequest => "GetRequestV2",
			PendingEntity::Order => "IOrderV3",
		}
	}

	/**
	 * Moves this entity's pending rows whose parent exists into its status table. `$1` is an
	 * optional commitment filter, `$2` the row limit. Hyperbridge's `PostRequestHandled` can't
	 * tell a request for Hyperbridge from one it routes until the request is indexed, so a
	 * `HYPERBRIDGE_DELIVERED` that waited becomes `DESTINATION` when Hyperbridge is the
	 * request's `dest`, as the handler decides for an indexed request.
	 */
	fn flush_sql(&self) -> &'static str {
		match self {
			PendingEntity::Request => concat!(
				"WITH ready AS (DELETE FROM app.pending_status_metadata p WHERE p.id IN (
				   SELECT p.id FROM app.pending_status_metadata p JOIN app.request_v2s r ON r.id = p.commitment
				   WHERE p.entity_type = 'RequestV2' AND ($1::text IS NULL OR p.commitment = $1) LIMIT $2)
				 RETURNING p.*),
				 resolved AS (SELECT ready.*, CASE
				   WHEN ready.status = 'HYPERBRIDGE_DELIVERED' AND ready.chain = r.dest THEN 'DESTINATION'
				   ELSE ready.status END AS resolved_status
				   FROM ready JOIN app.request_v2s r ON r.id = ready.commitment)
				 INSERT INTO app.request_status_metadata
				   (id, status, chain, timestamp, block_number, block_hash, transaction_hash, request_id, created_at)
				 SELECT commitment || '.' || resolved_status, resolved_status::app.\"167d3578e1\", chain,
				   timestamp, block_number, block_hash, transaction_hash, commitment, created_at FROM resolved",
				" ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status, chain = EXCLUDED.chain,
				   timestamp = EXCLUDED.timestamp, block_number = EXCLUDED.block_number,
				   block_hash = EXCLUDED.block_hash, transaction_hash = EXCLUDED.transaction_hash,
				   request_id = EXCLUDED.request_id, created_at = EXCLUDED.created_at"
			),
			PendingEntity::GetRequest => concat!(
				"WITH ready AS (DELETE FROM app.pending_status_metadata p WHERE p.id IN (
				   SELECT p.id FROM app.pending_status_metadata p JOIN app.get_request_v2s r ON r.id = p.commitment
				   WHERE p.entity_type = 'GetRequestV2' AND ($1::text IS NULL OR p.commitment = $1) LIMIT $2)
				 RETURNING p.*)
				 INSERT INTO app.get_request_status_metadata
				   (id, status, chain, timestamp, block_number, block_hash, transaction_hash, request_id, created_at)
				 SELECT commitment || '.' || status, status::app.\"167d3578e1\", chain, timestamp, block_number,
				   block_hash, transaction_hash, commitment, created_at FROM ready",
				" ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status, chain = EXCLUDED.chain,
				   timestamp = EXCLUDED.timestamp, block_number = EXCLUDED.block_number,
				   block_hash = EXCLUDED.block_hash, transaction_hash = EXCLUDED.transaction_hash,
				   request_id = EXCLUDED.request_id, created_at = EXCLUDED.created_at"
			),
			PendingEntity::Order => concat!(
				"WITH ready AS (DELETE FROM app.pending_status_metadata p WHERE p.id IN (
				   SELECT p.id FROM app.pending_status_metadata p JOIN app.i_order_v3s o ON o.id = p.commitment
				   WHERE p.entity_type = 'IOrderV3' AND ($1::text IS NULL OR p.commitment = $1) LIMIT $2)
				 RETURNING p.*)
				 INSERT INTO app.i_order_v3_status_metadata
				   (id, status, chain, timestamp, block_number, transaction_hash, order_id, filler, created_at)
				 SELECT commitment || '.' || status, status::app.\"869e90c211\", chain, timestamp, block_number,
				   transaction_hash, commitment, filler, created_at FROM ready",
				" ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status, chain = EXCLUDED.chain,
				   timestamp = EXCLUDED.timestamp, block_number = EXCLUDED.block_number,
				   transaction_hash = EXCLUDED.transaction_hash, order_id = EXCLUDED.order_id,
				   filler = EXCLUDED.filler, created_at = EXCLUDED.created_at"
			),
		}
	}
}

/** A status row waiting for its parent. Columns mirror the status tables. */
pub struct PendingStatus<'a> {
	pub commitment: &'a str,
	pub entity: PendingEntity,
	/** The `Status` or `OrderStatus` label, e.g. `DESTINATION` or `FILLED`. */
	pub status: &'a str,
	pub chain: &'a str,
	pub timestamp: Numeric,
	pub block_number: String,
	pub block_hash: &'a str,
	pub transaction_hash: &'a str,
	/** Orders only. */
	pub filler: Option<&'a str>,
	pub created_at: NaiveDateTime,
}

/** Parks a status until its parent exists. One row per parent and status; a replay overwrites. */
pub async fn defer_status(db: &Transaction<'_>, pending: &PendingStatus<'_>) -> anyhow::Result<()> {
	let entity = pending.entity.as_str();
	db.execute(
		"INSERT INTO app.pending_status_metadata
		   (id, commitment, entity_type, status, chain, timestamp, block_number, block_hash,
		    transaction_hash, filler, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
		 ON CONFLICT (id) DO UPDATE SET chain = EXCLUDED.chain, timestamp = EXCLUDED.timestamp,
		   block_number = EXCLUDED.block_number, block_hash = EXCLUDED.block_hash,
		   transaction_hash = EXCLUDED.transaction_hash, filler = EXCLUDED.filler,
		   created_at = EXCLUDED.created_at",
		&[
			&format!("{}.{entity}.{}", pending.commitment, pending.status),
			&pending.commitment,
			&entity,
			&pending.status,
			&pending.chain,
			&pending.timestamp,
			&pending.block_number,
			&pending.block_hash,
			&pending.transaction_hash,
			&pending.filler,
			&pending.created_at,
		],
	)
	.await?;
	Ok(())
}

/** `flushPendingStatuses`: call right after writing the parent row for `commitment`. */
pub async fn flush_commitment(
	db: &Transaction<'_>,
	entity: PendingEntity,
	commitment: &str,
) -> anyhow::Result<u64> {
	Ok(db.execute(entity.flush_sql(), &[&Some(commitment), &i64::MAX]).await?)
}

/**
 * The periodic flush: up to `limit` rows per entity whose parent now exists. It joins on the
 * parent, so rows whose parent never arrives can't block the rest (they did in SubQuery, which
 * took the first rows in storage order).
 */
pub async fn flush_ready(db: &Transaction<'_>, limit: i64) -> anyhow::Result<u64> {
	let mut flushed = 0;
	for entity in PendingEntity::ALL {
		flushed += db.execute(entity.flush_sql(), &[&None::<String>, &limit]).await?;
	}
	Ok(flushed)
}

/**
 * Runs the periodic flush. One flusher serves every chain: SubQuery ran one per chain only
 * because its per-chain history hid the other chains' rows.
 */
pub fn spawn_flusher(indexer: Arc<Indexer>) {
	tokio::spawn(async move {
		let mut interval = tokio::time::interval(Duration::from_secs(10));
		loop {
			interval.tick().await;
			let result = async {
				let mut conn = indexer.db.raw_connection().await?;
				let tx = conn.transaction().await?;
				let flushed = flush_ready(&tx, 100).await?;
				tx.commit().await?;
				anyhow::Ok(flushed)
			}
			.await;
			match result {
				Ok(0) => {},
				Ok(flushed) => tracing::info!(flushed, "flushed pending statuses"),
				Err(err) => tracing::warn!("pending status flush failed: {err:#}"),
			}
		}
	});
}
