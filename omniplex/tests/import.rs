//! Importing a copy of SubQuery's database. Set `INDEXER_TEST_DATABASE_URL` to run.

use std::collections::BTreeMap;

use omniplex::{config::Config, subquery};
use tokio_postgres::{Client, NoTls};

async fn connect(url: &str) -> Client {
	let (client, connection) = tokio_postgres::connect(url, NoTls).await.unwrap();
	tokio::spawn(connection);
	client
}

/**
 * Lays a schema's tables out as SubQuery keeps them: the same tables and columns, with every
 * row version keyed by `_id` over a `_block_range`.
 */
fn versioned(schema: &str) -> String {
	format!(
		r#"
DO $$
DECLARE t record; pk text;
BEGIN
	FOR t IN SELECT table_name FROM information_schema.tables
		WHERE table_schema = '{schema}' AND table_type = 'BASE TABLE'
		  AND table_name <> 'solver_inventories'
	LOOP
		SELECT conname INTO pk FROM pg_constraint
			WHERE conrelid = format('{schema}.%I', t.table_name)::regclass AND contype = 'p';
		EXECUTE format('ALTER TABLE {schema}.%I DROP CONSTRAINT %I', t.table_name, pk);
		EXECUTE format('ALTER TABLE {schema}.%I
			ADD COLUMN _id uuid NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
			ADD COLUMN _block_range int8range NOT NULL', t.table_name);
	END LOOP;
END $$;
"#
	)
}

/** SubQuery's own `_metadata_0x…` and `_global` tables, and rows in its layout of `app`. */
const SUBQUERY_LAYOUT: &str = r#"
CREATE TYPE app.enum__global_status AS ENUM ('complete', 'incomplete');
CREATE TABLE app._global (
	"chainId" character varying(255) PRIMARY KEY,
	"rewindTimestamp" timestamp with time zone,
	status app.enum__global_status,
	initiator boolean,
	"createdAt" timestamp with time zone NOT NULL DEFAULT now(),
	"updatedAt" timestamp with time zone NOT NULL DEFAULT now()
);
INSERT INTO app._global ("chainId", status) VALUES ('56', 'complete');

CREATE TABLE app."_metadata_0xaaaa" (key character varying(255) PRIMARY KEY, value jsonb,
	"createdAt" timestamp with time zone NOT NULL DEFAULT now(),
	"updatedAt" timestamp with time zone NOT NULL DEFAULT now());
CREATE TABLE app."_metadata_0xbbbb" (LIKE app."_metadata_0xaaaa" INCLUDING ALL);
CREATE TABLE app."_metadata_0xcccc" (LIKE app."_metadata_0xaaaa" INCLUDING ALL);
INSERT INTO app."_metadata_0xaaaa" (key, value) VALUES
	('chain', '"56"'), ('lastProcessedHeight', '1234'), ('historicalStateEnabled', '"timestamp"');
INSERT INTO app."_metadata_0xbbbb" (key, value) VALUES
	('chain', '"Hyperbridge (Nexus)"'), ('lastProcessedHeight', '5678'),
	('historicalStateEnabled', '"timestamp"');
-- A chain the config doesn't have.
INSERT INTO app."_metadata_0xcccc" (key, value) VALUES
	('chain', '"2030"'), ('lastProcessedHeight', '99'), ('historicalStateEnabled', '"timestamp"');

-- One status, overwritten at 2000, and one removed at 1500.
INSERT INTO app.request_status_metadata
	(id, status, chain, "timestamp", block_number, block_hash, transaction_hash, request_id,
	 created_at, _block_range)
VALUES
	('0xc.SOURCE', 'SOURCE', 'EVM-1', 1, '10', '0xb', '0xt', '0xc', '2026-11-01', '[1000,2000)'),
	('0xc.SOURCE', 'SOURCE', 'EVM-56', 2, '11', '0xb', '0xt', '0xc', '2026-11-01', '[2000,)'),
	('0xgone.SOURCE', 'SOURCE', 'EVM-1', 1, '10', '0xb', '0xt', '0xgone', '2026-11-01',
	 '[1000,1500)');

-- An inventory whose history simplex reads back in time.
INSERT INTO app.solver_inventories
	(id, chain, solver, token_address, balance, wallet, vaults, vault_shares, block_number,
	 last_log_index, observed_at, refreshed_at, genesis_block, last_read_block, discovered_by,
	 _id, _block_range)
VALUES
	('inv', 'EVM-56', '0xs', '0xt', 5, 5, 0, 0, 10, 0, '2026-11-01', '2026-11-01', 1, 10, 'FILL',
	 gen_random_uuid(), '[1000,2000)'),
	('inv', 'EVM-56', '0xs', '0xt', 7, 7, 0, 0, 11, 0, '2026-11-01', '2026-11-01', 1, 11, 'FILL',
	 gen_random_uuid(), '[2000,)');
"#;

async fn text(db: &Client, sql: &str) -> Option<String> {
	db.query_opt(sql, &[]).await.unwrap().and_then(|row| row.get(0))
}

/**
 * The import keeps each row's current version, keeps `solver_inventories`' history, and starts
 * each chain after SubQuery's height. It refuses a live database, a copy taken mid-rewind and a
 * chain SubQuery has no height for, and leaves the copy as it was when it does.
 */
#[tokio::test]
async fn a_subquery_copy_becomes_the_indexers_database() {
	let Ok(url) = std::env::var("INDEXER_TEST_DATABASE_URL") else { return };
	let name = format!("indexer_import_{}", std::process::id());
	let admin = connect(&url).await;
	admin.batch_execute(&format!("DROP DATABASE IF EXISTS {name}")).await.unwrap();
	admin.batch_execute(&format!("CREATE DATABASE {name}")).await.unwrap();
	let copy_url = format!("{}/{name}", url.rsplit_once('/').unwrap().0);
	let mut db = connect(&copy_url).await;
	db.batch_execute(include_str!("../migrations/0001_app_schema.sql"))
		.await
		.unwrap();
	db.batch_execute(&versioned("app")).await.unwrap();
	db.batch_execute(SUBQUERY_LAYOUT).await.unwrap();

	let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
	let mut config = Config::load(&root.join("config/mainnet.toml")).unwrap();
	config.chains.retain(|network, _| {
		["bsc-mainnet", "hyperbridge-nexus", "polkadot-hub-mainnet"].contains(&network.as_str())
	});
	let names: BTreeMap<String, String> = [
		("bsc-mainnet", "56"),
		("hyperbridge-nexus", "Hyperbridge (Nexus)"),
		("polkadot-hub-mainnet", "420420419"),
	]
	.into_iter()
	.map(|(network, name)| (network.to_string(), name.to_string()))
	.collect();
	let new = vec!["polkadot-hub-mainnet".to_string()];

	let other = connect(&copy_url).await;
	let err = subquery::import(&mut db, &config, &names, &new).await.unwrap_err();
	assert!(err.to_string().contains("other clients"), "{err:#}");
	drop(other);

	db.batch_execute("UPDATE app._global SET status = 'incomplete'").await.unwrap();
	let err = subquery::import(&mut db, &config, &names, &new).await.unwrap_err();
	assert!(err.to_string().contains("rewinding"), "{err:#}");
	db.batch_execute("UPDATE app._global SET status = 'complete'").await.unwrap();

	// Started at its start block, polkadot-hub-mainnet would index everything SubQuery did again.
	let err = subquery::import(&mut db, &config, &names, &[]).await.unwrap_err();
	assert!(err.to_string().contains("polkadot-hub-mainnet"), "{err:#}");
	let untouched = text(
		&db,
		"SELECT (to_regclass('indexer.migrations') IS NULL AND to_regnamespace('subquery') IS NULL
		   AND (SELECT count(*) FROM app.request_status_metadata) = 3)::text",
	)
	.await;
	assert_eq!(untouched.as_deref(), Some("true"));

	// Dropping SubQuery's schema would take this view with it.
	db.batch_execute("CREATE VIEW public.statuses AS SELECT id FROM app.request_status_metadata")
		.await
		.unwrap();
	let err = subquery::import(&mut db, &config, &names, &new).await.unwrap_err();
	assert!(err.to_string().contains("public.statuses"), "{err:#}");
	let untouched = text(&db, "SELECT (to_regclass('indexer.migrations') IS NULL)::text").await;
	assert_eq!(untouched.as_deref(), Some("true"));
	db.batch_execute("DROP VIEW public.statuses").await.unwrap();

	let imported = subquery::import(&mut db, &config, &names, &new).await.unwrap();
	assert_eq!(
		imported.cursors,
		BTreeMap::from([
			("bsc-mainnet".to_string(), 1234),
			("hyperbridge-nexus".to_string(), 5678)
		])
	);
	assert_eq!(imported.rows["request_status_metadata"], 1);
	let statuses = text(
		&db,
		"SELECT string_agg(id || ' ' || status || ' ' || chain, ',') FROM app.request_status_metadata",
	)
	.await;
	assert_eq!(statuses.as_deref(), Some("0xc.SOURCE SOURCE EVM-56"));
	let inventory = text(
		&db,
		"SELECT string_agg(balance || ' ' || _block_range::text, ',' ORDER BY lower(_block_range))
		 FROM app.solver_inventories",
	)
	.await;
	assert_eq!(inventory.as_deref(), Some("5 [1000,2000),7 [2000,)"));
	let cursors = text(
		&db,
		"SELECT string_agg(network || ' ' || block_number || ' ' || log_index, ',' ORDER BY network)
		 FROM indexer.cursors",
	)
	.await;
	let whole_block = i64::MAX;
	assert_eq!(
		cursors,
		Some(format!("bsc-mainnet 1234 {whole_block},hyperbridge-nexus 5678 {whole_block}"))
	);
	let migrations =
		text(&db, "SELECT string_agg(name, ',' ORDER BY name) FROM indexer.migrations").await;
	assert_eq!(
		migrations.as_deref(),
		Some(
			"0000_indexer,0001_app_schema,0002_deferred_fill_points,0003_explorer,0004_governance,\
			 0005_mode"
		)
	);
	// SubQuery's own tables went with its schema, and every table is keyed by `id` again.
	let leftovers = text(
		&db,
		"SELECT (to_regnamespace('subquery') IS NULL AND NOT EXISTS (
		   SELECT 1 FROM information_schema.tables WHERE table_schema = 'app' AND table_name LIKE '\\_%'
		 ) AND NOT EXISTS (
		   SELECT 1 FROM information_schema.columns WHERE table_schema = 'app'
		     AND column_name = '_block_range' AND table_name <> 'solver_inventories'
		 ))::text",
	)
	.await;
	assert_eq!(leftovers.as_deref(), Some("true"));

	let err = subquery::import(&mut db, &config, &names, &new).await.unwrap_err();
	assert!(err.to_string().contains("already the indexer's"), "{err:#}");

	drop(db);
	admin
		.batch_execute(&format!("DROP DATABASE {name} WITH (FORCE)"))
		.await
		.unwrap();
}

