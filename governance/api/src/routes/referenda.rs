use axum::{
	extract::{Path, RawQuery, State},
	Json,
};
use serde_json::{json, Value};
use tokio_postgres::{types::ToSql, Row};

use super::{
	common::{
		decimal, item_index, last_event, parse, post_fields, post_joins, timeline, POST_COLUMNS,
	},
	posts::post_reactions,
	with_param, AppState, Params,
};
use crate::{
	error::{ApiError, ApiResult},
	primitives::{post_id, Page, Paging, PostKind},
};

// A referendum's stage while it is live, read from the last ongoing `ReferendumInfoFor` value.
const DECIDING: &str = "jsonb_typeof(r.info::jsonb -> 'deciding') = 'object'";
const CONFIRMING: &str = "jsonb_typeof(r.info::jsonb #> '{deciding,confirming}') = 'number'";

fn query(filter: &str) -> String {
	format!(
		"SELECT r.index, r.track, r.proposer, r.proposal, r.proposal_hash, r.call, r.is_final, r.info,
			r.ayes::text AS ayes, r.nays::text AS nays, r.support::text AS support,
			t.name AS track_name, {POST_COLUMNS},
			CASE WHEN r.is_final THEN COALESCE(r.track_info, t.info) ELSE t.info END AS track_info
		FROM governance_referendums r
		LEFT JOIN governance_tracks t ON t.id = r.track::text
		{}
		WHERE {filter}",
		post_joins(
			"governance_referendum_events",
			"referendum_id = r.id AND changes_state",
			PostKind::Referendum,
			"r.index"
		),
	)
}

fn name_words(name: &str) -> impl Iterator<Item = &str> {
	name.trim_end_matches('\0').split('_').filter(|word| !word.is_empty())
}

/// A track as the forum describes it, with its padded on-chain name kept as `originalName`.
fn track(id: i32, info: Option<String>) -> Value {
	let mut track = parse(info);
	if let Value::Object(fields) = &mut track {
		let original = fields.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
		let deposit = fields.get("decisionDeposit").map(decimal).unwrap_or(Value::Null);
		// Nexus names each track's origin after the track, so `whitelisted_caller` is
		// `WhitelistedCaller`
		let origin = name_words(&original)
			.map(|word| word[..1].to_uppercase() + &word[1..])
			.collect::<String>();
		fields.insert("id".into(), id.into());
		fields.insert("name".into(), original.trim_end_matches('\0').into());
		fields.insert("originalName".into(), original.into());
		fields.insert("origin".into(), origin.into());
		fields.insert("decisionDeposit".into(), deposit);
	}
	track
}

/// Event args under the names the forum's timeline uses. The referendum index is implied, deposits
/// are decimal strings and the submission also names the proposer and proposal hash.
fn timeline_args(name: &str, args: Value, proposer: &str, proposal_hash: &Value) -> Value {
	let Value::Object(mut fields) = args else { return args };
	let index = fields.remove("index");
	if let Some(amount) = fields.get("amount").map(decimal) {
		fields.insert("amount".into(), amount);
	}
	match name {
		"Submitted" => {
			fields.insert("referendumIndex".into(), index.unwrap_or_default());
			fields.insert("proposer".into(), proposer.into());
			fields.insert("proposalHash".into(), proposal_hash.clone());
		},
		"DecisionStarted" => {
			fields.remove("proposal");
		},
		_ => {},
	}
	Value::Object(fields)
}

fn referendum(row: &Row) -> Value {
	let index = row.get::<_, i32>("index");
	let is_final = row.get::<_, bool>("is_final");
	let proposer = row.get::<_, Option<String>>("proposer").unwrap_or_default();
	let track_info = track(row.get("track"), row.get("track_info"));
	let proposal_hash = Value::from(row.get::<_, Option<String>>("proposal_hash"));
	let mut state = last_event(row);
	// A confirmed referendum is `Approved` on chain until its enactment runs, and stays so if it
	// fails
	if state["name"] == "Confirmed" {
		state["name"] = "Approved".into();
	}
	state["args"] = timeline_args(
		&row.get::<_, String>("l_name"),
		state["args"].take(),
		&proposer,
		&proposal_hash,
	);
	let proposal = match parse(row.get("call")) {
		Value::Null => parse(row.get("proposal")),
		call => json!({ "call": call, "shorten": false }),
	};
	let tally = json!({
		"ayes": row.get::<_, String>("ayes"),
		"nays": row.get::<_, String>("nays"),
		"support": row.get::<_, String>("support"),
	});

	let mut post = post_fields(row, PostKind::Referendum, index.into(), &proposer);
	post.extend([
		("referendumIndex".into(), index.into()),
		("track".into(), row.get::<_, i32>("track").into()),
		("trackInfo".into(), track_info.clone()),
		("state".into(), state.clone()),
		("stateSort".into(), if is_final { 5 } else { 1 }.into()),
		("isFinal".into(), is_final.into()),
		(
			"onchainData".into(),
			json!({
				"referendumIndex": index,
				"indexer": post["indexer"],
				"track": row.get::<_, i32>("track"),
				"trackInfo": track_info,
				"proposer": proposer,
				"proposal": proposal,
				"proposalHash": proposal_hash,
				"state": state,
				"info": parse(row.get("info")),
				"tally": tally,
				"isFinal": is_final,
			}),
		),
	]);
	Value::Object(post)
}

