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

use crate::BeefyHost;
use anyhow::anyhow;
use codec::Encode;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::StateMachineUpdated,
	messaging::ConsensusMessage,
};
use sp_core::H256;
use subxt::{config::substrate::SubstrateHeader, Config};
use tesseract_primitives::{ByzantineHandler, IsmpHost, IsmpProvider};

#[async_trait::async_trait]
impl<R, P> ByzantineHandler for BeefyHost<R, P>
where
	R: subxt::Config + Send + Sync + Clone,
	P: subxt::Config + Send + Sync + Clone,
	H256: From<<P as Config>::Hash>,
{
	async fn query_consensus_message(
		&self,
		event: StateMachineUpdated,
	) -> Result<ConsensusMessage, anyhow::Error> {
		let hash = self
			.prover
			.inner()
			.para
			.rpc()
			.block_hash(Some(event.latest_height.into()))
			.await?
			.ok_or_else(|| anyhow!("Block Hash should exist"))?;
		let header = self
			.prover
			.inner()
			.para
			.rpc()
			.header(Some(hash))
			.await?
			.ok_or_else(|| anyhow!("Block Header should exist"))?;
		Ok(ConsensusMessage {
			consensus_proof: header.encode(),
			consensus_state_id: self.consensus_state_id,
			signer: vec![],
		})
	}

	async fn check_for_byzantine_attack<C: IsmpHost + IsmpProvider>(
		&self,
		counterparty: &C,
		consensus_message: ConsensusMessage,
	) -> Result<(), anyhow::Error> {
		let header = <SubstrateHeader<u32, <P as Config>::Hasher> as codec::Decode>::decode(
			&mut &*consensus_message.consensus_proof,
		)?;
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.provider.state_machine_id().state_id,
				consensus_state_id: self.consensus_state_id,
			},
			height: header.number.into(),
		};

		let finalized_state_commitment =
			counterparty.query_state_machine_commitment(height).await?;

		if finalized_state_commitment.state_root != header.state_root.into() {
			log::info!(
				"Vetoing state commitment for {:?} on {:?}",
				self.provider.state_machine_id().state_id,
				counterparty.state_machine_id().state_id
			);
			counterparty.veto_state_commitment(height).await?;
		}

		Ok(())
	}
}
