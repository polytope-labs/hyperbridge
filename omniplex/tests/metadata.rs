//! The per-chain `_metadata` tables omnihedron serves. Set `INDEXER_TEST_DATABASE_URL` to run.

mod common;

use std::time::Duration;

use common::{database, text};
use futures::{channel::mpsc::UnboundedReceiver, StreamExt};
use omniplex::{
	config::ChainConfig,
	metadata::{record_progress, register, table, table_for, GOVERNANCE_TABLE},
};
use tokio_postgres::{AsyncMessage, NoTls};

#[tokio::test]
async fn each_chain_keeps_one_metadata_table_omnihedron_lists() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let chain = "Hyperbridge (Test)";
	register(&db, &table(chain), chain).await.unwrap();
	record_progress(&db, &table(chain), 100, 105).await.unwrap();
	// A restart registers again, and progress overwrites the last.
	register(&db, &table(chain), chain).await.unwrap();
	record_progress(&db, &table(chain), 104, 110).await.unwrap();

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

/**
 * The governance entry follows Nexus like the indexer's entry, so it records its progress in
 * the governance schema and leaves the indexer's table, which omnihedron serves, alone.
 */
#[tokio::test]
async fn the_governance_entry_keeps_its_progress_apart_from_the_indexers() {
	let Some(mut client) = database().await else { return };
	let db = client.transaction().await.unwrap();
	let entry = |handlers: &str| -> ChainConfig {
		toml::from_str(&format!(
			"kind = \"substrate\"\nhandlers = \"{handlers}\"\nchain_id = 1\n\
			 state_machine_id = \"POLKADOT-3367\"\nstart_block = 0\nreorg_safe_distance = 0"
		))
		.unwrap()
	};
	let chain = "Hyperbridge (Governance Test)";
	let (indexer, governance) =
		(table_for(&entry("indexer"), chain), table_for(&entry("governance"), chain));
	assert_eq!((indexer.as_str(), governance.as_str()), (table(chain).as_str(), GOVERNANCE_TABLE));

	register(&db, &indexer, chain).await.unwrap();
	record_progress(&db, &indexer, 12_000_000, 12_000_010).await.unwrap();
	register(&db, &governance, chain).await.unwrap();
	record_progress(&db, &governance, 6_000_000, 12_000_010).await.unwrap();
	let height =
		|table: &str| format!("SELECT value::text FROM {table} WHERE key = 'lastProcessedHeight'");
	assert_eq!(text(&db, &height(&indexer)).await.as_deref(), Some("12000000"));
	assert_eq!(text(&db, &height(&governance)).await.as_deref(), Some("6000000"));
}

/** The next notification within two seconds. */
async fn next(notifications: &mut UnboundedReceiver<String>) -> Option<String> {
	tokio::time::timeout(Duration::from_secs(2), notifications.next())
		.await
		.ok()
		.flatten()
}

/** omnihedron reads the schema at startup, so a new chain's table is announced on the channel
 * omnihedron listens on, and only when the table is new. */
#[tokio::test]
async fn a_new_metadata_table_tells_omnihedron_to_reload() {
	let Ok(url) = std::env::var("INDEXER_TEST_DATABASE_URL") else { return };
	let Some(mut client) = database().await else { return };
	let (listener, mut connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	let (sender, mut notifications) = futures::channel::mpsc::unbounded();
	tokio::spawn(async move {
		while let Some(Ok(message)) =
			futures::future::poll_fn(|cx| connection.poll_message(cx)).await
		{
			if let AsyncMessage::Notification(notification) = message {
				let _ = sender.unbounded_send(notification.payload().to_string());
			}
		}
	});
	listener
		.batch_execute(
			r#"LISTEN "9d679a5c8ab87467a7e5a14e0e727eb318af1e19cdf781e2aafa793629461a4""#,
		)
		.await
		.unwrap();

	let chain = "Hyperbridge (Notify Test)";
	client
		.batch_execute(&format!("DROP TABLE IF EXISTS {}", table(chain)))
		.await
		.unwrap();

	let db = client.transaction().await.unwrap();
	register(&db, &table(chain), chain).await.unwrap();
	db.commit().await.unwrap();
	assert_eq!(next(&mut notifications).await.as_deref(), Some("schema_updated"));

	let db = client.transaction().await.unwrap();
	register(&db, &table(chain), chain).await.unwrap();
	db.commit().await.unwrap();
	assert_eq!(next(&mut notifications).await, None, "an existing table is not announced");

	client.batch_execute(&format!("DROP TABLE {}", table(chain))).await.unwrap();
}