pub async fn list(
	State(state): State<AppState>,
	RawQuery(query_string): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	let params = Params::new(query_string);
	let paging = params.paging(10);
	let indexes = params
		.all("referendum_index")
		.filter_map(|index| index.parse().ok())
		.collect::<Vec<i32>>();

	let mut filters = vec!["TRUE".to_owned()];
	let mut values: Vec<Box<dyn ToSql + Sync + Send>> = Vec::new();
	if params.flag("ongoing") {
		filters.push("NOT r.is_final".into());
	}
	if params.flag("is_treasury") {
		filters.push("(t.name LIKE '%spend%' OR t.name LIKE '%treasur%')".into());
	}
	if !indexes.is_empty() {
		values.push(Box::new(indexes));
		filters.push(format!("r.index = ANY(${})", values.len()));
	}
	if let Some(proposer) = params.get("proposer") {
		values.push(Box::new(proposer.to_owned()));
		filters.push(format!("r.proposer = ${}", values.len()));
	}
	if let Some(track) = params.get("track").and_then(|track| track.parse::<i32>().ok()) {
		values.push(Box::new(track));
		filters.push(format!("r.track = ${}", values.len()));
	}
	if let Some(status) = params.get("status") {
		filters.push(match status {
			"Preparing" => format!("NOT r.is_final AND NOT {DECIDING}"),
			"Deciding" => format!("NOT r.is_final AND {DECIDING} AND NOT {CONFIRMING}"),
			"Confirming" => format!("NOT r.is_final AND {CONFIRMING}"),
			_ => {
				values.push(Box::new(status.to_owned()));
				format!("l.name = ${}", values.len())
			},
		});
	}

	page(&state, &filters.join(" AND "), &values, paging).await.map(Json)
}

async fn page(
	state: &AppState,
	filter: &str,
	values: &[Box<dyn ToSql + Sync + Send>],
	paging: Paging,
) -> ApiResult<Page<Value>> {
	let params = values
		.iter()
		.map(|value| value.as_ref() as &(dyn ToSql + Sync))
		.collect::<Vec<_>>();
	let rows = state
		.db
		.query(
			&format!(
				"{} ORDER BY r.is_final, r.index DESC LIMIT {} OFFSET {}",
				query(filter),
				paging.page_size,
				paging.offset()
			),
			&params,
		)
		.await?;
	let total = state
		.db
		.query_one(&format!("SELECT count(*) FROM ({}) items", query(filter)), &params)
		.await?
		.get::<_, i64>(0);
	Ok(paging.page(rows.iter().map(referendum).collect(), total))
}

/// A referendum as the lists show it.
pub async fn referendum_by_index(state: &AppState, index: i32) -> ApiResult<Value> {
	let row = state
		.db
		.query_opt(&query("r.index = $1"), &[&index])
		.await?
		.ok_or_else(|| ApiError::not_found("Referendum"))?;
	Ok(referendum(&row))
}

/// A referendum by its index or by the post id the frontend uses for comments and edits.
pub async fn detail(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
	let index = item_index(&id, PostKind::Referendum, "Referendum")?;
	let mut detail = referendum_by_index(&state, index).await?;
	let proposer = detail["proposer"].as_str().unwrap_or_default().to_owned();
	let proposal_hash = detail["onchainData"]["proposalHash"].clone();
	let timeline = timeline(
		&state.db,
		"governance_referendum_events",
		"referendum_id = $1 AND changes_state",
		index.to_string(),
	)
	.await?
	.into_iter()
	.map(|(name, args, indexer)| {
		let args = timeline_args(&name, args, &proposer, &proposal_hash);
		json!({ "name": name, "args": args, "indexer": indexer })
	})
	.collect::<Vec<_>>();

	detail["onchainData"]["timeline"] = timeline.into();
	detail["authors"] = json!([detail["proposer"]]);
	detail["reactions"] =
		post_reactions(&state, post_id(PostKind::Referendum, index.into())).await?;
	Ok(Json(detail))
}

pub async fn summary(State(state): State<AppState>) -> ApiResult<Json<Value>> {
	stage_counts(&state, None).await
}

pub async fn track_summary(
	State(state): State<AppState>,
	Path(track): Path<i32>,
) -> ApiResult<Json<Value>> {
	stage_counts(&state, Some(track)).await
}

