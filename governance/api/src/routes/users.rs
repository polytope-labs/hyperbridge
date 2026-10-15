use axum::{
	extract::{Path, State},
	Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{auth::Session, AppState};
use crate::{
	error::{ApiError, ApiResult},
	primitives::author,
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

pub async fn me(State(state): State<AppState>, session: Session) -> ApiResult<Json<Value>> {
	Ok(Json(profile(&state, &session.address).await?))
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
