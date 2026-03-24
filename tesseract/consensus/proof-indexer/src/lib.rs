// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;
use tokio_postgres::{Client, NoTls};

/// ZK consensus proof storage backed by the indexer PostgreSQL database.
///
/// Writes to `app.zk_consensus_proofs` — the same schema SubQuery's GraphQL
/// query service reads from, making proofs publicly queryable without extra
/// infrastructure.
#[derive(Clone)]
pub struct ProofIndexer {
	client: Arc<Client>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ZkProofRow {
	pub id: String,
	pub consensus_proof: Vec<u8>,
	pub finalized_height: i64,
	pub finalized_parachain_height: i64,
	pub validator_set_id: i64,
	pub created_at: chrono::NaiveDateTime,
}

impl ProofIndexer {
	/// Connect to the indexer PostgreSQL and ensure the proof table exists.
	/// Safe to call on every startup — all DDL is idempotent.
	pub async fn initialize(database_url: &str) -> anyhow::Result<Self> {
		let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;

		tokio::spawn(async move {
			if let Err(e) = connection.await {
				tracing::error!("PostgreSQL connection error: {e}");
			}
		});

		client.batch_execute("CREATE SCHEMA IF NOT EXISTS app").await?;

		// Table and column names follow SubQuery's convention:
		// PascalCase entity → snake_case table, camelCase fields → snake_case columns.
		// `id` is TEXT to match SubQuery's `ID!` type.
		// Height and set ID columns are NUMERIC to match SubQuery's `BigInt`.
		client
			.batch_execute(
				"CREATE TABLE IF NOT EXISTS app.zk_consensus_proofs (
				id                          TEXT PRIMARY KEY,
				consensus_proof             BYTEA NOT NULL,
				finalized_height            NUMERIC NOT NULL,
				finalized_parachain_height  NUMERIC NOT NULL,
				validator_set_id            NUMERIC NOT NULL,
				created_at                  TIMESTAMP NOT NULL DEFAULT NOW()
			);
			CREATE INDEX IF NOT EXISTS idx_zk_proofs_finalized_height
				ON app.zk_consensus_proofs(finalized_height);
			CREATE INDEX IF NOT EXISTS idx_zk_proofs_finalized_parachain_height
				ON app.zk_consensus_proofs(finalized_parachain_height);
			CREATE INDEX IF NOT EXISTS idx_zk_proofs_validator_set_id
				ON app.zk_consensus_proofs(validator_set_id);
			CREATE INDEX IF NOT EXISTS idx_zk_proofs_created_at
				ON app.zk_consensus_proofs(created_at);",
			)
			.await?;

		Ok(Self { client: Arc::new(client) })
	}

	pub async fn store_zk_proof(
		&self,
		consensus_proof: &[u8],
		finalized_height: u32,
		finalized_parachain_height: u64,
		validator_set_id: u64,
	) -> anyhow::Result<()> {
		let id = format!("{finalized_height}-{finalized_parachain_height}-{validator_set_id}");

		self.client
			.execute(
				"INSERT INTO app.zk_consensus_proofs
				(id, consensus_proof, finalized_height, finalized_parachain_height, validator_set_id)
			 VALUES ($1, $2, $3, $4, $5)
			 ON CONFLICT (id) DO NOTHING",
				&[
					&id,
					&consensus_proof,
					&(finalized_height as i64),
					&(finalized_parachain_height as i64),
					&(validator_set_id as i64),
				],
			)
			.await?;

		Ok(())
	}

	pub async fn latest_proof(&self) -> anyhow::Result<Option<ZkProofRow>> {
		let row = self
			.client
			.query_opt(
				"SELECT id, consensus_proof,
					finalized_height::BIGINT, finalized_parachain_height::BIGINT,
					validator_set_id::BIGINT, created_at
			 FROM app.zk_consensus_proofs
			 ORDER BY finalized_height DESC
			 LIMIT 1",
				&[],
			)
			.await?;

		Ok(row.map(row_to_proof))
	}

	pub async fn proofs_since_height(&self, height: i64) -> anyhow::Result<Vec<ZkProofRow>> {
		let rows = self
			.client
			.query(
				"SELECT id, consensus_proof,
					finalized_height::BIGINT, finalized_parachain_height::BIGINT,
					validator_set_id::BIGINT, created_at
			 FROM app.zk_consensus_proofs
			 WHERE finalized_height >= $1
			 ORDER BY finalized_height ASC",
				&[&height],
			)
			.await?;

		Ok(rows.into_iter().map(row_to_proof).collect())
	}
}

fn row_to_proof(r: tokio_postgres::Row) -> ZkProofRow {
	ZkProofRow {
		id: r.get(0),
		consensus_proof: r.get(1),
		finalized_height: r.get(2),
		finalized_parachain_height: r.get(3),
		validator_set_id: r.get(4),
		created_at: r.get(5),
	}
}
