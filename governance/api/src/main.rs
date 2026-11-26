mod crypto;
mod db;
mod error;
mod forum_client;
mod import;
mod limits;
mod primitives;
mod routes;
mod verify;

use std::{collections::BTreeSet, env, net::SocketAddr, sync::Arc, time::Duration};

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

	// `import <site>` copies the current forum's off-chain content into this one and exits
	if command == Some("import") {
		return import::run(&db::connect(&database_url).await?, site()?).await;
	}

	let admins = env::var("ADMINS")
		.unwrap_or_default()
		.split(',')
		.map(str::trim)
		.filter(|admin| !admin.is_empty())
		.map(|admin| {
			crypto::canonical(admin).with_context(|| format!("{admin} in ADMINS is not an address"))
		})
		.collect::<anyhow::Result<BTreeSet<_>>>()?;
	let trusted = RateLimiter::trusted_proxies(&env::var("TRUSTED_PROXIES").unwrap_or_default())?;
	if trusted.is_empty() {
		tracing::warn!(target: LOG_TARGET, "TRUSTED_PROXIES is not set, so X-Forwarded-For is ignored");
	}
	// Login messages name this domain, so a wallet never signs one naming the wrong site
	let domain = env::var("FORUM_DOMAIN")
		.ok()
		.map(|domain| domain.trim().to_owned())
		.filter(|domain| !domain.is_empty())
		.context("FORUM_DOMAIN is not set, give the forum's domain, such as forum.example")?;
	let state =
		AppState::new(db::pool(&database_url).await?, admins, RateLimiter::new(trusted), &domain)
			.await?;
	tokio::spawn(clean_up(state.clone()));
	tokio::spawn(prune_limits(state.limiter.clone()));
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

/// The hourly sweeps, each run on its own so one that fails or times out leaves the others done.
/// Uploads nothing has linked to in the day after they were made are removed. Uploads are named by
/// the hex of their sha256 and only checked for a week, and the ids the forum's text links to are
/// gathered in one pass, so the sweep grows with the forum's text rather than with files times
/// text.
const CLEAN_UPS: [(&str, &str); 3] = [
	("expired sessions", "DELETE FROM sessions WHERE expires_at < NOW()"),
	("spent signatures", "DELETE FROM used_signatures WHERE expires_at < NOW()"),
	(
		"unused uploads",
		"WITH linked AS (
			SELECT match[1] AS id FROM (
				SELECT content AS text FROM posts
				UNION ALL SELECT content FROM comments
				UNION ALL SELECT content FROM appendants
				UNION ALL SELECT long_description FROM delegate_announcements
			) texts, regexp_matches(texts.text, '(?<![0-9a-fA-F])([0-9a-f]{64})(?![0-9a-fA-F])', 'g') match
			UNION SELECT banner_cid FROM posts
			UNION SELECT avatar_cid FROM users
			UNION SELECT banner_cid FROM users
		)
		DELETE FROM files f WHERE f.uploader IS NOT NULL
			AND f.uploaded_at BETWEEN NOW() - INTERVAL '7 days' AND NOW() - INTERVAL '1 day'
			AND NOT EXISTS (SELECT 1 FROM linked WHERE linked.id = f.id)",
	),
];

/// Runs the clean ups every hour.
async fn clean_up(state: AppState) {
	let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));
	loop {
		interval.tick().await;
		for (what, statement) in CLEAN_UPS {
			let cleared =
				async { anyhow::Ok(state.db.get().await?.execute(statement, &[]).await?) };
			if let Err(err) = cleared.await {
				tracing::error!(target: LOG_TARGET, "clearing {what} failed: {err:?}");
			}
		}
	}
}

/// Forgets ended rate limit windows each window, so the limiter only holds recent visitors.
async fn prune_limits(limiter: Arc<RateLimiter>) {
	let mut interval = tokio::time::interval(limits::WINDOW);
	loop {
		interval.tick().await;
		limiter.prune();
	}
}
