//! A database for tests: `INDEXER_TEST_DATABASE_URL`, with the indexer's migrations applied.
//! Each test works inside a transaction it never commits.

use tokio_postgres::{Client, NoTls};

pub async fn database() -> Option<Client> {
	let url = std::env::var("INDEXER_TEST_DATABASE_URL").ok()?;
	let (client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	// Test binaries run in parallel; the first to arrive migrates a fresh database.
	client.execute("SELECT pg_advisory_lock(7171)", &[]).await.unwrap();
	let migrated: bool = client
		.query_one("SELECT to_regclass('app.request_v2s') IS NOT NULL", &[])
		.await
		.unwrap()
		.get(0);
	if !migrated {
		for sql in [
			"CREATE SCHEMA IF NOT EXISTS indexer",
			include_str!("../../migrations/0000_indexer.sql"),
			include_str!("../../migrations/0001_app_schema.sql"),
			include_str!("../../migrations/0002_deferred_fill_points.sql"),
		] {
			client.batch_execute(sql).await.unwrap();
		}
	}
	client.execute("SELECT pg_advisory_unlock(7171)", &[]).await.unwrap();
	Some(client)
}

/** One column of one row, as text. */
pub async fn text(db: &tokio_postgres::Transaction<'_>, sql: &str) -> Option<String> {
	db.query_opt(sql, &[]).await.unwrap().map(|row| row.get(0))
}
