use axum::{
	extract::{RawQuery, State},
	Json,
};
use serde_json::{json, Value};

use super::{AppState, Params};
use crate::{error::ApiResult, primitives::PostKind};

const RESULTS_PER_CATEGORY: i64 = 10;

/// Items of one kind whose index is the search text or whose forum title or content matches
/// `pattern`, keyed by the index field the frontend reads for that category.
async fn found(
	state: &AppState,
	table: &str,
	kind: PostKind,
	index_field: &str,
	text: &str,
	pattern: &str,
) -> ApiResult<Vec<Value>> {
	let kind = kind as i16;
	let rows = state
		.db
		.query(
			&format!(
				"SELECT i.index, p.title, p.content FROM {table} i
				LEFT JOIN posts p ON p.kind = {kind} AND p.number = i.index
				WHERE i.index::text = $1 OR p.title ILIKE $2 OR p.content ILIKE $2
				ORDER BY i.index DESC LIMIT {RESULTS_PER_CATEGORY}"
			),
			&[&text, &pattern],
		)
		.await?;
	Ok(rows
		.iter()
		.map(|row| {
			json!({
				index_field: row.get::<_, i32>("index"),
				"title": row.get::<_, Option<String>>("title"),
				"content": row.get::<_, Option<String>>("content"),
			})
		})
		.collect())
}

/// What the header search box shows, grouped by category as the frontend reads it.
pub async fn search(
	State(state): State<AppState>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Value>> {
	let params = Params::new(query);
	let text = params.get("text").unwrap_or_default().trim();
	if text.is_empty() {
		return Ok(Json(json!({})));
	}
	let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
	let pattern = format!("%{escaped}%");
	let referenda = found(
		&state,
		"governance_referendums",
		PostKind::Referendum,
		"referendumIndex",
		text,
		&pattern,
	);
	let spends = found(
		&state,
		"governance_treasury_spends",
		PostKind::TreasurySpend,
		"index",
		text,
		&pattern,
	);
	Ok(Json(json!({ "openGovReferenda": referenda.await?, "treasurySpends": spends.await? })))
}
