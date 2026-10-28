use anyhow::Context;
use sp_crypto_hashing::blake2_64;
use tokio_postgres::{Client, NoTls};

/// Where the indexer keeps the governance entities.
pub enum Indexer {
	/// A schema of its own, holding only the current rows and a `_metadata` table.
	Standalone { schema: String },
	/// The main deployment's multichain schema, where every row carries the block range it was
	/// current for and each chain has its own metadata table.
	Multichain { schema: String, genesis_hash: [u8; 32] },
}

/// Holds the views of the current governance rows when the indexer's schema is a multichain one.
const VIEWS_SCHEMA: &str = "governance_current";

/// Connects to the database the governance indexer writes, applies the forum schema and makes both
/// schemas resolvable without a prefix. The process exits if the connection drops, so the service
/// supervisor restarts it with a fresh one.
pub async fn connect(url: &str, indexer: &Indexer) -> anyhow::Result<Client> {
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
	let schema = match indexer {
		Indexer::Standalone { schema } => schema,
		Indexer::Multichain { schema, genesis_hash } => {
			create_views(&client, schema, genesis_hash).await?;
			VIEWS_SCHEMA
		},
	};
	client.batch_execute(&format!("SET search_path TO forum, \"{schema}\"")).await?;
	Ok(client)
}

/// Recreates the views that show the multichain schema's governance tables as a standalone schema:
/// only the rows current at the head, without the block range columns, and the chain's metadata as
/// `_metadata`. They are rebuilt on every start, so a restart picks up columns the indexer added.
async fn create_views(
	client: &Client,
	schema: &str,
	genesis_hash: &[u8; 32],
) -> anyhow::Result<()> {
	let tables = client
		.query(
			"SELECT table_name, string_agg(quote_ident(column_name), ', ' ORDER BY ordinal_position)
				FILTER (WHERE column_name NOT IN ('_id', '_block_range'))
			FROM information_schema.columns
			WHERE table_schema = $1 AND table_name LIKE 'governance\\_%'
			GROUP BY table_name",
			&[&schema],
		)
		.await?;
	if tables.is_empty() {
		anyhow::bail!("{schema} has no governance tables");
	}

	// Named as SubQuery's getMetadataTableName does
	let metadata = format!("_metadata_0x{}", hex::encode(blake2_64(genesis_hash)));
	let mut sql = format!(
		"BEGIN; DROP SCHEMA IF EXISTS {VIEWS_SCHEMA} CASCADE; CREATE SCHEMA {VIEWS_SCHEMA};
		CREATE VIEW {VIEWS_SCHEMA}._metadata AS SELECT key, value FROM \"{schema}\".\"{metadata}\";"
	);
	for table in tables {
		let (name, columns): (String, String) = (table.get(0), table.get(1));
		sql += &format!(
			"CREATE VIEW {VIEWS_SCHEMA}.\"{name}\" AS SELECT {columns} FROM \"{schema}\".\"{name}\"
			WHERE upper_inf(_block_range);"
		);
	}
	sql += "COMMIT;";
	client.batch_execute(&sql).await.context("creating the governance views")?;
	Ok(())
}
