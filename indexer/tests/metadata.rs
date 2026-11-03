//! The per-chain `_metadata` tables omnihedron serves. Set `INDEXER_TEST_DATABASE_URL` to run.

mod common;

use common::{database, text};
use hyperbridge_indexer::metadata::{record_progress, register, table};

#[tokio::test]
async fn each_chain_keeps_one_metadata_table_omnihedron_lists() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	register(&db, "test-chain", "Hyperbridge (Test)").await.unwrap();
	record_progress(&db, "test-chain", 100, 105).await.unwrap();
	// A restart registers again, and progress overwrites the last.
	register(&db, "test-chain", "Hyperbridge (Test)").await.unwrap();
	record_progress(&db, "test-chain", 104, 110).await.unwrap();

	let rows = text(
		&db,
		&format!(
			"SELECT string_agg(key || '=' || value::text, ',' ORDER BY key) FROM {}",
			table("test-chain")
		),
	)
	.await;
	assert_eq!(
		rows.as_deref(),
		Some(r#"chain="Hyperbridge (Test)",lastProcessedHeight=104,targetHeight=110"#)
	);

	// omnihedron serves the plain `_metadata` table and every `_metadata_0x…` one. A plain table
	// with no rows would add a node whose `chain` is null.
	let listed = text(
		&db,
		"SELECT string_agg(format('app.\"%s\"', relname), ',') FROM pg_class c
		 JOIN pg_namespace n ON n.oid = c.relnamespace
		 WHERE n.nspname = 'app' AND c.relkind = 'r'
		   AND (c.relname = '_metadata' OR c.relname LIKE '\\_metadata\\_0x%')",
	)
	.await;
	assert_eq!(listed, Some(table("test-chain")));
}
