//! `RequestService`, `GetRequestService` and `GetResponseService`: the request rows, their
//! status rows, and statuses parked until their request exists.

use serde_json::Value;
use tokio_postgres::Transaction;

use crate::{
	services::pending::{defer_status, flush_commitment, PendingEntity, PendingStatus},
	store::{numeric, timestamp_to_date, Numeric, Status},
};

/** A status event for a POST or GET request. */
pub struct StatusUpdate<'a> {
	pub commitment: &'a str,
	pub status: Status,
	pub chain: &'a str,
	/** Seconds on EVM chains, milliseconds on Substrate chains, stored as given. */
	pub timestamp: u64,
	pub block_number: u64,
	pub block_hash: &'a str,
	pub transaction_hash: &'a str,
}

/** `request_v2s` as `RequestService.createOrUpdate` writes it. */
pub struct PostRequestRow<'a> {
	pub commitment: &'a str,
	pub chain: &'a str,
	pub source: &'a str,
	pub dest: &'a str,
	pub from: &'a str,
	pub to: &'a str,
	pub body: &'a str,
	pub nonce: Numeric,
	pub timeout_timestamp: Numeric,
	pub fee: Numeric,
	pub block_timestamp: u64,
}

/**
 * `RequestService.createOrUpdate`. A new row takes every field and flushes the statuses that
 * waited for it. An existing row keeps its `chain` and `created_at` and is not flushed.
 */
pub async fn upsert_post_request(
	db: &Transaction<'_>,
	row: &PostRequestRow<'_>,
) -> anyhow::Result<()> {
	let inserted: bool = db
		.query_one(
			"INSERT INTO app.request_v2s
			   (id, commitment, chain, source, dest, timeout_timestamp, \"from\", \"to\", nonce, body, fee, created_at)
			 VALUES ($1, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
			 ON CONFLICT (id) DO UPDATE SET source = EXCLUDED.source, dest = EXCLUDED.dest,
			   timeout_timestamp = EXCLUDED.timeout_timestamp, \"from\" = EXCLUDED.\"from\",
			   \"to\" = EXCLUDED.\"to\", nonce = EXCLUDED.nonce, body = EXCLUDED.body, fee = EXCLUDED.fee
			 RETURNING (xmax = 0)",
			&[
				&row.commitment,
				&row.chain,
				&row.source,
				&row.dest,
				&row.timeout_timestamp,
				&row.from,
				&row.to,
				&row.nonce,
				&row.body,
				&row.fee,
				&timestamp_to_date(row.block_timestamp),
			],
		)
		.await?
		.get(0);
	if inserted {
		flush_commitment(db, PendingEntity::Request, row.commitment).await?;
	}
	Ok(())
}

/** `get_request_v2s` as `GetRequestService.createOrUpdate` writes it from a `GetRequestEvent`. */
pub struct GetRequestRow<'a> {
	pub commitment: &'a str,
	pub chain: &'a str,
	pub source: &'a str,
	pub dest: &'a str,
	pub from: &'a str,
	pub keys: Vec<String>,
	pub nonce: Numeric,
	pub height: Numeric,
	pub context: &'a str,
	pub timeout_timestamp: Numeric,
	pub fee: Numeric,
	pub block_number: u64,
	pub block_hash: &'a str,
	pub transaction_hash: &'a str,
	pub block_timestamp: u64,
}

/**
 * `GetRequestService.createOrUpdate` with every field given. An existing row, such as the
 * skeleton `GetRequestHandled` writes on Hyperbridge, takes every field except `created_at`.
 */
pub async fn upsert_get_request(
	db: &Transaction<'_>,
	row: &GetRequestRow<'_>,
) -> anyhow::Result<()> {
	let keys = Value::from(row.keys.clone());
	let inserted: bool = db
		.query_one(
			"INSERT INTO app.get_request_v2s
			   (id, source, dest, \"from\", keys, nonce, height, context, timeout_timestamp, fee,
			    block_number, block_hash, transaction_hash, block_timestamp, chain, commitment, created_at)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $1, $16)
			 ON CONFLICT (id) DO UPDATE SET source = EXCLUDED.source, dest = EXCLUDED.dest,
			   \"from\" = EXCLUDED.\"from\", keys = EXCLUDED.keys, nonce = EXCLUDED.nonce,
			   height = EXCLUDED.height, context = EXCLUDED.context,
			   timeout_timestamp = EXCLUDED.timeout_timestamp, fee = EXCLUDED.fee,
			   block_number = EXCLUDED.block_number, block_hash = EXCLUDED.block_hash,
			   transaction_hash = EXCLUDED.transaction_hash, block_timestamp = EXCLUDED.block_timestamp,
			   chain = EXCLUDED.chain
			 RETURNING (xmax = 0)",
			&[
				&row.commitment,
				&row.source,
				&row.dest,
				&row.from,
				&keys,
				&row.nonce,
				&row.height,
				&row.context,
				&row.timeout_timestamp,
				&row.fee,
				&row.block_number.to_string(),
				&row.block_hash,
				&row.transaction_hash,
				&numeric(row.block_timestamp),
				&row.chain,
				&timestamp_to_date(row.block_timestamp),
			],
		)
		.await?
		.get(0);
	if inserted {
		flush_commitment(db, PendingEntity::GetRequest, row.commitment).await?;
	}
	Ok(())
}

