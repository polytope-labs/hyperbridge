use anyhow::Context;
use tokio_postgres::{Client, NoTls};

/// Connects to the database omniplex's governance config indexes into, applies the forum schema
/// and makes the governance tables resolvable without a prefix, with the chain's progress as
/// `_metadata`. The process exits if the connection drops, so the service supervisor restarts it
/// with a fresh one.
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
	// omniplex names a chain's metadata table after the chain, and a governance database
	// follows one chain
	let tables = client
		.query(
			"SELECT table_name FROM information_schema.tables
			 WHERE table_schema = 'app' AND table_name LIKE '\\_metadata\\_0x%'",
			&[],
		)
		.await?;
	let [table] = tables.as_slice() else {
		anyhow::bail!(
			"expected the metadata table of one chain in the app schema, found {}; is the \
			 governance indexer running against this database?",
			tables.len()
		);
	};
	let table: String = table.get(0);
	client
		.batch_execute(&format!(
			"CREATE OR REPLACE VIEW forum._metadata AS SELECT key, value FROM app.\"{table}\";
			SET search_path TO forum, governance"
		))
		.await
		.context("reading the governance indexer's metadata")?;
	Ok(client)
}
