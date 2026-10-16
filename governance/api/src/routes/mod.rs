mod auth;
mod chain;
mod comments;
mod common;
pub mod files;
mod motions;
mod posts;
mod referenda;
mod signed;
mod spends;
mod statistics;
#[cfg(test)]
mod tests;
mod users;
mod votes;

use std::sync::Arc;

use axum::{
	extract::DefaultBodyLimit,
	http::{Method, Uri},
	middleware,
	routing::{get, patch, post, put},
	Router,
};
use tokio_postgres::Client;

use axum::{extract::State, Json};

use self::{signed::SignedEntity, users::ProfileField};
use crate::{
	error::{ApiError, ApiResult},
	limits::{self, RateLimiter},
	primitives::Paging,
};

#[derive(Clone)]
pub struct AppState {
	pub db: Arc<Client>,
	/// Addresses allowed to moderate the forum
	pub admins: Arc<Vec<String>>,
	pub limiter: Arc<RateLimiter>,
}

pub fn router(state: AppState) -> Router {
	let router = Router::new()
		.route("/auth/login/:id", get(auth::challenge).post(auth::login))
		.route("/auth/logout", post(auth::logout))
		.route("/user/status", get(users::status))
		.route("/user/profile", get(users::me))
		.route("/users/:address", get(users::user))
		.route("/admins", get(users::admins))
		.route("/user/bio", post(|state, body| profile(state, ProfileField::Bio, true, body)))
		.route(
			"/user/bio/unset",
			post(|state, body| profile(state, ProfileField::Bio, false, body)),
		)
		.route("/user/banner", post(|state, body| profile(state, ProfileField::Banner, true, body)))
		.route(
			"/user/banner/unset",
			post(|state, body| profile(state, ProfileField::Banner, false, body)),
		)
		.route("/user/avatar", post(|state, body| profile(state, ProfileField::Avatar, true, body)))
		.route(
			"/user/avatar/unset",
			post(|state, body| profile(state, ProfileField::Avatar, false, body)),
		)
		.route("/avatars", post(users::avatars))
		.route(
			"/s3/files",
			post(files::upload).layer(DefaultBodyLimit::max(files::MAX_FILE_SIZE + 64 * 1024)),
		)
		.route("/s3/files/:id", get(files::get))
		.route("/inspect/scan-height", get(chain::scan_height))
		.route("/stream/scan-height", get(chain::scan_height_stream))
		.route("/overview/summary", get(chain::summary))
		.route("/overview/recent/summary", get(chain::summary))
		.route("/overview/referenda", get(chain::overview_referenda))
		.route("/overview/tc-motions", get(chain::overview_motions))
		.route("/overview/treasury-spends", get(chain::overview_spends))
		.route("/overview/discussions", get(chain::overview_discussions))
		.route("/gov2/tracks/:id", get(referenda::track_detail))
		.route("/gov2/tracks/:id/referendums", get(referenda::track_list))
		.route("/gov2/tracks/:id/referendums/summary", get(referenda::track_summary))
		.route("/treasury/spends", get(spends::list))
		.route("/treasury/spends/summary", get(spends::summary))
		.route("/treasury/spends/:id", get(spends::detail).patch(posts::edit))
		.route("/users/:address/counts", get(users::counts))
		.route("/users/:address/gov2/referendums", get(users::referenda))
		.route("/users/:address/posts", get(users::discussions))
		.route("/users/:address/referenda/votes", get(votes::user_votes))
		.route("/users/:address/referenda/vote-calls", get(votes::user_vote_calls))
		.route("/users/:address/referenda/vote-stats", get(votes::user_vote_stats))
		.route("/delegation/referenda/delegates/:address", get(statistics::delegate))
		.route("/gov2/tracks", get(referenda::tracks))
		.route("/preimages/:hash", get(referenda::preimage))
		.route("/gov2/tracks/summary", get(referenda::tracks_summary))
		.route("/gov2/referendums", get(referenda::list))
		.route("/gov2/referendums/summary", get(referenda::summary))
		.route("/gov2/referendums/:id", get(referenda::detail).patch(posts::edit))
		.route("/gov2/referendums/:id/vote-stats", get(votes::vote_stats))
		.route("/gov2/referendums/:id/vote-calls", get(votes::vote_calls))
		.route("/gov2/referenda/:id/votes", get(votes::votes))
		.route("/gov2/referenda/:id/actions", get(votes::actions))
		.route("/gov2/referenda/:id/tally-history", get(votes::tally_history))
		.route("/gov2/referenda/:id/appendants", get(comments::appendants).post(comments::append))
		.route(
			"/appendants/:id",
			patch(comments::edit_appendant).delete(comments::delete_appendant),
		)
		.route("/delegation/referenda/delegates", get(statistics::delegates))
		.route("/delegation/announcements", post(statistics::announce))
		.route("/delegation/announcements/unset", post(statistics::unannounce))
		.route("/referenda/summary", get(statistics::summary))
		.route("/referenda/tracks", get(statistics::tracks))
		.route("/referenda/delegatee", get(statistics::all_delegatees))
		.route("/referenda/tracks/:id/summary", get(statistics::track_summary))
		.route("/referenda/tracks/:id/turnout", get(statistics::track_turnout))
		.route("/referenda/tracks/:id/delegatee", get(statistics::track_delegatees))
		.route("/referenda/tracks/:id/delegators", get(statistics::track_delegators))
		.route("/tech-comm/motions", get(motions::list))
		.route("/tech-comm/motions/:id", get(motions::detail).patch(posts::edit))
		.route("/posts", get(posts::list).post(posts::create))
		.route("/posts/:id", get(posts::detail).patch(posts::edit).delete(posts::delete))
		.route("/posts/:id/hide", patch(posts::hide))
		.route("/comments/:id", get(comments::get).patch(comments::edit).delete(comments::delete))
		.route("/comments/:id/reaction", put(posts::react).delete(posts::unreact));

	// Every kind of post shares its comment and reaction routes, the post id says which kind it is
	let router = ["posts", "gov2/referendums", "tech-comm/motions", "treasury/spends"]
		.into_iter()
		.fold(router, |router, prefix| {
			router
				.route(
					&format!("/{prefix}/:id/comments"),
					get(comments::list).post(comments::create),
				)
				.route(&format!("/{prefix}/:id/comments/:comment/replies"), post(comments::reply))
				.route(&format!("/{prefix}/:id/reaction"), put(posts::react).delete(posts::unreact))
		});
	router
		.layer(middleware::from_fn_with_state(state.clone(), limits::limit))
		.fallback(unknown_route)
		.with_state(state)
}

