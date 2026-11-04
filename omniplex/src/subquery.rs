//! Starting from a copy of SubQuery's database. SubQuery keeps every version of a row in its
//! `app` tables, keyed by `_id`, with the time each version held in `_block_range`. This
//! indexer keeps the current version, keyed by `id`, except in `solver_inventories`. The
//! import turns the copy into this indexer's database, and each chain continues after the last
//! block SubQuery indexed.

use std::{collections::BTreeMap, time::Duration};

use anyhow::{anyhow, bail, Context};
use serde_json::Value;
use tokio_postgres::{Client, Transaction};

use crate::{config::Config, db};

/** Where SubQuery's tables wait while the import copies them, before they are dropped. */
const OLD_SCHEMA: &str = "subquery";

/** What an import did. */
#[derive(Debug, Default)]
pub struct Imported {
	/** Each chain's cursor: the last block SubQuery indexed. */
	pub cursors: BTreeMap<String, u64>,
	/** The rows copied into each table. */
	pub rows: BTreeMap<String, u64>,
}

/**
 * Turns a copy of SubQuery's database into this indexer's, in one transaction, so a failure
 * leaves the copy as it was. `names` holds the name SubQuery's metadata gives each chain of
 * the config. `new_chains` are the chains SubQuery never indexed: they start at their start
 * block. Every other chain needs a height in SubQuery's metadata.
 */
pub async fn import(
	conn: &mut Client,
	config: &Config,
	names: &BTreeMap<String, String>,
	new_chains: &[String],
) -> anyhow::Result<Imported> {
	refuse_other_clients(conn).await?;
	let tx = conn.transaction().await?;
	tx.batch_execute("SET LOCAL statement_timeout = 0").await?;
	if flag(&tx, "SELECT to_regclass('indexer.migrations') IS NOT NULL").await? {
		bail!("this database is already the indexer's");
	}
	if !flag(
		&tx,
		"SELECT EXISTS (SELECT 1 FROM information_schema.columns
		   WHERE table_schema = 'app' AND column_name = '_block_range')",
	)
	.await?
	{
		bail!("schema app holds no SubQuery tables");
	}
	if flag(&tx, &format!("SELECT to_regnamespace('{OLD_SCHEMA}') IS NOT NULL")).await? {
		bail!("schema {OLD_SCHEMA} exists; the import needs the name for SubQuery's tables");
	}
	refuse_rewind(&tx).await?;

	let heights = subquery_heights(&tx).await?;
	let cursors = chain_cursors(config, names, new_chains, &heights)?;
	for (name, height) in &heights {
		if !names.values().any(|known| known == name) {
			tracing::info!(
				chain = name,
				height,
				"SubQuery indexed a chain the config doesn't have; its rows stay as they are"
			);
		}
	}

	tx.batch_execute(&format!("ALTER SCHEMA app RENAME TO {OLD_SCHEMA}")).await?;
	// An extension in SubQuery's schema would go when it is dropped, with the indexes that use it.
	let extensions = tx
		.query(
			&format!(
				"SELECT extname::text FROM pg_extension WHERE extnamespace = '{OLD_SCHEMA}'::regnamespace"
			),
			&[],
		)
		.await?;
	for row in extensions {
		let extension: String = row.get(0);
		tx.batch_execute(&format!("ALTER EXTENSION \"{extension}\" SET SCHEMA public"))
			.await?;
	}
	db::apply_in(&tx).await?;

	// SubQuery's own tables start with `_`, and go with its schema.
	let tables = entity_tables(&tx, "app").await?;
	let only_subquery: Vec<String> = entity_tables(&tx, OLD_SCHEMA)
		.await?
		.into_iter()
		.filter(|table| !tables.contains(table))
		.collect();
	if !only_subquery.is_empty() {
		bail!("only SubQuery has the tables {only_subquery:?}, which the import would drop");
	}
	let mut rows = BTreeMap::new();
	for table in tables {
		let copied = copy_table(&tx, &table).await.with_context(|| format!("copying {table}"))?;
		rows.insert(table, copied);
	}
	for (network, block) in &cursors {
		tx.execute(
			"INSERT INTO indexer.cursors (network, block_number, log_index) VALUES ($1, $2, $3)",
			&[network, &(*block as i64), &i64::MAX],
		)
		.await?;
	}
	tx.batch_execute(&format!("DROP SCHEMA {OLD_SCHEMA} CASCADE")).await?;
	tx.commit().await?;
	Ok(Imported { cursors, rows })
}

