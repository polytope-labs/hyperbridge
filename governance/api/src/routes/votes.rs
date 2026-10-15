use std::collections::HashMap;

use axum::{
	extract::{Path, State},
	Json,
};
use serde_json::{json, Value};
use tokio_postgres::{types::ToSql, Row};

use super::{
	common::{decimal, parse},
	AppState,
};
use crate::{
	error::{ApiError, ApiResult},
	primitives::Indexer,
};

// Values the frontend reads from the activity log of a referendum
const VOTE: i32 = 1;
const REMOVE_VOTE: i32 = 2;

fn amount(row: &Row, column: &str) -> u128 {
	row.get::<_, Option<String>>(column)
		.and_then(|value| value.parse().ok())
		.unwrap_or_default()
}

/// Votes a balance carries with its conviction: a tenth without a lock, then one to six times.
pub fn with_conviction(balance: u128, conviction: i32) -> u128 {
	match conviction {
		0 => balance / 10,
		n => balance * n as u128,
	}
}

/// A vote as the chain encodes it, in the shape SubSquare gives the frontend. A standard vote packs
/// its direction into the top bit of a byte and its conviction into the rest.
fn account_vote(vote: &Value) -> Value {
	let flags = |kind: &str| json!({ "isStandard": kind == "standard", "isSplit": kind == "split", "isSplitAbstain": kind == "splitAbstain" });
	if let Some(standard) = vote.get("standard") {
		let byte = standard["vote"]
			.as_str()
			.and_then(|hex| u8::from_str_radix(hex.trim_start_matches("0x"), 16).ok())
			.unwrap_or_default();
		let mut shaped = flags("standard");
		shaped["vote"] = json!({
			"balance": decimal(&standard["balance"]),
			"vote": { "isAye": byte & 0x80 != 0, "conviction": byte & 0x7f },
		});
		return shaped;
	}
	let (kind, fields) = match (vote.get("split"), vote.get("splitAbstain")) {
		(Some(split), _) => ("split", split),
		(_, Some(split)) => ("splitAbstain", split),
		_ => return Value::Null,
	};
	let mut shaped = flags(kind);
	shaped["vote"] = fields
		.as_object()
		.map(|fields| fields.iter().map(|(side, value)| (side.clone(), decimal(value))).collect())
		.unwrap_or_default();
	shaped
}

