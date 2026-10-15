use axum::{
	async_trait,
	extract::{FromRequestParts, Path, State},
	http::{
		header::{AUTHORIZATION, COOKIE, SET_COOKIE},
		request::Parts,
		HeaderMap,
	},
	response::IntoResponse,
	Json,
};
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use serde_json::{json, Value};
use sp_core::crypto::{AccountId32, Ss58Codec};

use super::{users::profile, AppState};
use crate::{
	crypto::{random_hex, verify},
	error::{ApiError, ApiResult},
};

const AUTH_COOKIE: &str = "auth-token";
const CONNECTED_COOKIE: &str = "connected-account";
const SESSION_SECONDS: i64 = 30 * 24 * 60 * 60;

/// The signed in account, from the session cookie or the bearer token server side rendering sends.
pub struct Session {
	pub token: String,
	pub address: String,
}

/// A cookie's raw value from the request.
fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
	headers
		.get_all(COOKIE)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.flat_map(|cookies| cookies.split(';'))
		.find_map(|cookie| cookie.trim().strip_prefix(name)?.strip_prefix('='))
}

fn session_token(headers: &HeaderMap) -> Option<String> {
	let bearer = headers
		.get(AUTHORIZATION)
		.and_then(|value| value.to_str().ok())
		.and_then(|value| value.strip_prefix("Bearer "));
	bearer.or_else(|| cookie(headers, AUTH_COOKIE)).map(str::to_owned)
}

#[async_trait]
impl FromRequestParts<AppState> for Session {
	type Rejection = ApiError;

	async fn from_request_parts(
		parts: &mut Parts,
		state: &AppState,
	) -> Result<Self, Self::Rejection> {
		let token = session_token(&parts.headers).ok_or_else(ApiError::unauthorized)?;
		let row = state
			.db
			.query_opt(
				"SELECT address FROM sessions WHERE token = $1 AND expires_at > NOW()",
				&[&token],
			)
			.await?
			.ok_or_else(ApiError::unauthorized)?;
		Ok(Session { token, address: row.get(0) })
	}
}

/// The account a visitor picked in the frontend. Picking an account needs no signature, so this
/// only says whose public profile to show; anything that writes needs a `Session`.
pub struct ConnectedAccount(pub String);

#[async_trait]
impl FromRequestParts<AppState> for ConnectedAccount {
	type Rejection = ApiError;

	async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Self::Rejection> {
		let account = cookie(&parts.headers, CONNECTED_COOKIE)
			.and_then(|value| percent_decode_str(value).decode_utf8().ok())
			.and_then(|value| serde_json::from_str::<Value>(&value).ok())
			.and_then(|account| account["address"].as_str().map(str::to_owned))
			.filter(|address| AccountId32::from_ss58check(address).is_ok())
			.ok_or_else(ApiError::unauthorized)?;
		Ok(ConnectedAccount(account))
	}
}

fn session_cookie(token: &str, max_age: i64) -> String {
	format!("{AUTH_COOKIE}={token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={max_age}")
}

/// Starts a wallet login with a one time challenge for the account to sign.
pub async fn challenge(
	State(state): State<AppState>,
	Path(address): Path<String>,
) -> ApiResult<Json<Value>> {
	AccountId32::from_ss58check(&address).map_err(|_| ApiError::bad_request("Invalid address"))?;
	let (attempt, challenge) = (random_hex(16), random_hex(16));
	state
		.db
		.execute(
			"INSERT INTO login_attempts (id, address, challenge) VALUES ($1, $2, $3)",
			&[&attempt, &address, &challenge],
		)
		.await?;
	Ok(Json(json!({ "challenge": challenge, "attemptId": attempt })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeAnswer {
	challenge_answer: String,
}

/// Finishes a wallet login. Each challenge can be answered once, within ten minutes.
pub async fn login(
	State(state): State<AppState>,
	Path(attempt): Path<String>,
	Json(answer): Json<ChallengeAnswer>,
) -> ApiResult<impl IntoResponse> {
	let row = state
		.db
		.query_opt(
			"DELETE FROM login_attempts WHERE id = $1 AND created_at > NOW() - INTERVAL '10 minutes'
			RETURNING address, challenge",
			&[&attempt],
		)
		.await?
		.ok_or_else(|| ApiError::bad_request("The login request has expired, please try again"))?;
	let (address, challenge) = (row.get::<_, String>(0), row.get::<_, String>(1));
	if !verify(&address, &challenge, &answer.challenge_answer) {
		return Err(ApiError::bad_request("Invalid signature"));
	}

	let token = random_hex(32);
	state
		.db
		.execute(
			"INSERT INTO sessions (token, address, expires_at)
			VALUES ($1, $2, NOW() + make_interval(secs => $3::double precision))",
			&[&token, &address, &(SESSION_SECONDS as f64)],
		)
		.await?;
	state
		.db
		.execute("INSERT INTO users (address) VALUES ($1) ON CONFLICT DO NOTHING", &[&address])
		.await?;

	Ok((
		[(SET_COOKIE, session_cookie(&token, SESSION_SECONDS))],
		Json(profile(&state, &address).await?),
	))
}

pub async fn logout(
	State(state): State<AppState>,
	session: Option<Session>,
) -> ApiResult<impl IntoResponse> {
	if let Some(session) = session {
		state
			.db
			.execute("DELETE FROM sessions WHERE token = $1", &[&session.token])
			.await?;
	}
	Ok(([(SET_COOKIE, session_cookie("", 0))], Json(json!({ "result": true }))))
}
