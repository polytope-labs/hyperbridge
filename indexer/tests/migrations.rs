//! Indexer processes starting together on a fresh database. Set `INDEXER_TEST_DATABASE_URL` to run.

use tokio_postgres::{Client, Config, NoTls};

async fn connect(config: &Config) -> Client {
	let (client, connection) = config.connect(NoTls).await.unwrap();
	tokio::spawn(connection);
	client
}

#[tokio::test]
async fn processes_starting_together_each_apply_migrations() {
	let Ok(url) = std::env::var("INDEXER_TEST_DATABASE_URL") else { return };
	let config: Config = url.parse().unwrap();
	let admin = connect(&config).await;
	let name = format!("indexer_migrations_{}", std::process::id());
	admin.batch_execute(&format!("DROP DATABASE IF EXISTS {name}")).await.unwrap();
	admin.batch_execute(&format!("CREATE DATABASE {name}")).await.unwrap();
	let mut fresh = config.clone();
	fresh.dbname(&name);

	// One process per chain, all starting at once.
	let mut processes = Vec::new();
	for _ in 0..6 {
		let fresh = fresh.clone();
		processes.push(tokio::spawn(async move {
			let mut client = connect(&fresh).await;
			hyperbridge_indexer::db::apply_migrations(&mut client).await
		}));
	}
	let mut results = Vec::new();
	for process in processes {
		results.push(process.await.unwrap());
	}
	admin
		.batch_execute(&format!("DROP DATABASE {name} WITH (FORCE)"))
		.await
		.unwrap();
	for result in results {
		result.unwrap();
	}
}