async fn profile(
	State(state): State<AppState>,
	field: ProfileField,
	set: bool,
	Json(signed): Json<SignedEntity>,
) -> ApiResult<Json<bool>> {
	users::update_profile(&state, field, set, signed).await
}

/// Logs requests for routes this API does not serve, so pages relying on them are easy to find.
async fn unknown_route(method: Method, uri: Uri) -> ApiError {
	tracing::warn!(target: crate::LOG_TARGET, "No route for {method} {uri}");
	ApiError::not_found("Route")
}

/// Query string pairs in order. The frontend repeats keys such as `referendum_index` to send a
/// list.
pub struct Params(Vec<(String, String)>);

/// A query string with one more `key=value` pair, for routes that narrow another route's list.
pub fn with_param(query: Option<String>, pair: &str) -> String {
	match query.filter(|query| !query.is_empty()) {
		Some(query) => format!("{query}&{pair}"),
		None => pair.to_owned(),
	}
}

impl Params {
	/// The frontend sends filters it does not use as empty values, so those count as not set.
	pub fn new(query: Option<String>) -> Self {
		Self(
			form_urlencoded::parse(query.unwrap_or_default().as_bytes())
				.into_owned()
				.filter(|(_, value)| !value.is_empty())
				.collect(),
		)
	}

	pub fn get(&self, key: &str) -> Option<&str> {
		self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
	}

	pub fn all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> {
		self.0.iter().filter(move |(k, _)| k == key).map(|(_, v)| v.as_str())
	}

	pub fn flag(&self, key: &str) -> bool {
		matches!(self.get(key), Some("true" | "1"))
	}

	pub fn paging(&self, default_size: i64) -> Paging {
		let number = |key, default| self.get(key).and_then(|v| v.parse().ok()).unwrap_or(default);
		Paging {
			page: number("page", 1).max(1),
			page_size: number("page_size", default_size).clamp(1, 100),
		}
	}
}

#[cfg(test)]
mod params_tests {
	use super::*;

	#[test]
	fn reads_query_strings_the_way_the_frontend_sends_them() {
		let params = Params::new(Some(
			"status=&referendum_index=1&referendum_index=2&ongoing=true&page_size=500".into(),
		));
		assert_eq!(params.get("status"), None, "an empty filter is not a filter");
		assert_eq!(params.all("referendum_index").collect::<Vec<_>>(), ["1", "2"]);
		assert!(params.flag("ongoing"));
		assert_eq!(params.paging(10).page_size, 100, "pages are capped");
		assert_eq!(with_param(None, "track=1"), "track=1");
		assert_eq!(with_param(Some("page=2".into()), "track=1"), "page=2&track=1");
	}
}
