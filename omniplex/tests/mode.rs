//! The indexer and a governance config never sharing a database, while the explorer shares with
//! either. Set `INDEXER_TEST_DATABASE_URL` to run.

mod common;

use omniplex::{config::Mode, db};
use tokio_postgres::NoTls;

#[tokio::test]
async fn a_database_with_another_modes_rows_is_refused() {
	let Some(mut client) = common::database().await else { return };
	let tx = client.transaction().await.unwrap();
	tx.batch_execute(
		"CREATE TABLE governance.probe (id int);
		 INSERT INTO governance.probe VALUES (1);
		 CREATE TABLE app.\"_metadata_0x0123\" (key text);
		 INSERT INTO app.\"_metadata_0x0123\" VALUES ('chain');",
	)
	.await
	.unwrap();
	let refused = db::claim_mode(&tx, Mode::Indexer).await.unwrap_err();
	assert!(refused.to_string().contains("governance.probe"), "{refused}");
	// SubQuery's metadata tables belong to no mode, so a governance config takes this database.
	db::claim_mode(&tx, Mode::Governance).await.unwrap();
	assert_eq!(
		common::text(&tx, "SELECT string_agg(mode, ',') FROM indexer.modes")
			.await
			.as_deref(),
		Some("governance")
	);
	db::claim_mode(&tx, Mode::Governance).await.unwrap();
	let refused = db::claim_mode(&tx, Mode::Indexer).await.unwrap_err();
	assert!(refused.to_string().contains("the governance config has run"), "{refused}");
}

#[tokio::test]
async fn the_explorer_shares_a_database_with_the_indexer() {
	let Some(mut client) = common::database().await else { return };
	let tx = client.transaction().await.unwrap();
	tx.batch_execute(
		"CREATE TABLE app.probe (id int);
		 INSERT INTO app.probe VALUES (1);
		 CREATE TABLE explorer.probe (id int);
		 INSERT INTO explorer.probe VALUES (1);",
	)
	.await
	.unwrap();
	db::claim_mode(&tx, Mode::Explorer).await.unwrap();
	db::claim_mode(&tx, Mode::Indexer).await.unwrap();
	db::claim_mode(&tx, Mode::Explorer).await.unwrap();
	let refused = db::claim_mode(&tx, Mode::Governance).await.unwrap_err();
	assert!(refused.to_string().contains("the indexer config has run"), "{refused}");
}

#[tokio::test(flavor = "multi_thread")]
async fn of_two_modes_starting_together_one_is_refused() {
	let Ok(url) = std::env::var("INDEXER_TEST_DATABASE_URL") else { return };
	let name = format!("indexer_mode_{}", std::process::id());
	let (admin, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	admin.batch_execute(&format!("DROP DATABASE IF EXISTS {name}")).await.unwrap();
	admin.batch_execute(&format!("CREATE DATABASE {name}")).await.unwrap();
	let fresh = format!("{}/{name}", url.rsplit_once('/').unwrap().0);

	let together = std::sync::Arc::new(tokio::sync::Barrier::new(2));
	let starts: Vec<_> = [Mode::Indexer, Mode::Governance]
		.into_iter()
		.map(|mode| {
			let (fresh, together) = (fresh.clone(), together.clone());
			tokio::spawn(async move {
				let (mut client, connection) = tokio_postgres::connect(&fresh, NoTls).await?;
				tokio::spawn(connection);
				db::apply_migrations(&mut client).await?;
				together.wait().await;
				let tx = client.transaction().await?;
				db::claim_mode(&tx, mode).await?;
				tx.commit().await?;
				anyhow::Ok(())
			})
		})
		.collect();
	let mut results = Vec::new();
	for start in starts {
		results.push(start.await.unwrap());
	}
	admin
		.batch_execute(&format!("DROP DATABASE {name} WITH (FORCE)"))
		.await
		.unwrap();
	assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1, "{results:?}");
}
