use rindexer::PostgresClient;

/** Schema migrations, applied in order. `app` holds the tables omnihedron serves. */
const MIGRATIONS: &[(&str, &str)] = &[
	("0000_indexer", include_str!("../migrations/0000_indexer.sql")),
	("0001_app_schema", include_str!("../migrations/0001_app_schema.sql")),
	("0002_deferred_fill_points", include_str!("../migrations/0002_deferred_fill_points.sql")),
];

/** Applies the migrations that haven't run yet. */
pub async fn migrate(db: &PostgresClient) -> anyhow::Result<()> {
	let mut conn = db.raw_connection().await?;
	apply_migrations(&mut conn).await
}

/** The advisory lock a process holds while it migrates. */
const MIGRATION_LOCK: i64 = 7171;

/**
 * Applies the migrations `indexer.migrations` doesn't list yet. Every chain's process migrates
 * at startup, so they take turns under a lock: the first applies them, and the rest find them
 * applied.
 */
pub async fn apply_migrations(conn: &mut tokio_postgres::Client) -> anyhow::Result<()> {
	conn.execute("SELECT pg_advisory_lock($1)", &[&MIGRATION_LOCK]).await?;
	let applied = apply_pending(conn).await;
	// The connection may go back to a pool, so release the lock even when a migration failed.
	let unlocked = conn.execute("SELECT pg_advisory_unlock($1)", &[&MIGRATION_LOCK]).await;
	applied?;
	unlocked?;
	Ok(())
}

/** Applies each pending migration in its own transaction. */
async fn apply_pending(conn: &mut tokio_postgres::Client) -> anyhow::Result<()> {
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