/**
 * `GetRequestService.createOrUpdate({ id })`: the row with every field empty, if there is none.
 * The TypeScript dated it `Date.now()`; `created_at` is the caller's block time instead, which
 * is what that clock read while indexing at the head. Returns the row's `source`.
 */
pub async fn ensure_get_request(
	db: &Transaction<'_>,
	commitment: &str,
	created_at_ms: u64,
) -> anyhow::Result<String> {
	let inserted = db
		.execute(
			"INSERT INTO app.get_request_v2s
			   (id, source, dest, \"from\", keys, nonce, height, context, timeout_timestamp, fee,
			    block_number, block_hash, transaction_hash, block_timestamp, chain, commitment, created_at)
			 VALUES ($1, '', '', '', '[\"\"]', 0, 0, '', 0, 0, '', '', '', 0, '', $1, $2)
			 ON CONFLICT (id) DO NOTHING",
			&[&commitment, &timestamp_to_date(created_at_ms)],
		)
		.await?;
	if inserted == 1 {
		flush_commitment(db, PendingEntity::GetRequest, commitment).await?;
	}
	let source: String = db
		.query_one("SELECT source FROM app.get_request_v2s WHERE id = $1", &[&commitment])
		.await?
		.get(0);
	Ok(source)
}

/** The `source` and `dest` of a POST request, if it is indexed. */
pub async fn post_request_route(
	db: &Transaction<'_>,
	commitment: &str,
) -> anyhow::Result<Option<(Option<String>, Option<String>)>> {
	Ok(db
		.query_opt("SELECT source, dest FROM app.request_v2s WHERE id = $1", &[&commitment])
		.await?
		.map(|row| (row.get(0), row.get(1))))
}

/** The `source` of a GET request, if it is indexed. */
pub async fn get_request_source(
	db: &Transaction<'_>,
	commitment: &str,
) -> anyhow::Result<Option<String>> {
	Ok(db
		.query_opt("SELECT source FROM app.get_request_v2s WHERE id = $1", &[&commitment])
		.await?
		.map(|row| row.get(0)))
}

/**
 * `RequestService.updateStatus` / `GetRequestService.updateStatus`: the status row, or a
 * pending row while the request isn't indexed yet. Both are keyed by commitment and status.
 */
