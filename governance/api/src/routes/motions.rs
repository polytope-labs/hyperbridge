use axum::{
	extract::{Path, RawQuery, State},
	Json,
};
use serde_json::{json, Value};
use tokio_postgres::Row;

use super::{
	common::{indexer, item_index, parse, post_fields, post_joins, timeline, POST_COLUMNS},
	posts::post_reactions,
	AppState, Params,
};
use crate::{
	error::{ApiError, ApiResult},
	primitives::{post_id, Page, PostKind},
};

fn query(filter: &str) -> String {
	format!(
		"SELECT m.index, m.hash, m.proposer, m.threshold, m.call, m.ayes, m.nays, m.\"end\", m.state,
			m.is_final, m.result, m.proposed_block, {POST_COLUMNS}
		FROM governance_motions m
		{}
		WHERE {filter}",
		post_joins("governance_motion_events", "motion_id = m.id", PostKind::Motion, "m.index"),
	)
}

/// Event args under the names the forum's timeline uses.
fn timeline_args(method: &str, args: Value) -> Value {
	let Value::Object(fields) = args else { return args };
	let rename = |key: String| match key.as_str() {
		"account" if method == "Voted" => "voter".into(),
		"account" => "proposer".into(),
		"proposalHash" => "hash".into(),
		"proposalIndex" => "index".into(),
		"voted" => "approve".into(),
		"yes" => "yesVotes".into(),
		"no" => "noVotes".into(),
		"result" => "dispatchResult".into(),
		_ => key,
	};
	Value::Object(fields.into_iter().map(|(key, value)| (rename(key), value)).collect())
}

fn motion(row: &Row) -> Value {
	let index = row.get::<_, i32>("index");
	let is_final = row.get::<_, bool>("is_final");
	let proposer = row.get::<_, String>("proposer");
	let state = row.get::<_, String>("state");
	let ayes = row.get::<_, Value>("ayes");
	let nays = row.get::<_, Value>("nays");
	let count = |votes: &Value| votes.as_array().map(Vec::len).unwrap_or_default();
	// The frontend reads the final event's fields as they are, the motion hash and its dispatch
	// result
	let state_data = match parse(row.get("result")) {
		Value::Null => Value::Null,
		result => json!([row.get::<_, String>("hash"), result]),
	};

	let mut post = post_fields(row, PostKind::Motion, index.into(), &proposer);
	post.extend([
		("motionIndex".into(), index.into()),
		("hash".into(), row.get::<_, String>("hash").into()),
		("height".into(), row.get::<_, i32>("proposed_block").into()),
		("state".into(), state.clone().into()),
		("stateSort".into(), if is_final { 4 } else { 1 }.into()),
		("isFinal".into(), is_final.into()),
		(
			"onchainData".into(),
			json!({
				"index": index,
				"hash": row.get::<_, String>("hash"),
				"indexer": post["indexer"],
				"proposer": proposer,
				"threshold": row.get::<_, i32>("threshold"),
				"proposal": parse(row.get("call")),
				"voting": {
					"index": index,
					"threshold": row.get::<_, i32>("threshold"),
					"ayes": ayes,
					"nays": nays,
					"end": row.get::<_, Option<i32>>("end"),
				},
				"tally": { "yesVotes": count(&ayes), "noVotes": count(&nays) },
				"state": { "state": state, "data": state_data, "indexer": indexer(row, "l") },
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
	let filter = if params.flag("ongoing") { "NOT m.is_final" } else { "TRUE" };
	let rows = state
		.db
		.query(
			&format!(
				"{} ORDER BY m.is_final, m.updated_block DESC LIMIT {} OFFSET {}",
				query(filter),
				paging.page_size,
				paging.offset()
			),
			&[],
		)
		.await?;
	let total = state
		.db
		.query_one(&format!("SELECT count(*) FROM governance_motions m WHERE {filter}"), &[])
		.await?
		.get::<_, i64>(0);
	Ok(Json(paging.page(rows.iter().map(motion).collect(), total)))
}

/// A motion by its index or by the post id the frontend uses for comments and edits.
pub async fn detail(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
	let index = item_index(&id, PostKind::Motion, "Motion")?;
	let row = state
		.db
		.query_opt(&query("m.index = $1"), &[&index])
		.await?
		.ok_or_else(|| ApiError::not_found("Motion"))?;

	let timeline =
		timeline(&state.db, "governance_motion_events", "motion_id = $1", index.to_string())
			.await?
			.into_iter()
			.map(|(method, args, indexer)| {
				let args = timeline_args(&method, args);
				json!({ "type": "event", "method": method, "args": args, "indexer": indexer })
			})
			.collect::<Vec<_>>();

	let mut detail = motion(&row);
	detail["onchainData"]["timeline"] = timeline.into();
	detail["authors"] = json!([detail["proposer"]]);
	detail["reactions"] = post_reactions(&state, post_id(PostKind::Motion, index.into())).await?;
	Ok(Json(detail))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn names_event_args_as_the_timeline_expects() {
		let voted =
			json!({ "account": "bob", "proposalHash": "0xabc", "voted": true, "yes": 2, "no": 0 });
		assert_eq!(
			timeline_args("Voted", voted),
			json!({ "voter": "bob", "hash": "0xabc", "approve": true, "yesVotes": 2, "noVotes": 0 })
		);
		assert_eq!(
			timeline_args("Proposed", json!({ "account": "alice" })),
			json!({ "proposer": "alice" })
		);
	}
}
