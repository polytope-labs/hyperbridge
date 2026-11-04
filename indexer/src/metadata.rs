//! SubQuery's per-chain `_metadata` tables. omnihedron serves each `app._metadata_0x…` table as
//! one `_metadatas` node. The explorer matches a node to a network by `chain` and reads its
//! `lastProcessedHeight`. The SDK's CI waits until `lastProcessedHeight` is close to
//! `targetHeight`.

use alloy::{hex, primitives::keccak256};
use anyhow::{anyhow, Context};
use serde_json::{json, Value};
use tokio_postgres::Transaction;

use crate::{
	chains::Chain,
	config::{ChainConfig, ChainKind},
	substrate::SubstrateRpc,
};

/**
 * The metadata table for the chain named `chain`. omnihedron lists the tables whose name starts
 * `_metadata_0x`. Naming them after `chain` keeps one node per chain.
 */
pub fn table(chain: &str) -> String {
	format!("app.\"_metadata_0x{}\"", hex::encode(&keccak256(chain.as_bytes())[..8]))
}

/**
 * The chain's name as SubQuery wrote it: the EVM chain id, or a Substrate node's
 * `system_chain`.
 */
pub async fn chain_name(chain: &Chain) -> anyhow::Result<String> {
	let node = match chain.config.kind {
		ChainKind::Substrate => Some(chain.substrate()?),
		ChainKind::Evm => None,
	};
	name(&chain.config, node).await.with_context(|| chain.name.clone())
}

/** As [`chain_name`], from the chain's config and, for a Substrate chain, its node. */
pub async fn name(config: &ChainConfig, node: Option<&SubstrateRpc>) -> anyhow::Result<String> {
	match (config.kind, node) {
		(ChainKind::Evm, _) => Ok(config.chain_id.to_string()),
		(ChainKind::Substrate, Some(node)) => node.call("system_chain", json!([])).await,
		(ChainKind::Substrate, None) => Err(anyhow!("a Substrate chain's name needs its node")),
	}
}

/** Creates the metadata table for the chain named `chain`, and records the name in it. */
pub async fn register(db: &Transaction<'_>, chain: &str) -> anyhow::Result<()> {
	db.batch_execute(&format!(
		"CREATE TABLE IF NOT EXISTS {} (
			key character varying(255) NOT NULL PRIMARY KEY,
			value jsonb,
			\"createdAt\" timestamp with time zone NOT NULL,
			\"updatedAt\" timestamp with time zone NOT NULL
		)",
		table(chain)
	))
	.await?;
	write(db, chain, &[("chain", json!(chain))]).await
}

/** Records how far a chain is indexed, and the block indexing is heading for. */
pub async fn record_progress(
	db: &Transaction<'_>,
	chain: &str,
	processed: u64,
	target: u64,
) -> anyhow::Result<()> {
	write(db, chain, &[("lastProcessedHeight", json!(processed)), ("targetHeight", json!(target))])
		.await
}

async fn write(db: &Transaction<'_>, chain: &str, entries: &[(&str, Value)]) -> anyhow::Result<()> {
	let sql = format!(
		"INSERT INTO {} (key, value, \"createdAt\", \"updatedAt\") VALUES ($1, $2, now(), now())
		 ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, \"updatedAt\" = now()",
		table(chain)
	);
	for (key, value) in entries {
		db.execute(&sql, &[key, value]).await?;
	}
	Ok(())
}
