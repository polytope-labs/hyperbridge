mod crypto;
mod db;
mod error;
mod import;
mod primitives;
mod routes;

use std::{env, sync::Arc};

use anyhow::Context;
use tower_http::cors::CorsLayer;

use crate::routes::AppState;

pub const LOG_TARGET: &str = "governance-api";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	dotenv::dotenv().ok();
	tracing_subscriber::fmt::init();

	let database_url = env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
	let db = db::connect(&database_url).await?;

	// `import [site]` copies a SubSquare site's off-chain content into the forum and exits
	let args = env::args().collect::<Vec<_>>();
	if args.get(1).map(String::as_str) == Some("import") {
		let site = args.get(2).map(String::as_str).unwrap_or("https://hyperbridge.subsquare.io");
		return import::run(&db, site).await;
	}

	let admins = env::var("ADMINS")
		.unwrap_or_default()
		.split(',')
		.map(str::trim)
		.filter(|admin| !admin.is_empty())
		.map(str::to_owned)
		.collect();
	let app = routes::router(AppState { db: Arc::new(db), admins: Arc::new(admins) })
		.layer(CorsLayer::permissive());

	let port = env::var("PORT").unwrap_or_else(|_| "7071".into());
	tracing::info!(target: LOG_TARGET, "Starting server on port: {port}");
	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
	axum::serve(listener, app).await?;

	Ok(())
}
