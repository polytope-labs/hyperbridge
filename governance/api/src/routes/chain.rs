use std::{convert::Infallible, time::Duration};

use axum::{
	body::Body,
	extract::{RawQuery, State},
	Json,
};
use futures::stream;
use serde_json::{json, Value};

use super::{referenda::track_counts, AppState, Params};
use crate::error::ApiResult;

/// The last block the governance indexer has processed.
async fn indexed_height(state: &AppState) -> ApiResult<i64> {
	let row = state
		.db
		.query_opt("SELECT value FROM governance._metadata WHERE key = 'lastProcessedHeight'", &[])
		.await?;
	Ok(row.and_then(|row| row.get::<_, Value>(0).as_i64()).unwrap_or_default())
}

pub async fn scan_height(State(state): State<AppState>) -> ApiResult<Json<Value>> {
	Ok(Json(json!({ "value": indexed_height(&state).await? })))
}

/// Streams the indexed height as one JSON object per line, which the frontend's header follows.
pub async fn scan_height_stream(State(state): State<AppState>, RawQuery(query): RawQuery) -> Body {
	let interval = Params::new(query)
		.get("interval")
		.and_then(|value| value.parse().ok())
		.unwrap_or(6000u64)
		.max(1000);

	Body::from_stream(stream::unfold((state, true), move |(state, first)| async move {
		if !first {
			tokio::time::sleep(Duration::from_millis(interval)).await;
		}
		let line = indexed_height(&state)
			.await
			.map(|height| format!("{{\"value\":{height}}}\n"))
			.ok()?;
		Some((Ok::<_, Infallible>(line), (state, false)))
	}))
}

pub async fn summary(State(state): State<AppState>) -> ApiResult<Json<Value>> {
	let row = state
		.db
		.query_one(
			"SELECT
				(SELECT count(*) FROM posts WHERE kind = 0 AND NOT hidden) AS discussions,
				(SELECT count(*) FROM posts WHERE kind = 0 AND NOT hidden AND last_activity_at > NOW() - INTERVAL '7 days')
					AS active_discussions,
				(SELECT count(*) FROM governance_referendums) AS referenda,
				(SELECT count(*) FROM governance_referendums WHERE NOT is_final) AS active_referenda,
				(SELECT count(*) FROM governance_motions) AS motions,
				(SELECT count(*) FROM governance_motions WHERE NOT is_final) AS active_motions,
				(SELECT count(*) FROM governance_treasury_spends) AS spends,
				(SELECT count(*) FROM governance_treasury_spends WHERE state = 'Approved') AS active_spends",
			&[],
		)
		.await?;
	let count = |all: &str, active: &str| json!({ "active": row.get::<_, i64>(active), "all": row.get::<_, i64>(all) });

	Ok(Json(json!({
		"discussions": count("discussions", "active_discussions"),
		"gov2Referenda": count("referenda", "active_referenda"),
		"techCommMotions": count("motions", "active_motions"),
		"treasurySpends": count("spends", "active_spends"),
		"gov2ReferendaTracks": track_counts(&state).await?,
	})))
}
