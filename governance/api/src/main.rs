mod db;
mod error;
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
	let app = routes::router(AppState { db: Arc::new(db) }).layer(CorsLayer::permissive());

	let port = env::var("PORT").unwrap_or_else(|_| "7071".into());
	tracing::info!(target: LOG_TARGET, "Starting server on port: {port}");
	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
	axum::serve(listener, app).await?;

	Ok(())
}
