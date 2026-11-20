use std::collections::{BTreeMap, BTreeSet};

use axum::{
	extract::{Path, RawQuery, State},
	Json,
};
use serde_json::{json, Value};

use super::{
	auth::Session,
	common::valid_address,
	signed::{Signed, SignedEntity},
	votes::{participation, turnouts},
	AppState, Params,
};
use crate::{
	error::{ApiError, ApiResult},
	primitives::{Page, Paging},
};

/// A delegation that is in force now.
struct Delegation {
	delegator: String,
	target: String,
	track: i32,
	balance: u128,
	votes: u128,
}

async fn delegations(state: &AppState, track: Option<i32>) -> ApiResult<Vec<Delegation>> {
	let rows = state
		.db
		.get()
		.await?
		.query(
			"SELECT delegator, target, track, balance::text, conviction_votes(balance, conviction)::text
			FROM governance_delegations WHERE active AND ($1::int IS NULL OR track = $1)
			ORDER BY delegator, track",
			&[&track],
		)
		.await?;
	Ok(rows
		.iter()
		.map(|row| Delegation {
			delegator: row.get(0),
			target: row.get(1),
			track: row.get(2),
			balance: row.get::<_, String>(3).parse().unwrap_or_default(),
			votes: row.get::<_, String>(4).parse().unwrap_or_default(),
		})
		.collect())
}

/// How many accounts delegate and are delegated to, and the capital and votes involved.
fn delegation_summary<'a>(delegations: impl Iterator<Item = &'a Delegation> + Clone) -> Value {
	let delegators = delegations.clone().map(|d| &d.delegator).collect::<BTreeSet<_>>().len();
	let delegatees = delegations.clone().map(|d| &d.target).collect::<BTreeSet<_>>().len();
	let (capital, votes) = delegations
		.fold((0u128, 0u128), |(capital, votes), d| (capital + d.balance, votes + d.votes));
	json!({
		"addresses": { "delegator": delegators, "delegatee": delegatees },
		"votes": { "capital": capital.to_string(), "votes": votes.to_string() },
	})
}

async fn track_names(state: &AppState) -> ApiResult<Vec<(i32, String)>> {
	let rows = state
		.db
		.get()
		.await?
		.query("SELECT id::int, name FROM governance_tracks ORDER BY id::int", &[])
		.await?;
	Ok(rows.iter().map(|row| (row.get(0), row.get(1))).collect())
}

/// Every referendum of a track with its turnout, oldest first.
pub async fn track_turnout(
	State(state): State<AppState>,
	Path(track): Path<i32>,
) -> ApiResult<Json<Vec<Value>>> {
	Ok(Json(turnouts(&state, "r.track = $1", &[&track]).await?))
}

pub async fn summary(State(state): State<AppState>) -> ApiResult<Json<Value>> {
	let tracks = track_names(&state).await?;
	let by_track = turnouts(&state, "TRUE", &[]).await?.into_iter().fold(
		BTreeMap::<i64, Vec<Value>>::new(),
		|mut by_track, turnout| {
			by_track
				.entry(turnout["trackId"].as_i64().unwrap_or_default())
				.or_default()
				.push(turnout);
			by_track
		},
	);
	let count_of = |id: i32| by_track.get(&id.into()).map(Vec::len).unwrap_or_default();
	let total = by_track.values().map(Vec::len).sum::<usize>();

	// A track's turnout is the average over its referenda of their capital, electorate and turnout,
	// rounded to the nearest unit as the forum shows it
	let track_turnouts = tracks
		.iter()
		.map(|(id, name)| {
			let turnouts = by_track.get(&(*id).into()).map(Vec::as_slice).unwrap_or_default();
			let count = turnouts.len().max(1) as u128;
			let average = |field: &str| {
				turnouts
					.iter()
					.filter_map(|t| t[field].as_str()?.parse::<u128>().ok())
					.sum::<u128>()
					.saturating_add(count / 2) /
					count
			};
			let percentage = turnouts.iter().filter_map(|t| t["percentage"].as_f64()).sum::<f64>() /
				count as f64;
			json!({
				"id": id,
				"name": name,
				"capital": average("totalCapital").to_string(),
				"electorate": average("electorate").to_string(),
				"percentage": percentage,
			})
		})
		.collect::<Vec<_>>();

	// Busiest tracks first, ties in track order
	let mut track_counts =
		tracks.iter().map(|(id, name)| (count_of(*id), *id, name)).collect::<Vec<_>>();
	track_counts.sort_by_key(|(count, ..)| std::cmp::Reverse(*count));

	let delegations = delegations(&state, None).await?;
	let delegation = delegation_summary(delegations.iter());
	Ok(Json(json!({
		"trackTurnouts": track_turnouts,
		"trackReferendaCounts": track_counts
			.into_iter()
			.map(|(count, id, name)| {
				let percent = if total == 0 { 0.0 } else { count as f64 / total as f64 };
				json!({ "id": id, "name": name, "count": count, "percent": percent })
			})
			.collect::<Vec<_>>(),
		"totalReferendaCount": total,
		"delegation": {
			"delegator": delegation["addresses"]["delegator"],
			"delegatee": delegation["addresses"]["delegatee"],
			"capital": delegation["votes"]["capital"],
		},
	})))
}