/// How many referenda are at each stage, across every track or within one.
async fn stage_counts(state: &AppState, track: Option<i32>) -> ApiResult<Json<Value>> {
	let row = state
		.db
		.query_one(
			&format!(
				"SELECT count(*) FILTER (WHERE NOT r.is_final) AS active,
					count(*) FILTER (WHERE NOT r.is_final AND {DECIDING}) AS deciding,
					count(*) FILTER (WHERE NOT r.is_final AND {CONFIRMING}) AS confirming,
					count(*) FILTER (WHERE NOT r.is_final AND NOT {DECIDING}) AS preparing,
					count(*) AS total
				FROM governance_referendums r WHERE $1::int IS NULL OR r.track = $1"
			),
			&[&track],
		)
		.await?;
	Ok(Json(json!({
		"activeCount": row.get::<_, i64>("active"),
		"decidingCount": row.get::<_, i64>("deciding"),
		"confirmingCount": row.get::<_, i64>("confirming"),
		"preparingCount": row.get::<_, i64>("preparing"),
		"total": row.get::<_, i64>("total"),
	})))
}

pub async fn track_detail(
	State(state): State<AppState>,
	Path(id): Path<i32>,
) -> ApiResult<Json<Value>> {
	let row = state
		.db
		.query_opt("SELECT info FROM governance_tracks WHERE id = $1::int::text", &[&id])
		.await?
		.ok_or_else(|| ApiError::not_found("Track"))?;
	Ok(Json(track(id, row.get(0))))
}

/// The referenda of one track, the same list with the track from the path.
pub async fn track_list(
	state: State<AppState>,
	Path(id): Path<i32>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	list(state, RawQuery(Some(with_param(query, &format!("track={id}"))))).await
}

/// A preimage's bytes, which the frontend decodes into the proposal call. They are kept after the
/// preimage is cleared on chain.
pub async fn preimage(
	State(state): State<AppState>,
	Path(hash): Path<String>,
) -> ApiResult<Json<Value>> {
	let row = state
		.db
		.query_opt(
			"SELECT length, data, status FROM governance_preimages WHERE id = $1 AND data IS NOT NULL",
			&[&hash],
		)
		.await?
		.ok_or_else(|| ApiError::not_found("Preimage"))?;
	Ok(Json(json!({
		"hash": hash,
		"length": row.get::<_, Option<i32>>(0),
		"hex": row.get::<_, String>(1),
		"status": row.get::<_, String>(2),
	})))
}

pub async fn tracks(State(state): State<AppState>) -> ApiResult<Json<Vec<Value>>> {
	let rows = state
		.db
		.query("SELECT id::int, info FROM governance_tracks ORDER BY id::int", &[])
		.await?;
	Ok(Json(rows.iter().map(|row| track(row.get(0), row.get(1))).collect()))
}

/// Each track with how many of its referenda are live, which the frontend builds its menu from.
pub async fn tracks_summary(State(state): State<AppState>) -> ApiResult<Json<Vec<Value>>> {
	Ok(Json(track_counts(&state).await?))
}

pub async fn track_counts(state: &AppState) -> ApiResult<Vec<Value>> {
	let rows = state
		.db
		.query(
			"SELECT t.id::int AS id, t.name, count(r.index) FILTER (WHERE NOT r.is_final) AS active
			FROM governance_tracks t LEFT JOIN governance_referendums r ON r.track::text = t.id
			GROUP BY t.id, t.name ORDER BY t.id::int",
			&[],
		)
		.await?;
	Ok(rows
		.iter()
		.map(|row| {
			json!({
				"id": row.get::<_, i32>("id"),
				"name": row.get::<_, String>("name"),
				"activeCount": row.get::<_, i64>("active"),
			})
		})
		.collect())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn names_event_args_as_the_timeline_expects() {
		let hash = json!("0xabc");
		let submitted =
			timeline_args("Submitted", json!({ "index": 80, "track": 1 }), "alice", &hash);
		assert_eq!(
			submitted,
			json!({ "referendumIndex": 80, "track": 1, "proposer": "alice", "proposalHash": "0xabc" })
		);
		let deposit = timeline_args(
			"DecisionDepositPlaced",
			json!({ "index": 80, "amount": "0x0a" }),
			"alice",
			&hash,
		);
		assert_eq!(deposit, json!({ "amount": "10" }));
	}

	#[test]
	fn describes_tracks_with_their_origin() {
		let info =
			json!({ "name": "whitelisted_caller\0\0", "decisionDeposit": "0x0a" }).to_string();
		let track = track(1, Some(info));
		assert_eq!(track["name"], "whitelisted_caller");
		assert_eq!(track["origin"], "WhitelistedCaller");
		assert_eq!(track["decisionDeposit"], "10");
	}
}
