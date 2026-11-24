use std::{convert::Infallible, net::SocketAddr, time::Duration};

use axum::{
	body::Body,
	extract::{ConnectInfo, RawQuery, State},
	http::HeaderMap,
	Json,
};
use futures::stream;
use serde_json::{json, Value};
use tokio::sync::watch;

use super::{
	motions, posts, referenda, referenda::track_counts, spends, with_param, AppState, Params,
};
use crate::{
	db::Pool,
	error::{ApiError, ApiResult},
	primitives::Page,
};

/// How often the indexed height is read on behalf of every visitor following it.
const HEIGHT_POLL: Duration = Duration::from_secs(3);

/// The last block the governance entry has processed, which omniplex records in
/// `governance._metadata`. Before its first progress the table is empty and the height reads as 0.
async fn indexed_height(db: &Pool) -> anyhow::Result<i64> {
	let connection = db.get().await?;
	let row = connection
		.query_opt("SELECT value FROM governance._metadata WHERE key = 'lastProcessedHeight'", &[])
		.await?;
	Ok(row.and_then(|row| row.get::<_, Value>(0).as_i64()).unwrap_or_default())
}

/// Reads the indexed height on a timer and publishes it, so visitors following it never query the
/// database themselves.
pub async fn follow_height(db: Pool, height: watch::Sender<i64>) {
	let mut interval = tokio::time::interval(HEIGHT_POLL);
	loop {
		interval.tick().await;
		match indexed_height(&db).await {
			Ok(value) => {
				height.send_if_modified(|current| std::mem::replace(current, value) != value);
			},
			Err(err) => {
				tracing::warn!(target: crate::LOG_TARGET, "reading the indexed height failed: {err:?}");
			},
		}
	}
}

pub async fn scan_height(State(state): State<AppState>) -> Json<Value> {
	Json(json!({ "value": *state.height.borrow() }))
}

/// Streams the indexed height as one JSON object per line, which the frontend's header follows. A
/// line goes out whenever the height changes, and at least every `interval` milliseconds. The
/// number of open streams is capped in all and for each visitor, since each holds a connection
/// open.
pub async fn scan_height_stream(
	State(state): State<AppState>,
	ConnectInfo(peer): ConnectInfo<SocketAddr>,
	headers: HeaderMap,
	RawQuery(query): RawQuery,
) -> ApiResult<Body> {
	let interval = Params::new(query)
		.get("interval")
		.and_then(|value| value.parse().ok())
		.unwrap_or(6000u64)
		.max(1000);
	let permit = state.streams.clone().try_acquire_owned().map_err(|_| ApiError::busy())?;
	let slot = state
		.limiter
		.open_stream(state.limiter.visitor(&headers, peer))
		.ok_or_else(ApiError::busy)?;
	let mut height = state.height.clone();
	height.mark_changed();

	Ok(Body::from_stream(stream::unfold(
		(height, (permit, slot)),
		move |(mut height, held)| async move {
			// The sender lives as long as the server, so a closed channel means it is shutting down
			if let Ok(Err(_)) =
				tokio::time::timeout(Duration::from_millis(interval), height.changed()).await
			{
				return None;
			}
			let line = format!("{{\"value\":{}}}\n", *height.borrow_and_update());
			Some((Ok::<_, Infallible>(line), (height, held)))
		},
	)))
}

pub async fn summary(State(state): State<AppState>) -> ApiResult<Json<Value>> {
	let row = state
		.db
		.get()
		.await?
		.query_one(
			&format!(
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
				(SELECT count(*) FROM governance_treasury_spends s WHERE {}) AS active_spends",
				spends::pending()
			),
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