pub async fn update_status(
	db: &Transaction<'_>,
	entity: PendingEntity,
	update: &StatusUpdate<'_>,
) -> anyhow::Result<()> {
	let (parent, table) = match entity {
		PendingEntity::Request => ("request_v2s", "request_status_metadata"),
		PendingEntity::GetRequest => ("get_request_v2s", "get_request_status_metadata"),
		PendingEntity::Order => anyhow::bail!("order statuses belong to the intent gateway"),
	};
	let exists = db
		.query_opt(&format!("SELECT 1 FROM app.{parent} WHERE id = $1"), &[&update.commitment])
		.await?
		.is_some();
	let block_number = update.block_number.to_string();
	let created_at = timestamp_to_date(update.timestamp);
	if !exists {
		return defer_status(
			db,
			&PendingStatus {
				commitment: update.commitment,
				entity,
				status: update.status.as_str(),
				chain: update.chain,
				timestamp: numeric(update.timestamp),
				block_number,
				block_hash: update.block_hash,
				transaction_hash: update.transaction_hash,
				filler: None,
				created_at,
			},
		)
		.await;
	}
	insert_status(db, table, update).await
}

/** The `SOURCE` row a request's own event writes, without checking for the request. */
pub async fn insert_status(
	db: &Transaction<'_>,
	table: &str,
	update: &StatusUpdate<'_>,
) -> anyhow::Result<()> {
	db.execute(
		&format!(
			"INSERT INTO app.{table}
			   (id, status, chain, timestamp, block_number, block_hash, transaction_hash, request_id, created_at)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
			 ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status, chain = EXCLUDED.chain,
			   timestamp = EXCLUDED.timestamp, block_number = EXCLUDED.block_number,
			   block_hash = EXCLUDED.block_hash, transaction_hash = EXCLUDED.transaction_hash,
			   request_id = EXCLUDED.request_id, created_at = EXCLUDED.created_at"
		),
		&[
			&format!("{}.{}", update.commitment, update.status.as_str()),
			&update.status,
			&update.chain,
			&numeric(update.timestamp),
			&update.block_number.to_string(),
			&update.block_hash,
			&update.transaction_hash,
			&update.commitment,
			&timestamp_to_date(update.timestamp),
		],
	)
	.await?;
	Ok(())
}

/** `get_responses` as `GetResponseService.findOrCreate` writes it. */
pub struct GetResponseRow<'a> {
	pub commitment: &'a str,
	pub chain: &'a str,
	pub request: &'a str,
	/** The response values, `null` where the key had no value. */
	pub values: Vec<Option<String>>,
	pub timeout_timestamp: Numeric,
	pub block_timestamp: u64,
	pub block_number: u64,
	pub block_hash: &'a str,
	pub transaction_hash: &'a str,
}

/**
 * `GetResponseService.findOrCreate`: the first write wins. `request_id` references
 * `get_request_v2s`; when that row is missing it is left empty, where the TypeScript's insert
 * failed the block.
 */
pub async fn insert_get_response(
	db: &Transaction<'_>,
	row: &GetResponseRow<'_>,
) -> anyhow::Result<()> {
	let request = db
		.query_opt("SELECT id FROM app.get_request_v2s WHERE id = $1", &[&row.request])
		.await?
		.map(|r| r.get::<_, String>(0));
	if request.is_none() {
		tracing::warn!(
			commitment = row.commitment,
			request = row.request,
			"GET response for a request that isn't indexed; storing it without request_id"
		);
	}
	let values = Value::from(
		row.values
			.iter()
			.map(|v| v.clone().map(Value::from).unwrap_or(Value::Null))
			.collect::<Vec<_>>(),
	);
	db.execute(
		"INSERT INTO app.get_responses
		   (id, commitment, chain, response_message, response_timeout_timestamp, request_id, created_at,
		    block_number, block_hash, transaction_hash)
		 VALUES ($1, $1, $2, $3, $4, $5, $6, $7, $8, $9)
		 ON CONFLICT (id) DO NOTHING",
		&[
			&row.commitment,
			&row.chain,
			&values,
			&row.timeout_timestamp,
			&request,
			&timestamp_to_date(row.block_timestamp),
			&row.block_number.to_string(),
			&row.block_hash,
			&row.transaction_hash,
		],
	)
	.await?;
	Ok(())
}
