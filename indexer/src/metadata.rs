//! SubQuery's per-chain `_metadata` tables. omnihedron serves each `app._metadata_0x…` table as
//! one `_metadatas` node. The explorer matches a node to a network by `chain` and reads its
//! `lastProcessedHeight`. The SDK's CI waits until `lastProcessedHeight` is close to
//! `targetHeight`.

use alloy::{hex, primitives::keccak256};
use anyhow::Context;
use serde_json::{json, Value};
use tokio_postgres::Transaction;

use crate::{chains::Chain, config::ChainKind};

/** A chain's metadata table. omnihedron lists the tables whose name starts `_metadata_0x`. */
pub fn table(network: &str) -> String {
	format!("app.\"_metadata_0x{}\"", hex::encode(&keccak256(network.as_bytes())[..8]))
}

/**
 * The chain's name as SubQuery wrote it: the EVM chain id, or a Substrate node's
 * `system_chain`.
 */
pub async fn chain_name(chain: &Chain) -> anyhow::Result<String> {
	match chain.config.kind {
		ChainKind::Evm => Ok(chain.config.chain_id.to_string()),
		ChainKind::Substrate => chain
			.substrate()?
			.call("system_chain", json!([]))
			.await
			.with_context(|| format!("{}: system_chain", chain.name)),
	}
}

/** Creates a chain's metadata table and records its name. */
pub async fn register(db: &Transaction<'_>, network: &str, name: &str) -> anyhow::Result<()> {
	db.batch_execute(&format!(
		"CREATE TABLE IF NOT EXISTS {} (
			key character varying(255) NOT NULL PRIMARY KEY,
			value jsonb,
			\"createdAt\" timestamp with time zone NOT NULL,
			\"updatedAt\" timestamp with time zone NOT NULL
		)",
		table(network)
	))
	.await?;
	write(db, network, &[("chain", json!(name))]).await
}

/** Records how far a chain is indexed, and the block indexing is heading for. */
pub async fn record_progress(
	db: &Transaction<'_>,
	network: &str,
	processed: u64,
	target: u64,
) -> anyhow::Result<()> {
	write(
		db,
		network,
		&[("lastProcessedHeight", json!(processed)), ("targetHeight", json!(target))],
	)
	.await
}

async fn write(
	db: &Transaction<'_>,
	network: &str,
	entries: &[(&str, Value)],
) -> anyhow::Result<()> {
	let sql = format!(
		"INSERT INTO {} (key, value, \"createdAt\", \"updatedAt\") VALUES ($1, $2, now(), now())
		 ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, \"updatedAt\" = now()",
		table(network)
	);
	for (key, value) in entries {
		db.execute(&sql, &[key, value]).await?;
	}
	Ok(())
}
