use axum::{
	extract::{Path, RawQuery, State},
	Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{
	auth::{ConnectedAccount, Session},
	posts, referenda, with_param, AppState,
};
use crate::{
	error::{ApiError, ApiResult},
	primitives::{author, Page},
};

/// An account's public profile. Wallet accounts have no username or email, so those come from the
/// key.
pub async fn profile(state: &AppState, address: &str) -> ApiResult<Value> {
	let row = state
		.db
		.query_opt("SELECT avatar_cid, banner_cid, bio FROM users WHERE address = $1", &[&address])
		.await?;
	let field = |name: &str| row.as_ref().and_then(|row| row.get::<_, Option<String>>(name));

	let mut profile = author(address);
	profile["avatarCid"] = field("avatar_cid").into();
	profile["bannerCid"] = field("banner_cid").into();
	profile["bio"] = field("bio").into();
	profile["emailVerified"] = false.into();
	profile["notification"] = json!({ "reply": false, "mention": false });
	Ok(profile)
}

pub async fn status(session: Option<Session>) -> Json<Value> {
	Json(json!({ "isLoggedIn": session.is_some() }))
}

/// The visitor's own profile, for a signed in session or else for the account they connected.
pub async fn me(
	State(state): State<AppState>,
	session: Option<Session>,
	connected: Option<ConnectedAccount>,
) -> ApiResult<Json<Value>> {
	let address = session
		.map(|session| session.address)
		.or(connected.map(|ConnectedAccount(address)| address))
		.ok_or_else(ApiError::unauthorized)?;
	Ok(Json(profile(&state, &address).await?))
}

pub async fn user(
	State(state): State<AppState>,
	Path(address): Path<String>,
) -> ApiResult<Json<Value>> {
	Ok(Json(profile(&state, &address).await?))
}

pub async fn admins(State(state): State<AppState>) -> Json<Vec<String>> {
	Json(state.admins.as_ref().clone())
}

#[derive(Deserialize)]
pub struct AvatarRequest {
	addresses: Vec<String>,
}

/// The avatars set for a batch of addresses the frontend is about to show.
pub async fn avatars(
	State(state): State<AppState>,
	Json(request): Json<AvatarRequest>,
) -> ApiResult<Json<Value>> {
	if request.addresses.len() > 500 {
		return Err(ApiError::bad_request("Too many addresses"));
	}
	let rows = state
		.db
		.query(
			"SELECT address, avatar_cid FROM users WHERE address = ANY($1) AND avatar_cid IS NOT NULL",
			&[&request.addresses],
		)
		.await?;
	Ok(Json(
		rows.iter()
			.map(
				|row| json!({ "address": row.get::<_, String>(0), "avatarCid": row.get::<_, String>(1) }),
			)
			.collect(),
	))
}

/// How much an account has done, for the tabs of its profile. Nexus has no democracy, council, tips
/// or bounties pallets, so those are always zero.
pub async fn counts(
	State(state): State<AppState>,
	Path(address): Path<String>,
) -> ApiResult<Json<Value>> {
	let row = state
		.db
		.query_one(
			"SELECT
				(SELECT count(*) FROM posts WHERE kind = 0 AND author = $1 AND NOT hidden) AS discussions,
				(SELECT count(*) FROM comments WHERE author = $1 AND NOT deleted) AS comments,
				(SELECT count(*) FROM governance_referendums WHERE proposer = $1) AS referenda,
				(SELECT count(*) FROM governance_motions WHERE proposer = $1) AS motions",
			&[&address],
		)
		.await?;
	Ok(Json(json!({
		"discussions": row.get::<_, i64>("discussions"),
		"comments": row.get::<_, i64>("comments"),
		"openGov": { "referenda": row.get::<_, i64>("referenda") },
		"techComm": { "proposals": row.get::<_, i64>("motions") },
		"treasury": { "tips": 0, "proposals": 0, "bounties": 0 },
		"democracy": { "proposals": 0, "externals": 0, "referendums": 0 },
		"council": { "motions": 0 },
		"polkassemblyDiscussions": 0,
	})))
}

pub async fn referenda(
	state: State<AppState>,
	Path(address): Path<String>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	referenda::list(state, RawQuery(Some(with_param(query, &format!("proposer={address}"))))).await
}

pub async fn discussions(
	state: State<AppState>,
	Path(address): Path<String>,
	RawQuery(query): RawQuery,
) -> ApiResult<Json<Page<Value>>> {
	posts::list(state, RawQuery(Some(with_param(query, &format!("author={address}"))))).await
}
