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

use codec::Decode;
use ismp::messaging::ConsensusMessage;
use proof_indexer::ProofIndexer;

#[async_trait::async_trait]
pub trait ConsensusProofProvider: Send + Sync {
	/// Returns a consensus proof that finalizes at least up to `target_height`,
	/// or None if no suitable proof is available yet.
	async fn get_proof(
		&self,
		target_height: u64,
	) -> Result<Option<ConsensusMessage>, anyhow::Error>;
}

/// V1: queries pre-generated ZK proofs from the indexer PostgreSQL.
pub struct IndexerProofProvider {
	indexer: ProofIndexer,
}

impl IndexerProofProvider {
	pub fn new(indexer: ProofIndexer) -> Self {
		Self { indexer }
	}
}

#[async_trait::async_trait]
impl ConsensusProofProvider for IndexerProofProvider {
	async fn get_proof(
		&self,
		target_height: u64,
	) -> Result<Option<ConsensusMessage>, anyhow::Error> {
		let proof = self.indexer.latest_proof().await?;
		match proof {
			Some(row) if row.finalized_height >= target_height as i64 => {
				let msg = ConsensusMessage::decode(&mut &row.consensus_proof[..])?;
				Ok(Some(msg))
			},
			_ => Ok(None),
		}
	}
}