/**
 * Fails while another client is connected: the import runs on a copy, never on the database
 * SubQuery writes to. A session that just closed can linger for a moment, so this looks three
 * times, a second apart.
 */
async fn refuse_other_clients(conn: &Client) -> anyhow::Result<()> {
	let mut others = 0;
	for _ in 0..3 {
		others = conn
			.query_one(
				"SELECT count(*) FROM pg_stat_activity
				 WHERE datname = current_database() AND pid <> pg_backend_pid()
				   AND backend_type = 'client backend'",
				&[],
			)
			.await?
			.get::<_, i64>(0);
		if others == 0 {
			return Ok(());
		}
		tokio::time::sleep(Duration::from_secs(1)).await;
	}
	bail!(
		"{others} other clients are connected to this database. Import a copy of SubQuery's \
		 database that nothing else uses"
	)
}

/** Fails on a copy taken while SubQuery was rewinding its chains together, which is half done. */
async fn refuse_rewind(tx: &Transaction<'_>) -> anyhow::Result<()> {
	if !flag(tx, "SELECT to_regclass('app._global') IS NOT NULL").await? {
		return Ok(());
	}
	if flag(tx, "SELECT EXISTS (SELECT 1 FROM app._global WHERE status::text <> 'complete')")
		.await?
	{
		bail!("SubQuery was rewinding when this copy was taken; take another");
	}
	Ok(())
}

/**
 * The last block SubQuery indexed on each chain, by the chain's name in its `_metadata`
 * tables. Fails unless SubQuery kept history by timestamp, as `solver_inventories` does here.
 */
async fn subquery_heights(tx: &Transaction<'_>) -> anyhow::Result<BTreeMap<String, u64>> {
	let tables = tx
		.query(
			"SELECT table_name::text FROM information_schema.tables
			 WHERE table_schema = 'app' AND table_name LIKE '\\_metadata%'",
			&[],
		)
		.await?;
	let mut heights = BTreeMap::new();
	for row in tables {
		let table: String = row.get(0);
		let entries: BTreeMap<String, Value> = tx
			.query(
				&format!(
					"SELECT key::text, value FROM app.\"{table}\"
					 WHERE key IN ('chain', 'lastProcessedHeight', 'historicalStateEnabled')"
				),
				&[],
			)
			.await?
			.iter()
			.map(|row| (row.get(0), row.get(1)))
			.collect();
		if let Some(history) = entries.get("historicalStateEnabled") {
			if history.as_str() != Some("timestamp") {
				bail!("{table}: SubQuery keeps history by {history}, not by timestamp");
			}
		}
		let (Some(chain), Some(height)) = (
			entries.get("chain").and_then(Value::as_str),
			entries.get("lastProcessedHeight").and_then(Value::as_u64),
		) else {
			tracing::warn!(table, "SubQuery metadata without a chain and a height; skipped");
			continue;
		};
		if heights.insert(chain.to_string(), height).is_some() {
			bail!("two SubQuery metadata tables name the chain {chain}");
		}
	}
	Ok(heights)
}

/**
 * Each chain's cursor at SubQuery's height. A chain without a height must be one of
 * `new_chains`: started at its start block on data SubQuery already indexed, it would count
 * everything twice.
 */
fn chain_cursors(
	config: &Config,
	names: &BTreeMap<String, String>,
	new_chains: &[String],
	heights: &BTreeMap<String, u64>,
) -> anyhow::Result<BTreeMap<String, u64>> {
	if let Some(unknown) = new_chains.iter().find(|chain| !config.chains.contains_key(*chain)) {
		bail!("--new-chains: {unknown} is not a chain of the config");
	}
	let mut cursors = BTreeMap::new();
	let mut missing = vec![];
	for network in config.chains.keys() {
		let name = names.get(network).ok_or_else(|| anyhow!("no SubQuery name for {network}"))?;
		match (heights.get(name), new_chains.contains(network)) {
			(Some(height), false) => {
				cursors.insert(network.clone(), *height);
			},
			(Some(height), true) =>
				bail!("SubQuery indexed {network} up to block {height}, so it isn't new"),
			(None, true) => {},
			(None, false) => missing.push(network.as_str()),
		}
	}
	if !missing.is_empty() {
		let known: Vec<&String> = heights.keys().collect();
		bail!(
			"SubQuery's metadata has no height for {missing:?}; it has {known:?}. A chain \
			 SubQuery never indexed starts at its start block once named in --new-chains"
		);
	}
	Ok(cursors)
}