pub async fn tracks(State(state): State<AppState>) -> ApiResult<Json<Vec<Value>>> {
	let delegations = delegations(&state, None).await?;
	let tracks = track_names(&state).await?;
	Ok(Json(
		tracks
			.into_iter()
			.map(|(id, name)| {
				let on_track = delegations.iter().filter(|d| d.track == id);
				json!({ "trackId": id, "trackName": name, "statistics": delegation_summary(on_track) })
			})
			.collect(),
	))
}

pub async fn track_summary(
	State(state): State<AppState>,
	Path(track): Path<i32>,
) -> ApiResult<Json<Value>> {
	Ok(Json(delegation_summary(delegations(&state, Some(track)).await?.iter())))
}

/// Accounts others delegate to, with how many delegate to each and with what weight.
async fn delegatees(
	state: &AppState,
	track: Option<i32>,
	query: Option<String>,
) -> ApiResult<Page<Value>> {
	let paging = Params::new(query).paging(25);
	let by_target = delegations(state, track).await?.into_iter().fold(
		BTreeMap::<String, (BTreeSet<String>, u128, u128)>::new(),
		|mut by_target, d| {
			let entry = by_target.entry(d.target).or_default();
			entry.0.insert(d.delegator);
			entry.1 += d.balance;
			entry.2 += d.votes;
			by_target
		},
	);
	let mut items = by_target
		.into_iter()
		.map(|(account, (delegators, capital, votes))| {
			json!({
				"account": account,
				"delegatorsCount": delegators.len(),
				"capital": capital.to_string(),
				"votes": votes.to_string(),
			})
		})
		.collect::<Vec<_>>();
	items.sort_by_key(|item| {
		std::cmp::Reverse(item["votes"].as_str().and_then(|v| v.parse::<u128>().ok()))
	});
	let total = items.len() as i64;
	let page = items
		.into_iter()
		.skip(paging.offset() as usize)
		.take(paging.page_size as usize)
		.collect();
	Ok(paging.page(page, total))
}

pub async fn all_delegatees(
	State(state): State<AppState>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	Ok(Json(delegatees(&state, None, query).await?))
}

pub async fn track_delegatees(
	State(state): State<AppState>,
	Path(track): Path<i32>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	Ok(Json(delegatees(&state, Some(track), query).await?))
}

