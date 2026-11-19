//! The per-chain `_metadata` tables omnihedron serves. Set `INDEXER_TEST_DATABASE_URL` to run.

mod common;

use common::{database, text};
use omniplex::metadata::{record_progress, register, table};

#[tokio::test]
async fn each_chain_keeps_one_metadata_table_omnihedron_lists() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let chain = "Hyperbridge (Test)";
	register(&db, chain).await.unwrap();
	record_progress(&db, chain, 100, 105).await.unwrap();
	// A restart registers again, and progress overwrites the last.
	register(&db, chain).await.unwrap();
	record_progress(&db, chain, 104, 110).await.unwrap();

	let rows = text(
		&db,
		&format!(
			"SELECT string_agg(key || '=' || value::text, ',' ORDER BY key) FROM {}",
			table(chain)
		),
	)
	.await;
	assert_eq!(
		rows.as_deref(),
		Some(r#"chain="Hyperbridge (Test)",lastProcessedHeight=104,targetHeight=110"#)
	);

	// omnihedron serves every `_metadata_0x…` table, and the plain `_metadata` one first. With no
	// rows, a plain table would answer `_metadata` with nulls and add a node whose `chain` is null.
	let listed = text(
		&db,
		&format!(
			"SELECT count(*)::text FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
			 WHERE n.nspname = 'app' AND c.relkind = 'r' AND c.relname LIKE '\\_metadata\\_0x%'
			   AND format('app.\"%s\"', c.relname) = '{}'",
			table(chain)
		),
	)
	.await;
	assert_eq!(listed.as_deref(), Some("1"));
	let plain_absent = text(&db, "SELECT (to_regclass('app._metadata') IS NULL)::text").await;
	assert_eq!(plain_absent.as_deref(), Some("true"));
}
