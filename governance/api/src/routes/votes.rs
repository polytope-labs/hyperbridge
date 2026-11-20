use std::collections::HashMap;

use axum::{
	extract::{Path, RawQuery, State},
	Json,
};
use serde_json::{json, Value};
use tokio_postgres::{types::ToSql, Row};

use super::{
	common::{decimal, item_index, parse, valid_address, INDEXED_HEIGHT, LAST_EVENT_COLUMNS},
	referenda::{proposal_summary, summary_joins, STATE_EVENTS, SUMMARY_COLUMNS},
	AppState, Params,
};
use crate::{
	error::{ApiError, ApiResult},
	primitives::{Indexer, Page, PostKind},
};

// Values the frontend reads from the activity log of a referendum
const VOTE: i32 = 1;
const REMOVE_VOTE: i32 = 2;

fn amount(row: &Row, column: &str) -> u128 {
	row.get::<_, Option<String>>(column)
		.and_then(|value| value.parse().ok())
		.unwrap_or_default()
}

/// The share of referenda an account voted on.
pub fn participation(voted: i64, referenda: i64) -> f64 {
	if referenda == 0 {
		0.0
	} else {
		voted as f64 / referenda as f64
	}
}

/// A vote as the chain encodes it, in the shape the forum frontend reads. A standard vote packs
/// its direction into the top bit of a byte and its conviction into the rest.
fn account_vote(vote: &Value) -> Value {
	let flags = |kind: &str| json!({ "isStandard": kind == "standard", "isSplit": kind == "split", "isSplitAbstain": kind == "splitAbstain" });
	if let Some(standard) = vote.get("standard") {
		let vote = &standard["vote"];
		let byte = vote
			.as_u64()
			.map(|byte| byte as u8)
			.or_else(|| u8::from_str_radix(vote.as_str()?.trim_start_matches("0x"), 16).ok())
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

/// The referenda matching `filter`, each with the block its votes are read at: the end of decision
/// for a finished referendum, the latest indexed block otherwise.
fn referenda_at(filter: &str) -> String {
	format!(
		"SELECT r.index, r.track, r.is_final, r.ayes, r.nays, r.support, r.electorate,
			CASE WHEN r.is_final THEN (SELECT max(block_number) FROM governance_referendum_events
				WHERE {STATE_EVENTS} AND name <> 'Executed')
			ELSE {INDEXED_HEIGHT} END AS query_at
		FROM governance_referendums r WHERE {filter}"
	)
}

/// The delegations counted on each referendum of `q`, as `d`, with the direct vote `v` of the
/// account they delegate to. A finished referendum counts those in force at its query block. An
/// ongoing one counts those in force now, as conviction voting does, since the indexed height can
/// trail the delegations already written.
const DELEGATIONS_IN_FORCE: &str = "FROM q
	JOIN governance_votes v ON v.referendum_index = q.index AND v.kind = 'Standard'
	JOIN governance_delegations d ON d.target = v.account AND d.track = q.track AND CASE
		WHEN q.is_final THEN d.from_block <= q.query_at AND (d.to_block IS NULL OR d.to_block > q.query_at)
		ELSE d.active END";

/// The block a referendum's votes are read at, see [`referenda_at`].
async fn query_at(state: &AppState, index: i32) -> ApiResult<i32> {
	let row = state
		.db
		.get()
		.await?
		.query_opt(&format!("SELECT query_at FROM ({}) q", referenda_at("r.index = $1")), &[&index])
		.await?
		.ok_or_else(|| ApiError::not_found("Referendum"))?;
	Ok(row.get::<_, Option<i32>>(0).unwrap_or_default())
}

/// Every vote counted on a referendum, direct votes and the delegations they carry.
const VOTE_COLUMNS: &str = "v.referendum_index, v.account, v.kind, v.aye, v.conviction,
	v.balance::text AS balance, conviction_votes(v.balance, COALESCE(v.conviction, 0))::text AS votes,
	v.aye_balance::text AS aye_balance, v.nay_balance::text AS nay_balance,
	v.abstain_balance::text AS abstain_balance, v.delegated_votes::text AS delegated_votes,
	v.delegated_capital::text AS delegated_capital";

/// An account's own vote on a referendum, as read at `query_at`.
fn direct_vote(row: &Row, query_at: i32) -> Value {
	let kind = row.get::<_, String>("kind");
	json!({
		"referendumIndex": row.get::<_, i32>("referendum_index"),
		"account": row.get::<_, String>("account"),
		"isDelegating": false,
		"isStandard": kind == "Standard",
		"isSplit": kind == "Split",
		"isSplitAbstain": kind == "SplitAbstain",
		"balance": amount(row, "balance").to_string(),
		"ayeBalance": row.get::<_, Option<String>>("aye_balance"),
		"nayBalance": row.get::<_, Option<String>>("nay_balance"),
		"abstainBalance": row.get::<_, Option<String>>("abstain_balance"),
		"aye": row.get::<_, Option<bool>>("aye"),
		"conviction": row.get::<_, Option<i32>>("conviction").unwrap_or_default(),
		"votes": amount(row, "votes").to_string(),
		"delegations": {
			"votes": amount(row, "delegated_votes").to_string(),
			"capital": amount(row, "delegated_capital").to_string(),
		},
		"queryAt": query_at,
	})
}

/// The delegations counted on a referendum, to every voter or only to `target`, largest first.
async fn delegated_votes(
	state: &AppState,
	index: i32,
	target: Option<&str>,
) -> ApiResult<Vec<Value>> {
	let rows = state
		.db
		.get()
		.await?
		.query(
			&format!(
				"WITH q AS ({}) SELECT d.delegator, d.target, d.conviction, d.balance::text AS balance,
					conviction_votes(d.balance, d.conviction)::text AS votes, v.aye, q.query_at
				{DELEGATIONS_IN_FORCE}
				WHERE $2::text IS NULL OR d.target = $2 ORDER BY d.balance DESC, d.delegator",
				referenda_at("r.index = $1")
			),
			&[&index, &target],
		)
		.await?;
	Ok(rows
		.iter()
		.map(|row| {
			json!({
				"referendumIndex": index,
				"account": row.get::<_, String>("delegator"),
				"target": row.get::<_, String>("target"),
				"isDelegating": true,
				"isStandard": true,
				"isSplit": false,
				"isSplitAbstain": false,
				"balance": amount(row, "balance").to_string(),
				"aye": row.get::<_, bool>("aye"),
				"conviction": row.get::<_, i32>("conviction"),
				"votes": amount(row, "votes").to_string(),
				"queryAt": row.get::<_, Option<i32>>("query_at").unwrap_or_default(),
			})
		})
		.collect())
}

pub async fn votes(
	State(state): State<AppState>,
	Path(index): Path<i32>,
) -> ApiResult<Json<Vec<Value>>> {
	let query_at = query_at(&state, index).await?;
	let direct = state
		.db
		.get()
		.await?
		.query(
			&format!(
				"SELECT {VOTE_COLUMNS} FROM governance_votes v WHERE v.referendum_index = $1
				ORDER BY v.balance DESC, v.account"
			),
			&[&index],
		)
		.await?;
	let delegated = delegated_votes(&state, index, None).await?;
	Ok(Json(direct.iter().map(|row| direct_vote(row, query_at)).chain(delegated).collect()))
}

/// The delegations counted on a referendum for the voter named by `target`, which the vote detail
/// popup lists.
pub async fn delegation_votes(
	State(state): State<AppState>,
	Path(id): Path<String>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Vec<Value>>> {
	let index = item_index(&id, PostKind::Referendum, "Referendum")?;
	let target = Params::new(query).get("target").map(valid_address).transpose()?;
	Ok(Json(delegated_votes(&state, index, target.as_deref()).await?))
}

/// Turnout for every referendum matching `filter`, oldest first: its tally against the electorate
/// and the capital behind it, split into direct votes and delegations.
pub async fn turnouts(
	state: &AppState,
	filter: &str,
	params: &[&(dyn ToSql + Sync)],
) -> ApiResult<Vec<Value>> {
	let rows = state
		.db
		.get()
		.await?
		.query(
			&format!(
				"WITH q AS ({}),
				direct AS (SELECT v.referendum_index AS index, sum(v.balance) AS capital, count(*) AS addresses
					FROM q JOIN governance_votes v ON v.referendum_index = q.index GROUP BY 1),
				delegated AS (SELECT q.index, sum(d.balance) AS capital,
					sum(conviction_votes(d.balance, d.conviction)) AS votes, count(*) AS addresses
					{DELEGATIONS_IN_FORCE} GROUP BY q.index)
				SELECT q.index, q.track, q.ayes::text AS ayes, q.nays::text AS nays, q.support::text AS support,
					q.electorate::text AS electorate, COALESCE(di.capital, 0)::text AS direct_capital,
					COALESCE(di.addresses, 0) AS direct_addresses,
					COALESCE(de.capital, 0)::text AS delegation_capital,
					COALESCE(de.votes, 0)::text AS delegation_votes,
					COALESCE(de.addresses, 0) AS delegation_addresses
				FROM q LEFT JOIN direct di ON di.index = q.index LEFT JOIN delegated de ON de.index = q.index
				ORDER BY q.index",
				referenda_at(filter)
			),
			params,
		)
		.await?;
	Ok(rows
		.iter()
		.map(|row| {
			let (ayes, nays) = (amount(row, "ayes"), amount(row, "nays"));
			let electorate = amount(row, "electorate");
			let direct_capital = amount(row, "direct_capital");
			let delegation_capital = amount(row, "delegation_capital");
			let total_capital = direct_capital + delegation_capital;
			json!({
				"referendumIndex": row.get::<_, i32>("index"),
				"trackId": row.get::<_, i32>("track"),
				"support": amount(row, "support").to_string(),
				"electorate": electorate.to_string(),
				"ayes": ayes.to_string(),
				"nays": nays.to_string(),
				"votes": (ayes + nays).to_string(),
				"directCapital": direct_capital.to_string(),
				"delegationCapital": delegation_capital.to_string(),
				"totalCapital": total_capital.to_string(),
				"delegationVotes": amount(row, "delegation_votes").to_string(),
				"directAddresses": row.get::<_, i64>("direct_addresses"),
				"delegationAddresses": row.get::<_, i64>("delegation_addresses"),
				"percentage": if electorate == 0 { 0.0 } else { total_capital as f64 / electorate as f64 },
			})
		})
		.collect())
}

pub async fn vote_stats(
	State(state): State<AppState>,
	Path(index): Path<i32>,
) -> ApiResult<Json<Value>> {
	turnouts(&state, "r.index = $1", &[&index])
		.await?
		.into_iter()
		.next()
		.map(Json)
		.ok_or_else(|| ApiError::not_found("Referendum"))
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
		.get()
		.await?
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
		.get()
		.await?
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
		.get()
		.await?
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

/// The vote call columns [`vote_call`] reads, from vote calls aliased `c`.
const VOTE_CALL_COLUMNS: &str = "c.id, c.account, c.referendum_index, c.vote, c.removed,
	c.block_number, c.block_hash, c.extrinsic_index, c.event_index, ms(c.timestamp) AS ms";

/// An account's votes, newest referendum first.
pub async fn user_votes(
	State(state): State<AppState>,
	Path(account): Path<String>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	let account = valid_address(&account)?;
	let params = Params::new(query);
	let paging = params.paging(25);
	let rows = state
		.db
		.get()
		.await?
		.query(
			&format!(
				"SELECT page.*, q.query_at, {SUMMARY_COLUMNS}, {LAST_EVENT_COLUMNS}
				FROM (SELECT {VOTE_COLUMNS} FROM governance_votes v WHERE v.account = $1
					ORDER BY v.referendum_index DESC LIMIT {} OFFSET {}) page
				LEFT JOIN LATERAL ({}) q ON TRUE
				{}
				ORDER BY page.referendum_index DESC",
				paging.page_size,
				paging.offset(),
				referenda_at("r.index = page.referendum_index"),
				summary_joins("page.referendum_index"),
			),
			&[&account],
		)
		.await?;
	let total = state
		.db
		.get()
		.await?
		.query_one("SELECT count(*) FROM governance_votes WHERE account = $1", &[&account])
		.await?
		.get(0);
	let items = rows
		.iter()
		.map(|row| {
			let mut vote =
				direct_vote(row, row.get::<_, Option<i32>>("query_at").unwrap_or_default());
			if params.flag("includes_title") {
				vote["proposal"] = proposal_summary(row);
			}
			vote
		})
		.collect();
	Ok(Json(paging.page(items, total)))
}

/// An account's vote calls, newest first.
pub async fn user_vote_calls(
	State(state): State<AppState>,
	Path(account): Path<String>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	let account = valid_address(&account)?;
	let params = Params::new(query);
	let paging = params.paging(25);
	let rows = state
		.db
		.get()
		.await?
		.query(
			&format!(
				"SELECT {VOTE_CALL_COLUMNS}, {SUMMARY_COLUMNS}, {LAST_EVENT_COLUMNS}
				FROM (SELECT * FROM governance_vote_calls WHERE account = $1 AND NOT removed
					ORDER BY block_number DESC, event_index DESC, id DESC LIMIT {} OFFSET {}) c
				{}
				ORDER BY c.block_number DESC, c.event_index DESC, c.id DESC",
				paging.page_size,
				paging.offset(),
				summary_joins("c.referendum_index"),
			),
			&[&account],
		)
		.await?;
	let total = state
		.db
		.get()
		.await?
		.query_one(
			"SELECT count(*) FROM governance_vote_calls WHERE account = $1 AND NOT removed",
			&[&account],
		)
		.await?
		.get(0);
	let items = rows
		.iter()
		.map(|row| {
			let mut call = vote_call(row);
			if params.flag("includes_title") {
				call["proposal"] = proposal_summary(row);
			}
			call
		})
		.collect();
	Ok(Json(paging.page(items, total)))
}

/// How much an account takes part: votes held, votes cast and the share of referenda it voted on.
pub async fn user_vote_stats(
	State(state): State<AppState>,
	Path(account): Path<String>,
) -> ApiResult<Json<Value>> {
	let account = valid_address(&account)?;
	let row = state
		.db
		.get()
		.await?
		.query_one(
			"SELECT (SELECT count(*) FROM governance_votes WHERE account = $1),
				(SELECT count(*) FROM governance_vote_calls WHERE account = $1 AND NOT removed),
				(SELECT count(DISTINCT referendum_index) FROM governance_vote_calls WHERE account = $1),
				(SELECT count(*) FROM governance_referendums)",
			&[&account],
		)
		.await?;
	Ok(Json(json!({
		"totalVotes": row.get::<_, i64>(0),
		"totalCalls": row.get::<_, i64>(1),
		"participationRate": participation(row.get(2), row.get(3)),
	})))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn decodes_the_packed_vote_byte() {
		let aye = account_vote(&json!({ "standard": { "vote": "0x83", "balance": "0x0a" } }));
		assert_eq!(
			aye["vote"],
			json!({ "balance": "10", "vote": { "isAye": true, "conviction": 3 } })
		);
		let nay = account_vote(&json!({ "standard": { "vote": "0x01", "balance": 5 } }));
		assert_eq!(nay["vote"]["vote"], json!({ "isAye": false, "conviction": 1 }));
		let numbered = account_vote(&json!({ "standard": { "vote": 0x83, "balance": "10" } }));
		assert_eq!(numbered["vote"], aye["vote"]);
		let split = account_vote(&json!({ "split": { "aye": 3, "nay": 4 } }));
		assert_eq!(
			(split["isSplit"].clone(), split["vote"].clone()),
			(json!(true), json!({ "aye": "3", "nay": "4" }))
		);
	}
}
