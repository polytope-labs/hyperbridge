use std::{convert::Infallible, time::Duration};

use axum::{
	body::Body,
	extract::{RawQuery, State},
	Json,
};
use futures::stream;
use serde_json::{json, Value};

use super::{
	motions, posts, referenda, referenda::track_counts, spends, with_param, AppState, Params,
};
use crate::{error::ApiResult, primitives::Page};

/// The last block the governance indexer has processed.
async fn indexed_height(state: &AppState) -> ApiResult<i64> {
	let row = state
		.db
		.query_opt("SELECT value FROM _metadata WHERE key = 'lastProcessedHeight'", &[])
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
		// A failed read skips this beat instead of ending the stream, so it recovers on its own
		let line = indexed_height(&state)
			.await
			.map(|height| format!("{{\"value\":{height}}}\n"))
			.unwrap_or_default();
		Some((Ok::<_, Infallible>(line), (state, false)))
	}))
}

pub async fn summary(State(state): State<AppState>) -> ApiResult<Json<Value>> {
	let row = state
		.db
		.query_one(
			"SELECT
				(SELECT count(*) FROM posts WHERE kind = 0 AND NOT hidden) AS discussions,
				(SELECT count(*) FROM posts p WHERE kind = 0 AND NOT hidden AND GREATEST(p.created_at,
					(SELECT max(created_at) FROM comments WHERE post_kind = 0 AND post_number = p.number))
					> NOW() - INTERVAL '7 days')
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

// The overview lists only what is live: open referenda, motions and spends, and recent discussions

pub async fn overview_referenda(
	state: State<AppState>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	referenda::list(state, RawQuery(Some(with_param(query, "ongoing=true")))).await
}

pub async fn overview_motions(
	state: State<AppState>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	motions::list(state, RawQuery(Some(with_param(query, "ongoing=true")))).await
}

pub async fn overview_spends(
	state: State<AppState>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	spends::list(state, RawQuery(Some(with_param(query, "ongoing=true")))).await
}

pub async fn overview_discussions(
	state: State<AppState>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	posts::list(state, RawQuery(Some(with_param(query, "active=true")))).await
}