/** A schema's tables, apart from the internal ones whose names start with `_`. */
async fn entity_tables(tx: &Transaction<'_>, schema: &str) -> anyhow::Result<Vec<String>> {
	Ok(tx
		.query(
			"SELECT table_name::text FROM information_schema.tables
			 WHERE table_schema = $1 AND table_type = 'BASE TABLE' AND table_name NOT LIKE '\\_%'
			 ORDER BY 1",
			&[&schema],
		)
		.await?
		.iter()
		.map(|row| row.get(0))
		.collect())
}

/** A column of a table in one of the two schemas. */
struct Column {
	name: String,
	data_type: String,
	udt_name: String,
}

async fn columns(tx: &Transaction<'_>, schema: &str, table: &str) -> anyhow::Result<Vec<Column>> {
	Ok(tx
		.query(
			"SELECT column_name::text, data_type::text, udt_name::text FROM information_schema.columns
			 WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
			&[&schema, &table],
		)
		.await?
		.iter()
		.map(|row| Column { name: row.get(0), data_type: row.get(1), udt_name: row.get(2) })
		.collect())
}

/**
 * Copies a table's rows from SubQuery's schema: every version where this indexer keeps the
 * history too, else each row's current version. Fails when the two tables' columns differ,
 * other than SubQuery's history columns, since the copy would lose or invent values.
 */
async fn copy_table(tx: &Transaction<'_>, table: &str) -> anyhow::Result<u64> {
	let target = columns(tx, "app", table).await?;
	let source = columns(tx, OLD_SCHEMA, table).await?;
	if source.is_empty() {
		bail!("SubQuery has no such table");
	}
	let has = |columns: &[Column], name: &str| columns.iter().any(|column| column.name == name);
	let history = |name: &str| name == "_id" || name == "_block_range";
	let only_subquery: Vec<&str> = source
		.iter()
		.filter(|column| !history(&column.name) && !has(&target, &column.name))
		.map(|column| column.name.as_str())
		.collect();
	let only_here: Vec<&str> = target
		.iter()
		.filter(|column| !has(&source, &column.name))
		.map(|column| column.name.as_str())
		.collect();
	if !only_subquery.is_empty() || !only_here.is_empty() {
		bail!("columns differ: only SubQuery has {only_subquery:?}, only this indexer has {only_here:?}");
	}

	let names: Vec<String> = target.iter().map(|column| format!("\"{}\"", column.name)).collect();
	// Enums are types of their schema, so SubQuery's values go through text.
	let values: Vec<String> = target
		.iter()
		.map(|column| match column.data_type.as_str() {
			"USER-DEFINED" => format!("s.\"{}\"::text::app.\"{}\"", column.name, column.udt_name),
			_ => format!("s.\"{}\"", column.name),
		})
		.collect();
	let (names, values) = (names.join(", "), values.join(", "));
	let current_only = has(&source, "_block_range") && !has(&target, "_block_range");
	let sql =
		if current_only {
			format!(
				"INSERT INTO app.\"{table}\" ({names})
			 SELECT DISTINCT ON (s.id) {values} FROM {OLD_SCHEMA}.\"{table}\" s
			 WHERE upper_inf(s._block_range) ORDER BY s.id, lower(s._block_range) DESC"
			)
		} else {
			format!("INSERT INTO app.\"{table}\" ({names}) SELECT {values} FROM {OLD_SCHEMA}.\"{table}\" s")
		};
	Ok(tx.execute(&sql, &[]).await?)
}

async fn flag(tx: &Transaction<'_>, sql: &str) -> anyhow::Result<bool> {
	Ok(tx.query_one(sql, &[]).await?.get(0))
}