/// The delegations in force on a track, to an account, or both, ordered by the column the
/// frontend's `sort` names as `[column, direction]`, largest votes first by default. Ties go in
/// delegator and track order, so pages never overlap. Without `paging` every delegation is
/// returned. Returns the delegations and how many there are in all.
async fn delegators(
	state: &AppState,
	track: Option<i32>,
	target: Option<&str>,
	sort: Option<&str>,
	paging: Option<Paging>,
) -> ApiResult<(Vec<Value>, i64)> {
	let (column, direction) = sort
		.and_then(|sort| serde_json::from_str::<(String, String)>(sort).ok())
		.unwrap_or_default();
	let column = match column.as_str() {
		"balance" => "d.balance",
		"trackId" => "d.track",
		"account" => "d.delegator",
		_ => "conviction_votes(d.balance, d.conviction)",
	};
	let direction = if direction == "asc" { "ASC" } else { "DESC" };
	let limit = paging
		.map(|paging| format!("LIMIT {} OFFSET {}", paging.page_size, paging.offset()))
		.unwrap_or_default();
	let rows = state
		.db
		.get()
		.await?
		.query(
			&format!(
				"SELECT d.delegator, d.target, d.track, t.name AS track_name, d.conviction,
					d.balance::text AS balance, conviction_votes(d.balance, d.conviction)::text AS votes,
					count(*) OVER () AS total
				FROM governance_delegations d LEFT JOIN governance_tracks t ON t.id = d.track::text
				WHERE d.active AND ($1::int IS NULL OR d.track = $1) AND ($2::text IS NULL OR d.target = $2)
				ORDER BY {column} {direction}, d.delegator, d.track {limit}"
			),
			&[&track, &target],
		)
		.await?;
	let total = rows.first().map(|row| row.get::<_, i64>("total")).unwrap_or_default();
	let items = rows
		.iter()
		.map(|row| {
			json!({
				"account": row.get::<_, String>("delegator"),
				"delegatee": row.get::<_, String>("target"),
				"trackId": row.get::<_, i32>("track"),
				"trackName": row.get::<_, Option<String>>("track_name"),
				"conviction": row.get::<_, i32>("conviction"),
				"balance": row.get::<_, String>("balance"),
				"votes": row.get::<_, String>("votes"),
			})
		})
		.collect();
	Ok((items, total))
}

/// A page of delegations, from the query string's paging and sort.
async fn delegator_page(
	state: &AppState,
	track: Option<i32>,
	target: Option<&str>,
	query: Option<String>,
) -> ApiResult<Json<Page<Value>>> {
	let params = Params::new(query);
	let paging = params.paging(25);
	let (items, total) = delegators(state, track, target, params.get("sort"), Some(paging)).await?;
	Ok(Json(paging.page(items, total)))
}

pub async fn track_delegators(
	State(state): State<AppState>,
	Path(track): Path<i32>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	delegator_page(&state, Some(track), None, query).await
}

/// The delegations to one delegatee, which the statistics pages drill down into.
pub async fn delegatee_delegators(
	State(state): State<AppState>,
	Path(address): Path<String>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	delegator_page(&state, None, Some(&valid_address(&address)?), query).await
}

pub async fn track_delegatee_delegators(
	State(state): State<AppState>,
	Path((track, address)): Path<(i32, String)>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	delegator_page(&state, Some(track), Some(&valid_address(&address)?), query).await
}

/// Every delegation in force to an account, on any track, largest first.
pub async fn user_delegators(
	State(state): State<AppState>,
	Path(address): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
	let address = valid_address(&address)?;
	Ok(Json(delegators(&state, None, Some(&address), None, None).await?.0))
}

/// The accounts delegated to now, with how many delegate to each and on how many tracks.
const DELEGATED_TO: &str = "d AS (SELECT target, count(DISTINCT delegator) AS delegators,
	count(DISTINCT track) AS tracks FROM governance_delegations WHERE active GROUP BY target)";

/// What the delegation pages show about the accounts `addresses` selects as `address`, most
/// delegated first, with `limit` applied before the per account counts are read.
fn delegates_query(addresses: &str, limit: &str) -> String {
	format!(
		"WITH {DELEGATED_TO}, page AS (
			SELECT a.address, COALESCE(d.delegators, 0) AS delegators, COALESCE(d.tracks, 0) AS tracks,
				count(*) OVER () AS total
			FROM ({addresses}) a LEFT JOIN d ON d.target = a.address
			ORDER BY delegators DESC, a.address {limit})
		SELECT page.*, m.short_description, m.long_description,
			(SELECT count(DISTINCT referendum_index) FROM governance_vote_calls WHERE account = page.address)
				AS voted,
			(SELECT count(*) FROM governance_referendums) AS referenda
		FROM page LEFT JOIN delegate_announcements m ON m.address = page.address
		ORDER BY page.delegators DESC, page.address"
	)
}

