mod crypto;
mod db;
mod error;
mod forum_client;
mod import;
mod limits;
mod primitives;
mod routes;
mod verify;

use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use axum::http::HeaderValue;
use tower_http::cors::{Any, CorsLayer};

use crate::{limits::RateLimiter, routes::AppState};

pub const LOG_TARGET: &str = "governance-api";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	tracing_subscriber::fmt::init();

	let args = env::args().collect::<Vec<_>>();
	let command = args.get(1).map(String::as_str);
	let site = || {
		args.get(2)
			.map(String::as_str)
			.context("give the current forum's address, such as https://forum.example")
	};

	// `verify <site> [api]` compares a running API with the current forum and exits
	if command == Some("verify") {
		let api = args.get(3).map(String::as_str).unwrap_or("http://localhost:7071");
		return verify::run(api, site()?).await;
	}

	let database_url = env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
	let db = db::connect(&database_url).await?;

	// `import <site>` copies the current forum's off-chain content into this one and exits
	if command == Some("import") {
		return import::run(&db, site()?).await;
	}

	let admins = env::var("ADMINS")
		.unwrap_or_default()
		.split(',')
		.map(str::trim)
		.filter(|admin| !admin.is_empty())
		.map(str::to_owned)
		.collect();
	let state = AppState {
		db: Arc::new(db),
		admins: Arc::new(admins),
		limiter: Arc::new(RateLimiter::default()),
	};
	tokio::spawn(clean_up(state.clone()));
	let app = routes::router(state).layer(cors());

	let port = env::var("PORT").unwrap_or_else(|_| "7071".into());
	tracing::info!(target: LOG_TARGET, "Starting server on port: {port}");
	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
	axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

	Ok(())
}

/// Browsers call the API directly for some requests, so the site's origin has to be allowed.
/// Without `CORS_ORIGINS` every origin is, which is only meant for running locally.
fn cors() -> CorsLayer {
	let origins = env::var("CORS_ORIGINS").unwrap_or_default();
	let origins = origins
		.split(',')
		.map(str::trim)
		.filter(|origin| !origin.is_empty())
		.filter_map(|origin| origin.parse::<HeaderValue>().ok())
		.collect::<Vec<_>>();
	if origins.is_empty() {
		tracing::warn!(target: LOG_TARGET, "CORS_ORIGINS is not set, so every origin is allowed");
		return CorsLayer::permissive();
	}
	CorsLayer::new().allow_origin(origins).allow_methods(Any).allow_headers(Any)
}

/// Clears expired sessions, unanswered login challenges and ended rate limit windows every hour.
async fn clean_up(state: AppState) {
	let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
	loop {
		interval.tick().await;
		let cleared = state
			.db
			.batch_execute(
				"DELETE FROM sessions WHERE expires_at < NOW();
				DELETE FROM login_attempts WHERE created_at < NOW() - INTERVAL '10 minutes';",
			)
			.await;
		if let Err(err) = cleared {
			tracing::error!(target: LOG_TARGET, "cleaning up sessions failed: {err}");
		}
		state.limiter.prune();
	}
}
