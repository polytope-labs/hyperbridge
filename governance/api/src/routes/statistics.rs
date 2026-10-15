use std::collections::{BTreeMap, BTreeSet};

use axum::{
	extract::{Path, RawQuery, State},
	Json,
};
use serde_json::{json, Value};

use super::{
	signed::SignedEntity,
	votes::{participation, referendum_stats, with_conviction},
	AppState, Params,
};
use crate::{
	error::{ApiError, ApiResult},
	primitives::Page,
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
		.query(
			"SELECT delegator, target, track, conviction, balance::text FROM governance_delegations
			WHERE active AND ($1::int IS NULL OR track = $1)",
			&[&track],
		)
		.await?;
	Ok(rows
		.iter()
		.map(|row| {
			let balance = row.get::<_, String>(4).parse().unwrap_or_default();
			Delegation {
				delegator: row.get(0),
				target: row.get(1),
				track: row.get(2),
				balance,
				votes: with_conviction(balance, row.get(3)),
			}
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
		.query("SELECT id::int, name FROM governance_tracks ORDER BY id::int", &[])
		.await?;
	Ok(rows.iter().map(|row| (row.get(0), row.get(1))).collect())
}

/// Every referendum of a track with its turnout, oldest first.
async fn turnouts(state: &AppState, track: i32) -> ApiResult<Vec<Value>> {
	let rows = state
		.db
		.query(
			"SELECT index FROM governance_referendums WHERE track = $1 ORDER BY index",
			&[&track],
		)
		.await?;
	let mut turnouts = Vec::with_capacity(rows.len());
	for row in rows {
		turnouts.push(referendum_stats(state, row.get(0)).await?);
	}
	Ok(turnouts)
}

pub async fn track_turnout(
	State(state): State<AppState>,
	Path(track): Path<i32>,
) -> ApiResult<Json<Vec<Value>>> {
	Ok(Json(turnouts(&state, track).await?))
}

pub async fn summary(State(state): State<AppState>) -> ApiResult<Json<Value>> {
	let tracks = track_names(&state).await?;
	let counts = state
		.db
		.query("SELECT track, count(*) FROM governance_referendums GROUP BY track", &[])
		.await?
		.iter()
		.map(|row| (row.get::<_, i32>(0), row.get::<_, i64>(1)))
		.collect::<BTreeMap<_, _>>();
	let total = counts.values().sum::<i64>();

	// A track's turnout is the average over its referenda of their capital, electorate and turnout
	let mut track_turnouts = Vec::with_capacity(tracks.len());
	for (id, name) in &tracks {
		let turnouts = turnouts(&state, *id).await?;
		let count = turnouts.len().max(1) as u128;
		let average = |field: &str| {
			turnouts
				.iter()
				.filter_map(|t| t[field].as_str()?.parse::<u128>().ok())
				.sum::<u128>() /
				count
		};
		let percentage =
			turnouts.iter().filter_map(|t| t["percentage"].as_f64()).sum::<f64>() / count as f64;
		track_turnouts.push(json!({
			"id": id,
			"name": name,
			"capital": average("totalCapital").to_string(),
			"electorate": average("electorate").to_string(),
			"percentage": percentage,
		}));
	}

	let delegations = delegations(&state, None).await?;
	let delegation = delegation_summary(delegations.iter());
	Ok(Json(json!({
		"trackTurnouts": track_turnouts,
		"trackReferendaCounts": tracks
			.iter()
			.map(|(id, name)| {
				let count = counts.get(id).copied().unwrap_or_default();
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

pub async fn track_delegators(
	State(state): State<AppState>,
	Path(track): Path<i32>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	let paging = Params::new(query).paging(25);
	let mut delegations = delegations(&state, Some(track)).await?;
	delegations.sort_by_key(|d| std::cmp::Reverse(d.votes));
	let total = delegations.len() as i64;
	let items = delegations
		.into_iter()
		.skip(paging.offset() as usize)
		.take(paging.page_size as usize)
		.map(|d| {
			json!({
				"account": d.delegator,
				"delegatee": d.target,
				"trackId": d.track,
				"balance": d.balance.to_string(),
				"votes": d.votes.to_string(),
			})
		})
		.collect();
	Ok(Json(paging.page(items, total)))
}

/// What the delegation pages show about an account others may delegate to.
async fn delegate_summary(
	state: &AppState,
	address: &str,
	delegations: &[Delegation],
) -> ApiResult<Value> {
	let to_address = delegations.iter().filter(|d| d.target == address);
	let delegators = to_address.clone().map(|d| &d.delegator).collect::<BTreeSet<_>>().len();
	let tracks = to_address.map(|d| d.track).collect::<BTreeSet<_>>().len();
	let manifesto = state
		.db
		.query_opt(
			"SELECT short_description, long_description FROM delegate_announcements WHERE address = $1",
			&[&address],
		)
		.await?
		.map(|row| json!({ "shortDescription": row.get::<_, String>(0), "longDescription": row.get::<_, String>(1) }));
	Ok(json!({
		"address": address,
		"delegatorsCount": delegators,
		"maxTrackDelegations": tracks,
		"participationRate": participation(state, address).await?,
		"manifesto": manifesto,
	}))
}

pub async fn delegate(
	State(state): State<AppState>,
	Path(address): Path<String>,
) -> ApiResult<Json<Value>> {
	let delegations = delegations(&state, None).await?;
	Ok(Json(delegate_summary(&state, &address, &delegations).await?))
}

/// Accounts that announced themselves as delegates or already have delegations, most delegated
/// first.
pub async fn delegates(
	State(state): State<AppState>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	let paging = Params::new(query).paging(18);
	let delegations = delegations(&state, None).await?;
	let announced = state.db.query("SELECT address FROM delegate_announcements", &[]).await?;
	let addresses = announced
		.iter()
		.map(|row| row.get::<_, String>(0))
		.chain(delegations.iter().map(|d| d.target.clone()))
		.collect::<BTreeSet<_>>();

	let mut items = Vec::with_capacity(addresses.len());
	for address in &addresses {
		items.push(delegate_summary(&state, address, &delegations).await?);
	}
	items.sort_by_key(|item| std::cmp::Reverse(item["delegatorsCount"].as_u64()));
	let total = items.len() as i64;
	let page = items
		.into_iter()
		.skip(paging.offset() as usize)
		.take(paging.page_size as usize)
		.collect();
	Ok(Json(paging.page(page, total)))
}

/// Publishes the signing account's delegate announcement.
pub async fn announce(
	State(state): State<AppState>,
	Json(signed): Json<SignedEntity>,
) -> ApiResult<Json<bool>> {
	let entity = signed.verify("set-delegation-announcement")?;
	let text = |key: &str| entity[key].as_str().unwrap_or_default().trim().to_owned();
	let (short, long) = (text("shortDescription"), text("longDescription"));
	if short.is_empty() || short.chars().count() > 256 || long.chars().count() > 20_000 {
		return Err(ApiError::bad_request(
			"The announcement needs a short description of up to 256 characters",
		));
	}
	state
		.db
		.execute(
			"INSERT INTO delegate_announcements (address, short_description, long_description) VALUES ($1, $2, $3)
			ON CONFLICT (address) DO UPDATE SET short_description = $2, long_description = $3, updated_at = NOW()",
			&[&signed.address, &short, &long],
		)
		.await?;
	Ok(Json(true))
}

pub async fn unannounce(
	State(state): State<AppState>,
	Json(signed): Json<SignedEntity>,
) -> ApiResult<Json<bool>> {
	signed.verify("unset-delegation-announcement")?;
	state
		.db
		.execute("DELETE FROM delegate_announcements WHERE address = $1", &[&signed.address])
		.await?;
	Ok(Json(true))
}
