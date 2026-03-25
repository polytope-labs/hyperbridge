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
	/// Connect to the indexer PostgreSQL.
	/// The `app.zk_consensus_proofs` table is created by the SubQuery indexer
	/// from the `ZkConsensusProof` entity in `schema.graphql`.
	pub async fn initialize(database_url: &str) -> anyhow::Result<Self> {
		let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;

		// tokio_postgres requires the connection to be driven in a background task
		tokio::spawn(async move {
			if let Err(e) = connection.await {
				tracing::error!("PostgreSQL connection error: {e}");
			}
		});

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
					finalized_height, finalized_parachain_height,
					validator_set_id, created_at
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
					finalized_height, finalized_parachain_height,
					validator_set_id, created_at
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

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	#[ignore]
	async fn store_and_query_zk_proofs() {
		let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
		let indexer = ProofIndexer::initialize(&db_url).await.unwrap();

		let proof_bytes = vec![0x01, 0xaa, 0xbb, 0xcc];

		indexer.store_zk_proof(&proof_bytes, 1000, 500, 42).await.unwrap();
		indexer.store_zk_proof(&proof_bytes, 2000, 1000, 42).await.unwrap();
		indexer.store_zk_proof(&proof_bytes, 3000, 1500, 43).await.unwrap();

		// duplicate insert is a no-op
		indexer.store_zk_proof(&proof_bytes, 1000, 500, 42).await.unwrap();

		let latest = indexer.latest_proof().await.unwrap().unwrap();
		assert_eq!(latest.finalized_height, 3000);
		assert_eq!(latest.finalized_parachain_height, 1500);
		assert_eq!(latest.validator_set_id, 43);
		assert_eq!(latest.consensus_proof, proof_bytes);

		let since = indexer.proofs_since_height(2000).await.unwrap();
		assert_eq!(since.len(), 2);
		assert_eq!(since[0].finalized_height, 2000);
		assert_eq!(since[1].finalized_height, 3000);

		let none = indexer.proofs_since_height(5000).await.unwrap();
		assert!(none.is_empty());

		indexer
			.client
			.batch_execute("DELETE FROM app.zk_consensus_proofs")
			.await
			.unwrap();
	}
}
