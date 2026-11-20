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
use chrono::{SecondsFormat, Utc};
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{common::valid_address, users::profile, AppState};
use crate::{
	crypto::{canonical, random_hex, token_hash, verify},
	error::{ApiError, ApiResult},
};

const AUTH_COOKIE: &str = "auth-token";
const CONNECTED_COOKIE: &str = "connected-account";
const SESSION_SECONDS: i64 = 30 * 24 * 60 * 60;

/// The signed in account, from the session cookie or the bearer token server side rendering sends.
pub struct Session {
	pub token_hash: String,
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
		let token_hash = token_hash(&token);
		let row = state
			.db
			.get()
			.await?
			.query_opt(
				"SELECT address FROM sessions WHERE token_hash = $1 AND expires_at > NOW()",
				&[&token_hash],
			)
			.await?
			.ok_or_else(ApiError::unauthorized)?;
		Ok(Session { token_hash, address: row.get(0) })
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
			.and_then(|account| account["address"].as_str().and_then(canonical))
			.ok_or_else(ApiError::unauthorized)?;
		Ok(ConnectedAccount(account))
	}
}

fn session_cookie(token: &str, max_age: i64) -> String {
	format!("{AUTH_COOKIE}={token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={max_age}")
}

/// The message a wallet signs to log in. It names the forum, what the signature is for and when
/// it was asked for, so a wallet shows a site relaying it for what it is.
fn sign_in_message(domain: &str, address: &str, nonce: &str) -> String {
	let issued = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
	format!(
		"{domain} wants you to sign in with your account:\n{address}\n\n\
		Purpose: Sign in to the {domain} governance forum\nNonce: {nonce}\nIssued at: {issued}"
	)
}

/// Starts a wallet login with a one time challenge for the account to sign. Challenges count
/// against each visitor's login allowance, and expired ones are cleared as new ones are made, so
/// they cannot pile up. Nobody can cancel a challenge someone else asked for.
pub async fn challenge(
	State(state): State<AppState>,
	Path(address): Path<String>,
) -> ApiResult<Json<Value>> {
	let address = valid_address(&address)?;
	let attempt = random_hex(16);
	let challenge = sign_in_message(&state.domain, &address, &random_hex(16));
	state
		.db
		.get()
		.await?
		.execute(
			"WITH expired AS (
				DELETE FROM login_attempts WHERE created_at < NOW() - INTERVAL '10 minutes'
			)
			INSERT INTO login_attempts (id, address, challenge) VALUES ($1, $2, $3)",
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
		.get()
		.await?
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
		.get()
		.await?
		.execute(
			"INSERT INTO sessions (token_hash, address, expires_at)
			VALUES ($1, $2, NOW() + make_interval(secs => $3::double precision))",
			&[&token_hash(&token), &address, &(SESSION_SECONDS as f64)],
		)
		.await?;
	state
		.db
		.get()
		.await?
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
			.get()
			.await?
			.execute("DELETE FROM sessions WHERE token_hash = $1", &[&session.token_hash])
			.await?;
	}
	Ok(([(SET_COOKIE, session_cookie("", 0))], Json(json!({ "result": true }))))
}
