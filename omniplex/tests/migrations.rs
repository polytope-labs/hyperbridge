//! Indexer processes starting together on a fresh database. Set `INDEXER_TEST_DATABASE_URL` to run.

use std::{
	collections::{BTreeMap, HashMap},
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use omniplex::{config::Config, db, handlers, manifest, registry};
use rindexer::{manifest::core::Manifest, PostgresClient};
use tokio_postgres::NoTls;

const CHAINS: [&str; 6] = [
	"hyperbridge-gargantua",
	"sepolia",
	"arbitrum-sepolia",
	"base-sepolia",
	"bsc-chapel",
	"polygon-amoy",
];

/** The rindexer manifest a process indexing only `chain` runs with. */
fn chain_manifest(config: &Config, chain: &str, dir: &Path) -> (PathBuf, Manifest) {
	let handlers = handlers::all();
	let routes = HashMap::from([(
		chain.to_string(),
		Arc::new(registry::ChainRoutes::new(
			&config.chains[chain],
			&handlers.subscriptions,
			&handlers.hooks,
		)),
	)]);
	let rpc_urls = BTreeMap::from([(chain.to_string(), "http://127.0.0.1:9".to_string())]);
	manifest::write(dir, config, &rpc_urls, &BTreeMap::new(), &routes).unwrap()
}

/** What each process does at startup: `db::migrate`, then `start_rindexer`'s unlocked setup. */
async fn start(db: &PostgresClient, path: &Path, manifest: &Manifest) -> anyhow::Result<()> {
	db::migrate(db, path, manifest).await?;
	rindexer::setup_postgres(path.parent().unwrap(), manifest).await?;
	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn processes_starting_together_each_set_up_the_database() {
	let Ok(url) = std::env::var("INDEXER_TEST_DATABASE_URL") else { return };
	let name = format!("indexer_setup_{}", std::process::id());
	let config = Config::load(Path::new("config/testnet.toml")).unwrap();
	let runs = std::env::temp_dir().join(&name);
	let manifests: Vec<_> = CHAINS
		.iter()
		.map(|chain| chain_manifest(&config, chain, &runs.join(chain)))
		.collect();

	let (admin, connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
	tokio::spawn(connection);
	admin.batch_execute(&format!("DROP DATABASE IF EXISTS {name}")).await.unwrap();
	admin.batch_execute(&format!("CREATE DATABASE {name}")).await.unwrap();
	// As in `main`: rindexer connects to `DATABASE_URL`, over rustls.
	let _ = rustls::crypto::ring::default_provider().install_default();
	std::env::set_var("DATABASE_URL", format!("{}/{name}", url.rsplit_once('/').unwrap().0));
	std::env::set_var("DATABASE_POOL_SIZE", "2");

	let together = Arc::new(tokio::sync::Barrier::new(CHAINS.len()));
	let mut processes = Vec::new();
	for (path, manifest) in manifests {
		let together = together.clone();
		processes.push(tokio::spawn(async move {
			let db = PostgresClient::new().await?;
			together.wait().await;
			let started = start(&db, &path, &manifest).await;
			// Then every process runs rindexer's setup at once. That only passes if `db::migrate`
			// created rindexer's tables. A process that failed still waits, so none hangs here.
			together.wait().await;
			started?;
			rindexer::setup_postgres(path.parent().unwrap(), &manifest).await?;
			anyhow::Ok(())
		}));
	}
	let mut results = Vec::new();
	for process in processes {
		results.push(tokio::time::timeout(Duration::from_secs(60), process).await);
	}
	// Drop the database before checking, so a failure leaves nothing behind.
	admin
		.batch_execute(&format!("DROP DATABASE {name} WITH (FORCE)"))
		.await
		.unwrap();
	let _ = std::fs::remove_dir_all(&runs);
	for (chain, result) in CHAINS.iter().zip(results) {
		if let Err(err) = result.expect("timed out").expect("panicked") {
			panic!("{chain}: {err:#}");
		}
	}
}
