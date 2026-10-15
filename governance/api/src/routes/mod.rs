mod chain;
mod common;
mod motions;
mod referenda;

use std::sync::Arc;

use axum::{routing::get, Router};
use tokio_postgres::Client;

use crate::primitives::Paging;

#[derive(Clone)]
pub struct AppState {
	pub db: Arc<Client>,
}

pub fn router(state: AppState) -> Router {
	Router::new()
		.route("/inspect/scan-height", get(chain::scan_height))
		.route("/stream/scan-height", get(chain::scan_height_stream))
		.route("/overview/summary", get(chain::summary))
		.route("/gov2/tracks", get(referenda::tracks))
		.route("/gov2/tracks/summary", get(referenda::tracks_summary))
		.route("/gov2/referendums", get(referenda::list))
		.route("/gov2/referendums/summary", get(referenda::summary))
		.route("/gov2/referendums/:id", get(referenda::detail))
		.route("/tech-comm/motions", get(motions::list))
		.route("/tech-comm/motions/:id", get(motions::detail))
		.with_state(state)
}

/// Query string pairs in order. SubSquare repeats keys such as `referendum_index` to send a list.
pub struct Params(Vec<(String, String)>);

impl Params {
	pub fn new(query: Option<String>) -> Self {
		Self(
			form_urlencoded::parse(query.unwrap_or_default().as_bytes())
				.into_owned()
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