fn delegate_summary(row: &tokio_postgres::Row) -> Value {
	let manifesto = row.get::<_, Option<String>>("short_description").map(|short| {
		json!({
			"shortDescription": short,
			"longDescription": row.get::<_, Option<String>>("long_description"),
		})
	});
	json!({
		"address": row.get::<_, String>("address"),
		"delegatorsCount": row.get::<_, i64>("delegators"),
		"maxTrackDelegations": row.get::<_, i64>("tracks"),
		"participationRate": participation(row.get("voted"), row.get("referenda")),
		"manifesto": manifesto,
	})
}

pub async fn delegate(
	State(state): State<AppState>,
	Path(address): Path<String>,
) -> ApiResult<Json<Value>> {
	let address = valid_address(&address)?;
	let row = state
		.db
		.get()
		.await?
		.query_one(&delegates_query("SELECT $1::text AS address", ""), &[&address])
		.await?;
	Ok(Json(delegate_summary(&row)))
}

/// Accounts that announced themselves as delegates or already have delegations, most delegated
/// first.
pub async fn delegates(
	State(state): State<AppState>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	let paging = Params::new(query).paging(18);
	let rows = state
		.db
		.get()
		.await?
		.query(
			&delegates_query(
				"SELECT address FROM delegate_announcements UNION SELECT target FROM d",
				&format!("LIMIT {} OFFSET {}", paging.page_size, paging.offset()),
			),
			&[],
		)
		.await?;
	let total = rows.first().map(|row| row.get::<_, i64>("total")).unwrap_or_default();
	Ok(Json(paging.page(rows.iter().map(delegate_summary).collect(), total)))
}

/// Publishes the signing account's delegate announcement. Only accounts the indexer has seen vote
/// or be delegated to may announce, so the list holds no accounts made up for it.
pub async fn announce(
	State(state): State<AppState>,
	Json(signed): Json<SignedEntity>,
) -> ApiResult<Json<bool>> {
	let Signed { entity, address } = signed.verify(&state, "set-delegation-announcement").await?;
	let text = |key: &str| entity[key].as_str().unwrap_or_default().trim().to_owned();
	let (short, long) = (text("shortDescription"), text("longDescription"));
	if short.is_empty() || short.chars().count() > 256 || long.chars().count() > 20_000 {
		return Err(ApiError::bad_request(
			"The announcement needs a short description of up to 256 characters",
		));
	}
	let inserted = state
		.db
		.get()
		.await?
		.execute(
			"INSERT INTO delegate_announcements (address, short_description, long_description)
			SELECT $1, $2, $3 WHERE EXISTS (SELECT 1 FROM governance_vote_calls WHERE account = $1)
				OR EXISTS (SELECT 1 FROM governance_delegations WHERE target = $1)
			ON CONFLICT (address) DO UPDATE SET short_description = $2, long_description = $3, updated_at = NOW()",
			&[&address, &short, &long],
		)
		.await?;
	if inserted == 0 {
		return Err(ApiError::bad_request(
			"Only accounts that have voted or been delegated to can announce themselves",
		));
	}
	Ok(Json(true))
}

pub async fn unannounce(
	State(state): State<AppState>,
	Json(signed): Json<SignedEntity>,
) -> ApiResult<Json<bool>> {
	let Signed { address, .. } = signed.verify(&state, "unset-delegation-announcement").await?;
	remove_announcement(&state, &address).await
}

/// Removes any account's announcement, which only admins may do.
pub async fn delete_announcement(
	State(state): State<AppState>,
	session: Session,
	Path(address): Path<String>,
) -> ApiResult<Json<bool>> {
	if !state.admins.contains(&session.address) {
		return Err(ApiError::forbidden());
	}
	remove_announcement(&state, &valid_address(&address)?).await
}

async fn remove_announcement(state: &AppState, address: &str) -> ApiResult<Json<bool>> {
	state
		.db
		.get()
		.await?
		.execute("DELETE FROM delegate_announcements WHERE address = $1", &[&address])
		.await?;
	Ok(Json(true))
}