/** SubQuery's governance project: its `governance` schema with its own metadata, a referendum
 * whose metadata was cleared after it was executed, and that referendum's events. */
const SUBQUERY_GOVERNANCE: &str = r#"
CREATE TABLE governance."_metadata" (key character varying(255) PRIMARY KEY, value jsonb,
	"createdAt" timestamp with time zone NOT NULL DEFAULT now(),
	"updatedAt" timestamp with time zone NOT NULL DEFAULT now());
INSERT INTO governance."_metadata" (key, value) VALUES
	('chain', '"Hyperbridge (Nexus)"'), ('lastProcessedHeight', '12249395'),
	('historicalStateEnabled', '"timestamp"');

INSERT INTO governance.governance_referendums
	(id, index, track, proposal, state, is_final, ayes, nays, support, submitted_block,
	 submitted_at, updated_block, _block_range)
VALUES
	('15', 15, 1, '{}', 'Executed', true, 9, 1, 9, 100, '2026-01-01', 150, '[150,200)'),
	('15', 15, 1, '{}', 'MetadataCleared', true, 9, 1, 9, 100, '2026-01-01', 200, '[200,)');

INSERT INTO governance.governance_referendum_events
	(id, referendum_id, name, block_number, block_hash, event_index, "timestamp", changes_state,
	 _block_range)
