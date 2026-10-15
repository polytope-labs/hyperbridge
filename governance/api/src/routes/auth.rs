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
use serde::Deserialize;
use serde_json::{json, Value};
use sp_core::crypto::{AccountId32, Ss58Codec};

use super::{users::profile, AppState};
use crate::{
	crypto::{random_hex, verify},
	error::{ApiError, ApiResult},
};

const AUTH_COOKIE: &str = "auth-token";
const SESSION_SECONDS: i64 = 30 * 24 * 60 * 60;

/// The signed in account, from the session cookie or the bearer token server side rendering sends.
pub struct Session {
	pub token: String,
	pub address: String,
}

fn session_token(headers: &HeaderMap) -> Option<String> {
	let values = |name| headers.get_all(name).iter().filter_map(|value| value.to_str().ok());
	let bearer = values(AUTHORIZATION).find_map(|value| value.strip_prefix("Bearer "));
	let cookie = || {
		values(COOKIE)
			.flat_map(|cookies| cookies.split(';'))
			.find_map(|cookie| cookie.trim().strip_prefix(AUTH_COOKIE)?.strip_prefix('='))
	};
	bearer.or_else(cookie).map(str::to_owned)
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
