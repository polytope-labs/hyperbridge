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
