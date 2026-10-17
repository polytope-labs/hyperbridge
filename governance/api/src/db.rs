use anyhow::Context;
use tokio_postgres::{Client, NoTls};

/// Connects to the database the governance indexer writes, applies the forum schema and makes both
/// schemas resolvable without a prefix. The process exits if the connection drops, so the service
/// supervisor restarts it with a fresh one.
pub async fn connect(url: &str) -> anyhow::Result<Client> {
	let (client, connection) =
		tokio_postgres::connect(url, NoTls).await.context("connecting to postgres")?;
	tokio::spawn(async move {
		if let Err(err) = connection.await {
			tracing::error!(target: crate::LOG_TARGET, "postgres connection closed: {err}");
		}
		std::process::exit(1);
	});

	client
		.batch_execute(include_str!("../migrations/forum.sql"))
		.await
		.context("applying the forum schema")?;
	client.batch_execute("SET search_path TO forum, governance").await?;
	Ok(client)
}
