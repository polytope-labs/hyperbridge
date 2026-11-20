use anyhow::Context;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::{Client, Config, NoTls};

/// The connections requests share. Each has the forum and governance tables on its search path
/// and a statement timeout, so a slow query only holds up its own request.
pub type Pool = bb8::Pool<PostgresConnectionManager<NoTls>>;

const POOL_SIZE: u32 = 16;
const STATEMENT_TIMEOUT: &str = "10s";
const SEARCH_PATH: &str = "forum,governance";

/// A single connection with the forum schema applied, for startup and the `import` command. It has
/// no statement timeout, since importing a large forum can take a while.
pub async fn connect(url: &str) -> anyhow::Result<Client> {
	let mut config = url.parse::<Config>().context("parsing DATABASE_URL")?;
	config.options(format!("-c search_path={SEARCH_PATH}"));
	let (client, connection) = config.connect(NoTls).await.context("connecting to postgres")?;
	tokio::spawn(async move {
		if let Err(err) = connection.await {
			tracing::error!(target: crate::LOG_TARGET, "postgres connection closed: {err}");
		}
	});
	client
		.batch_execute(include_str!("../migrations/forum.sql"))
		.await
		.context("applying the forum schema")?;
	Ok(client)
}

/// Applies the forum schema once, then opens the pool requests use.
pub async fn pool(url: &str) -> anyhow::Result<Pool> {
	connect(url).await?;
	let mut config = url.parse::<Config>().context("parsing DATABASE_URL")?;
	config
		.options(format!("-c search_path={SEARCH_PATH} -c statement_timeout={STATEMENT_TIMEOUT}"));
	bb8::Pool::builder()
		.max_size(POOL_SIZE)
		// A broken connection is dropped when it is returned, so checking each one out again with a
		// query would only add a round trip to every request
		.test_on_check_out(false)
		.build(PostgresConnectionManager::new(config, NoTls))
		.await
		.context("opening the connection pool")
}

/// The metadata tables omniplex has created in the app schema. omniplex creates one on its first
/// progress update and names it after the chain, so there are none until then.
pub async fn metadata_tables(db: &Client) -> anyhow::Result<Vec<String>> {
	let tables = db
		.query(
			"SELECT table_name FROM information_schema.tables
			 WHERE table_schema = 'app' AND table_name LIKE '\\_metadata\\_0x%' ORDER BY table_name",
			&[],
		)
		.await?;
	Ok(tables.iter().map(|table| table.get(0)).collect())
}

/// Points `forum._metadata` at the governance indexer's metadata table. Unless `tables` holds
/// exactly one, the view has no rows and the indexed height reads as 0.
pub async fn link_metadata(db: &Client, tables: &[String]) -> anyhow::Result<()> {
	let source = match tables {
		[table] => format!("SELECT key, value FROM app.\"{table}\""),
		_ => {
			if tables.len() > 1 {
				tracing::error!(
					target: crate::LOG_TARGET,
					"found the metadata tables of {} chains in the app schema, the governance \
					 indexer needs a database of its own",
					tables.len()
				);
			}
			"SELECT NULL::varchar AS key, NULL::jsonb AS value WHERE FALSE".into()
		},
	};
	db.batch_execute(&format!(
		"DROP VIEW IF EXISTS forum._metadata; CREATE VIEW forum._metadata AS {source}"
	))
	.await
	.context("linking the governance indexer's metadata")
}