/// The referendum's track and the block its votes are read at: the end of decision for a finished
/// referendum, the latest indexed block otherwise.
async fn vote_context(state: &AppState, index: i32) -> ApiResult<(i32, i32)> {
	let row = state
		.db
		.query_opt(
			"SELECT r.track, CASE WHEN r.is_final THEN
				(SELECT max(block_number) FROM governance_referendum_events WHERE referendum_id = r.id AND name <> 'Executed')
				ELSE (SELECT (value #>> '{}')::int FROM governance._metadata WHERE key = 'lastProcessedHeight') END
			FROM governance_referendums r WHERE r.index = $1",
			&[&index],
		)
		.await?
		.ok_or_else(|| ApiError::not_found("Referendum"))?;
	Ok((row.get(0), row.get::<_, Option<i32>>(1).unwrap_or_default()))
}

/// Every vote counted on a referendum, direct votes and the delegations they carry.
async fn referendum_votes(state: &AppState, index: i32) -> ApiResult<Vec<Value>> {
	let (track, query_at) = vote_context(state, index).await?;
	let direct = state
		.db
		.query(
			"SELECT account, kind, aye, conviction, balance::text, aye_balance::text, nay_balance::text,
				abstain_balance::text, delegated_votes::text, delegated_capital::text
			FROM governance_votes WHERE referendum_index = $1 ORDER BY balance DESC",
			&[&index],
		)
		.await?;
	let delegated = state
		.db
		.query(
			"SELECT d.delegator, d.target, d.conviction, d.balance::text, v.aye
			FROM governance_delegations d
			JOIN governance_votes v ON v.account = d.target AND v.referendum_index = $1 AND v.kind = 'Standard'
			WHERE d.track = $2 AND d.from_block <= $3 AND (d.to_block IS NULL OR d.to_block > $3)",
			&[&index, &track, &query_at],
		)
		.await?;

	let direct = direct.iter().map(|row| {
		let kind = row.get::<_, String>("kind");
		let balance = amount(row, "balance");
		let conviction = row.get::<_, Option<i32>>("conviction").unwrap_or_default();
		json!({
			"referendumIndex": index,
			"account": row.get::<_, String>("account"),
			"isDelegating": false,
			"isStandard": kind == "Standard",
			"isSplit": kind == "Split",
			"isSplitAbstain": kind == "SplitAbstain",
			"balance": balance.to_string(),
			"ayeBalance": row.get::<_, Option<String>>("aye_balance"),
			"nayBalance": row.get::<_, Option<String>>("nay_balance"),
			"abstainBalance": row.get::<_, Option<String>>("abstain_balance"),
			"aye": row.get::<_, Option<bool>>("aye"),
			"conviction": conviction,
			"votes": with_conviction(balance, conviction).to_string(),
			"delegations": { "votes": amount(row, "delegated_votes").to_string(), "capital": amount(row, "delegated_capital").to_string() },
			"queryAt": query_at,
		})
	});
	let delegated = delegated.iter().map(|row| {
		let balance = amount(row, "balance");
		let conviction = row.get::<_, i32>("conviction");
		json!({
			"referendumIndex": index,
			"account": row.get::<_, String>("delegator"),
			"target": row.get::<_, String>("target"),
			"isDelegating": true,
			"isStandard": true,
			"isSplit": false,
			"isSplitAbstain": false,
			"balance": balance.to_string(),
			"aye": row.get::<_, bool>("aye"),
			"conviction": conviction,
			"votes": with_conviction(balance, conviction).to_string(),
			"queryAt": query_at,
		})
	});
	Ok(direct.chain(delegated).collect())
}

pub async fn votes(
	State(state): State<AppState>,
	Path(index): Path<i32>,
) -> ApiResult<Json<Vec<Value>>> {
	Ok(Json(referendum_votes(&state, index).await?))
}

pub async fn vote_stats(
	State(state): State<AppState>,
	Path(index): Path<i32>,
) -> ApiResult<Json<Value>> {
	Ok(Json(referendum_stats(&state, index).await?))
}

/// A referendum's turnout: its tally against the electorate and the capital behind it, split into
/// direct votes and delegations.
pub async fn referendum_stats(state: &AppState, index: i32) -> ApiResult<Value> {
	let tally = state
		.db
		.query_opt(
			"SELECT track, ayes::text, nays::text, support::text, electorate::text FROM governance_referendums WHERE index = $1",
			&[&index],
		)
		.await?
		.ok_or_else(|| ApiError::not_found("Referendum"))?;
	let votes = referendum_votes(state, index).await?;
	let sum = |delegating: bool, field: &str| -> u128 {
		votes
			.iter()
			.filter(|vote| vote["isDelegating"] == delegating)
			.filter_map(|vote| vote[field].as_str()?.parse::<u128>().ok())
			.sum()
	};
	let count =
		|delegating: bool| votes.iter().filter(|vote| vote["isDelegating"] == delegating).count();
	let (support, electorate) = (amount(&tally, "support"), amount(&tally, "electorate"));
	let direct_capital = sum(false, "balance");
	let delegation_capital = sum(true, "balance");
	let total_capital = direct_capital + delegation_capital;

	Ok(json!({
		"referendumIndex": index,
		"trackId": tally.get::<_, i32>("track"),
		"support": support.to_string(),
		"electorate": electorate.to_string(),
		"ayes": amount(&tally, "ayes").to_string(),
		"nays": amount(&tally, "nays").to_string(),
		"votes": (amount(&tally, "ayes") + amount(&tally, "nays")).to_string(),
		"directCapital": direct_capital.to_string(),
		"delegationCapital": delegation_capital.to_string(),
		"totalCapital": total_capital.to_string(),
		"delegationVotes": sum(true, "votes").to_string(),
		"directAddresses": count(false),
		"delegationAddresses": count(true),
		"percentage": if electorate == 0 { 0.0 } else { total_capital as f64 / electorate as f64 },
	}))
}

fn call_indexer(row: &Row) -> Indexer {
	Indexer {
		block_height: row.get("block_number"),
		block_hash: row.get("block_hash"),
		block_time: row.get("ms"),
		extrinsic_index: row.get("extrinsic_index"),
		event_index: row.get("event_index"),
	}
}

async fn vote_call_rows(
	state: &AppState,
	filter: &str,
	value: &(dyn ToSql + Sync),
) -> ApiResult<Vec<Row>> {
	Ok(state
		.db
		.query(
			&format!(
				"SELECT id, account, referendum_index, vote, removed, block_number, block_hash, extrinsic_index,
					event_index, ms(timestamp) AS ms
				FROM governance_vote_calls WHERE {filter} ORDER BY block_number, event_index"
			),
			&[value],
		)
		.await?)
}

fn vote_call(row: &Row) -> Value {
	let mut call = account_vote(&parse(row.get("vote")));
	call["_id"] = row.get::<_, String>("id").into();
	call["referendumIndex"] = row.get::<_, i32>("referendum_index").into();
	call["voter"] = row.get::<_, String>("account").into();
	call["indexer"] = json!(call_indexer(row));
	call
}

/// The votes cast on a referendum as they happened, without removals.
pub async fn vote_calls(
	State(state): State<AppState>,
	Path(index): Path<i32>,
) -> ApiResult<Json<Vec<Value>>> {
	let rows = vote_call_rows(&state, "referendum_index = $1 AND NOT removed", &index).await?;
	Ok(Json(rows.iter().map(vote_call).collect()))
}

/// Every vote and removal on a referendum with the vote it replaced, for the activity log.
pub async fn actions(
	State(state): State<AppState>,
	Path(index): Path<i32>,
) -> ApiResult<Json<Vec<Value>>> {
	let rows = vote_call_rows(&state, "referendum_index = $1", &index).await?;
	let delegations = state
		.db
		.query(
			"SELECT account, delegated_votes::text, delegated_capital::text FROM governance_votes WHERE referendum_index = $1",
			&[&index],
		)
		.await?
		.iter()
		.map(|row| {
			let value = json!({ "votes": amount(row, "delegated_votes").to_string(), "capital": amount(row, "delegated_capital").to_string() });
			(row.get::<_, String>(0), value)
		})
		.collect::<HashMap<_, _>>();
	let none = json!({ "votes": "0", "capital": "0" });

	let mut previous = HashMap::<String, Value>::new();
	let actions = rows
		.iter()
		.map(|row| {
			let who = row.get::<_, String>("account");
			let removed = row.get::<_, bool>("removed");
			let delegated = delegations.get(&who).unwrap_or(&none).clone();
			let before = previous.remove(&who);
			let data = if removed {
				json!({ "vote": before, "delegations": delegated })
			} else {
				let vote = account_vote(&parse(row.get("vote")));
				previous.insert(who.clone(), vote.clone());
				let mut data = json!({ "vote": vote, "delegations": delegated });
				if let Some(before) = before {
					data["preVote"] = before;
					data["preDelegations"] = delegated;
				}
				data
			};
			json!({
				"who": who,
				"referendumIndex": index,
				"type": if removed { REMOVE_VOTE } else { VOTE },
				"data": data,
				"indexer": call_indexer(row),
			})
		})
		.collect();
	Ok(Json(actions))
}

/// The tally after each change, which the support and approval curves are drawn from.
pub async fn tally_history(
	State(state): State<AppState>,
	Path(index): Path<i32>,
) -> ApiResult<Json<Vec<Value>>> {
	let rows = state
		.db
		.query(
			"SELECT block_number, ms(timestamp), ayes::text, nays::text, support::text, electorate::text
			FROM governance_tally_snapshots WHERE referendum_id = $1 ORDER BY block_number",
			&[&index.to_string()],
		)
		.await?;
	Ok(Json(
		rows.iter()
			.map(|row| {
				json!({
					"referendumIndex": index,
					"indexer": { "blockHeight": row.get::<_, i32>(0), "blockTime": row.get::<_, i64>(1) },
					"tally": {
						"ayes": row.get::<_, String>(2),
						"nays": row.get::<_, String>(3),
						"support": row.get::<_, String>(4),
						"issuance": row.get::<_, String>(5),
					},
				})
			})
			.collect(),
	))
}
