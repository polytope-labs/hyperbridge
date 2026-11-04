//! A database for tests: `INDEXER_TEST_DATABASE_URL`, with the indexer's migrations applied.
//! Each test works inside a transaction it never commits.

use tokio_postgres::{Client, NoTls};

pub async fn database() -> Option<Client> {
	let url = std::env::var("INDEXER_TEST_DATABASE_URL").ok()?;
	let (mut client, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	hyperbridge_indexer::db::apply_migrations(&mut client).await.unwrap();
	Some(client)
}

/** One column of one row, as text. */
pub async fn text(db: &tokio_postgres::Transaction<'_>, sql: &str) -> Option<String> {
	db.query_opt(sql, &[]).await.unwrap().map(|row| row.get(0))
}
