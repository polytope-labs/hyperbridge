use rindexer::PostgresClient;

/** Schema migrations, applied in order. `app` holds the tables omnihedron serves. */
const MIGRATIONS: &[(&str, &str)] = &[
	("0000_indexer", include_str!("../migrations/0000_indexer.sql")),
	("0001_app_schema", include_str!("../migrations/0001_app_schema.sql")),
];

/** Applies the migrations that haven't run yet, each in its own transaction. */
pub async fn migrate(db: &PostgresClient) -> anyhow::Result<()> {
	let mut conn = db.raw_connection().await?;
	conn.batch_execute(
		"CREATE SCHEMA IF NOT EXISTS indexer;
		 CREATE TABLE IF NOT EXISTS indexer.migrations (
			name text PRIMARY KEY,
			applied_at timestamptz NOT NULL DEFAULT now()
		 );",
	)
	.await?;
	for (name, sql) in MIGRATIONS {
		let tx = conn.transaction().await?;
		let applied = tx
			.query_opt("SELECT 1 FROM indexer.migrations WHERE name = $1", &[name])
			.await?
			.is_some();
		if !applied {
			tracing::info!(migration = name, "applying");
			tx.batch_execute(sql).await?;
			tx.execute("INSERT INTO indexer.migrations (name) VALUES ($1)", &[name]).await?;
		}
		tx.commit().await?;
	}
	Ok(())
}
