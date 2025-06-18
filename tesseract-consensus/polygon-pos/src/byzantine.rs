// Copyright (C) 2023 Polytope Labs.
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

use anyhow::anyhow;
use codec::{Decode, Encode};
use ethers::prelude::Middleware;
use geth_primitives::CodecHeader;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::StateMachineUpdated,
	messaging::ConsensusMessage,
};
use tesseract_primitives::{ByzantineHandler, IsmpHost, IsmpProvider};

use crate::PolygonPosHost;

#[async_trait::async_trait]
impl ByzantineHandler for PolygonPosHost {
	async fn query_consensus_message(
		&self,
		event: StateMachineUpdated,
	) -> Result<ConsensusMessage, anyhow::Error> {
		let header = self
			.prover
			.fetch_header(event.latest_height)
			.await?
			.ok_or_else(|| anyhow!("Consensus update header not found"))?;

		Ok(ConsensusMessage {
			consensus_proof: header.encode(),
			consensus_state_id: self.consensus_state_id,
		})
	}

	async fn check_for_byzantine_attack<C: IsmpHost + IsmpProvider>(
		&self,
		counterparty: &C,
		consensus_message: ConsensusMessage,
	) -> Result<(), anyhow::Error> {
		let header = CodecHeader::decode(&mut &*consensus_message.consensus_proof)?;
		let uncle_count =
			self.prover.client.get_uncle_count(header.number.low_u64()).await?.low_u64();
		let mut headers: Vec<CodecHeader> = vec![];
		for i in 0..uncle_count {
			let header = self.prover.client.get_uncle(header.number.low_u64(), i.into()).await?;
			if let Some(header) = header {
				headers.push(header.into())
			}
		}

		headers.push(header);
		headers.sort_by(|a, b| a.difficulty.cmp(&b.difficulty));
		let highest = headers[headers.len() - 1].clone();

		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.state_machine,
				consensus_state_id: self.consensus_state_id,
			},
			height: highest.number.low_u64(),
		};
		let state_machine_commitment = counterparty.query_state_machine_commitment(height).await?;
		if state_machine_commitment.state_root != highest.state_root {
			// Submit Freeze message
			log::info!(
				"Freezing {:?} on {:?}",
				self.state_machine,
				counterparty.state_machine_id().state_id
			);
			counterparty.freeze_state_machine(height.id).await?;
		}
		Ok(())
	}
}