VALUES
	('150-1', '15', 'Executed', 150, '0xb', 1, '2026-01-01', true, '[150,)'),
	('200-1', '15', 'MetadataCleared', 200, '0xb', 1, '2026-01-01', true, '[200,)');
"#;

/**
 * A governance config imports SubQuery's governance database: the current rows of its
 * `governance` schema, with the governance chain continuing after its height. A referendum
 * keeps the state its metadata events overwrote there.
 */
#[tokio::test]
async fn a_subquery_governance_copy_becomes_the_governance_indexers_database() {
	let Ok(url) = std::env::var("INDEXER_TEST_DATABASE_URL") else { return };
	let name = format!("governance_import_{}", std::process::id());
	let admin = connect(&url).await;
	admin.batch_execute(&format!("DROP DATABASE IF EXISTS {name}")).await.unwrap();
	admin.batch_execute(&format!("CREATE DATABASE {name}")).await.unwrap();
	let copy_url = format!("{}/{name}", url.rsplit_once('/').unwrap().0);
	let mut db = connect(&copy_url).await;
	db.batch_execute(include_str!("../migrations/0004_governance.sql"))
		.await
		.unwrap();
	db.batch_execute(&versioned("governance")).await.unwrap();
	db.batch_execute(SUBQUERY_GOVERNANCE).await.unwrap();

	let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
	let config = Config::load(&root.join("config/governance-mainnet.toml")).unwrap();
	let names =
		BTreeMap::from([("nexus-governance".to_string(), "Hyperbridge (Nexus)".to_string())]);
	let imported = subquery::import(&mut db, &config, &names, &[]).await.unwrap();
	assert_eq!(imported.cursors, BTreeMap::from([("nexus-governance".to_string(), 12249395)]));
	assert_eq!(imported.rows["governance_referendums"], 1);

	let referendum =
		text(&db, "SELECT state || ' ' || updated_block FROM governance.governance_referendums")
			.await;
	assert_eq!(referendum.as_deref(), Some("Executed 200"));
	let events = text(
		&db,
		"SELECT string_agg(name || ' ' || changes_state, ',' ORDER BY block_number)
		 FROM governance.governance_referendum_events",
	)
	.await;
	assert_eq!(events.as_deref(), Some("Executed true,MetadataCleared false"));
	let cursors =
		text(&db, "SELECT string_agg(network || ' ' || block_number, ',') FROM indexer.cursors")
			.await;
	assert_eq!(cursors.as_deref(), Some("nexus-governance 12249395"));
	let leftovers = text(&db, "SELECT (to_regnamespace('subquery') IS NULL)::text").await;
	assert_eq!(leftovers.as_deref(), Some("true"));

	drop(db);
	admin
		.batch_execute(&format!("DROP DATABASE {name} WITH (FORCE)"))
		.await
		.unwrap();
}
