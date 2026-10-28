use axum::{
	extract::{Path, RawQuery, State},
	Json,
};
use serde_json::{json, Value};
use tokio_postgres::Row;

use super::{
	common::{item_index, parse},
	AppState, Params,
};
use crate::{
	error::{ApiError, ApiResult},
	primitives::{iso, post_id, Page, PostKind},
};

const SPENDS: &str = "
	SELECT s.index, s.asset_kind, s.amount::text AS amount, s.beneficiary, s.valid_from, s.expire_at, s.state,
		s.payment_id, s.approved_block, ms(s.approved_at) AS approved_ms, p.title, p.content, p.content_type
	FROM governance_treasury_spends s
	LEFT JOIN posts p ON p.kind = 3 AND p.number = s.index";

/// Spends still waiting to be paid out.
const PENDING: &str = "s.state = 'Approved'";

fn spend(row: &Row) -> Value {
	let index = row.get::<_, i32>("index");
	let state = row.get::<_, String>("state");
	json!({
		"_id": post_id(PostKind::TreasurySpend, index.into()),
		"index": index,
		"title": row.get::<_, Option<String>>("title"),
		"content": row.get::<_, Option<String>>("content"),
		"contentType": row.get::<_, Option<String>>("content_type").unwrap_or_else(|| "markdown".into()),
		"state": state,
		"isFinal": state != "Approved",
		"createdAt": iso(row.get("approved_ms")),
		"onchainData": {
			"index": index,
			"meta": {
				"assetKind": parse(row.get("asset_kind")),
				"amount": row.get::<_, String>("amount"),
				"beneficiary": parse(row.get("beneficiary")),
				"validFrom": row.get::<_, i32>("valid_from"),
				"expireAt": row.get::<_, i32>("expire_at"),
			},
			"paymentId": row.get::<_, Option<String>>("payment_id"),
			"state": { "state": state },
			"indexer": { "blockHeight": row.get::<_, i32>("approved_block"), "blockTime": row.get::<_, i64>("approved_ms") },
		},
	})
}

pub async fn list(
	State(state): State<AppState>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	let params = Params::new(query);
	let paging = params.paging(10);
	let filter = if params.flag("ongoing") { PENDING } else { "TRUE" };
	let rows = state
		.db
		.query(
			&format!(
				"{SPENDS} WHERE {filter} ORDER BY s.index DESC LIMIT {} OFFSET {}",
				paging.page_size,
				paging.offset()
			),
			&[],
		)
		.await?;
	let total = state
		.db
		.query_one(
			&format!("SELECT count(*) FROM governance_treasury_spends s WHERE {filter}"),
			&[],
		)
		.await?
		.get::<_, i64>(0);
	Ok(Json(paging.page(rows.iter().map(spend).collect(), total)))
}

pub async fn detail(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
	let index = item_index(&id, PostKind::TreasurySpend, "Treasury spend")?;
	let row = state
		.db
		.query_opt(&format!("{SPENDS} WHERE s.index = $1"), &[&index])
		.await?
		.ok_or_else(|| ApiError::not_found("Treasury spend"))?;
	Ok(Json(spend(&row)))
}

pub async fn summary(State(state): State<AppState>) -> ApiResult<Json<Value>> {
	let rows = state
		.db
		.query("SELECT state, count(*) FROM governance_treasury_spends GROUP BY state", &[])
		.await?;
	let detail = rows
		.iter()
		.map(|row| (row.get::<_, String>(0), json!({ "proposalsCount": row.get::<_, i64>(1) })))
		.collect::<serde_json::Map<_, _>>();
	let total = rows.iter().map(|row| row.get::<_, i64>(1)).sum::<i64>();
	Ok(Json(json!({ "totalProposalsCount": total, "detail": detail })))
}
