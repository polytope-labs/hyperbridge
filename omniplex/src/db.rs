use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context};
use rindexer::{manifest::core::Manifest, setup_postgres, PostgresClient};
use tokio_postgres::{Client, Transaction};

use crate::config::Mode;

/** Schema migrations, applied in order. `app` holds the tables omnihedron serves. */
const MIGRATIONS: &[(&str, &str)] = &[
	("0000_indexer", include_str!("../migrations/0000_indexer.sql")),
	("0001_app_schema", include_str!("../migrations/0001_app_schema.sql")),
	("0002_deferred_fill_points", include_str!("../migrations/0002_deferred_fill_points.sql")),
	("0003_explorer", include_str!("../migrations/0003_explorer.sql")),
	("0004_governance", include_str!("../migrations/0004_governance.sql")),
	("0005_mode", include_str!("../migrations/0005_mode.sql")),
];

/** The advisory lock a process holds while it sets up the database. */
const MIGRATION_LOCK: i64 = 7171;

/** The advisory lock a start holds while it claims the database for its mode. */
const MODE_LOCK: i64 = 7172;

/**
 * Applies our migrations, then creates rindexer's tables. Every chain's process does this at
 * startup, so they take turns under a lock: the first does the work, and the rest find it done.
 * `start_rindexer` creates rindexer's tables again, without a lock, and finds them there. That
 * holds while the manifest sets `disable_create_tables`: rindexer's setup then only creates
 * what is missing.
 */
pub async fn migrate(
	db: &PostgresClient,
	manifest_path: &Path,
	manifest: &Manifest,
) -> anyhow::Result<()> {
	let project = manifest_path.parent().context("the manifest has no directory")?;
	let mut conn = db.raw_connection().await?;
	lock(&conn).await?;
	let migrated = async {
		apply_pending(&mut conn).await?;
		setup_postgres(project, manifest).await.context("creating rindexer's tables")?;
		anyhow::Ok(())
	}
	.await;
	unlock(&conn, migrated).await
}

/** Applies the migrations `indexer.migrations` doesn't list yet, under the same lock. */
pub async fn apply_migrations(conn: &mut Client) -> anyhow::Result<()> {
	lock(conn).await?;
	let applied = apply_pending(conn).await;
	unlock(conn, applied).await
}

async fn lock(conn: &Client) -> anyhow::Result<()> {
	conn.execute("SELECT pg_advisory_lock($1)", &[&MIGRATION_LOCK]).await?;
	Ok(())
}

/**
 * Releases the lock, then returns `result`. The connection may go back to a pool, so this runs
 * after a failure too.
 */
async fn unlock(conn: &Client, result: anyhow::Result<()>) -> anyhow::Result<()> {
	let unlocked = conn.execute("SELECT pg_advisory_unlock($1)", &[&MIGRATION_LOCK]).await;
	result?;
	unlocked?;
	Ok(())
}

/** Applies the pending migrations in one transaction. */
async fn apply_pending(conn: &mut Client) -> anyhow::Result<()> {
	let tx = conn.transaction().await?;
	apply_in(&tx).await?;
	tx.commit().await?;
	Ok(())
}

/** Applies the migrations `indexer.migrations` doesn't list yet, in the caller's transaction. */
pub async fn apply_in(tx: &Transaction<'_>) -> anyhow::Result<()> {
	tx.batch_execute(
		"CREATE SCHEMA IF NOT EXISTS indexer;
		 CREATE TABLE IF NOT EXISTS indexer.migrations (
			name text PRIMARY KEY,
			applied_at timestamptz NOT NULL DEFAULT now()
		 );",
	)
	.await?;
	for (name, sql) in MIGRATIONS {
		let applied = tx
			.query_opt("SELECT 1 FROM indexer.migrations WHERE name = $1", &[name])
			.await?
			.is_some();
		if !applied {
			tracing::info!(migration = name, "applying");
			tx.batch_execute(sql).await?;
			tx.execute("INSERT INTO indexer.migrations (name) VALUES ($1)", &[name]).await?;
		}
	}
	Ok(())
}

/** The block each chain's cursor is at. */
pub async fn cursor_blocks(conn: &Client) -> anyhow::Result<BTreeMap<String, u64>> {
	let rows = conn.query("SELECT network, block_number FROM indexer.cursors", &[]).await?;
	Ok(rows.iter().map(|row| (row.get(0), row.get::<_, i64>(1) as u64)).collect())
}

/**
 * Claims the database for `mode`, or fails when it belongs to another mode. Each mode needs a
 * database of its own: the governance chain and the indexer's Nexus share a chain name, so both
 * would write the same metadata table. The first start records its mode under a lock, so starts
 * that race on a fresh database are refused as well. A database from before that record, or a
 * fresh import, is claimed only while no other mode's tables hold rows. SubQuery's bookkeeping
 * tables, named with a leading underscore, don't count.
 */
pub async fn claim_mode(tx: &Transaction<'_>, mode: Mode) -> anyhow::Result<()> {
	tx.execute("SELECT pg_advisory_xact_lock($1)", &[&MODE_LOCK]).await?;
	let name = mode.name();
	if let Some(row) = tx.query_opt("SELECT mode FROM indexer.mode", &[]).await? {
		let recorded: String = row.get(0);
		if recorded != name {
			bail!("this database belongs to a {recorded} config; a {name} config needs its own");
		}
		return Ok(());
	}
	let schemas: Vec<&str> =
		Mode::ALL.iter().filter(|other| **other != mode).map(Mode::schema).collect();
	let filled: Vec<String> = tx
		.query(
			"SELECT format('%I.%I', n.nspname, c.relname)
			 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
			 WHERE n.nspname = ANY($1) AND c.relkind = 'r' AND c.relname NOT LIKE '\\_%'
			   AND (xpath('/row/filled/text()', query_to_xml(
			     format('SELECT EXISTS (SELECT 1 FROM %I.%I) AS filled', n.nspname, c.relname),
			     false, true, '')))[1]::text = 'true'
			 ORDER BY 1",
			&[&schemas],
		)
		.await?
		.iter()
		.map(|row| row.get(0))
		.collect();
	if !filled.is_empty() {
		bail!("this database holds another config's rows in {filled:?}; a {name} config needs its own");
	}
	tx.execute("INSERT INTO indexer.mode (mode) VALUES ($1)", &[&name]).await?;
	Ok(())
}
