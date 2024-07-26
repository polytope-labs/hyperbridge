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

use std::sync::Arc;

use crate::BeefyHost;
use anyhow::anyhow;
use codec::{Decode, Encode};
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::StateMachineUpdated,
};
use sp_core::H256;
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip,
		substrate::SubstrateHeader, ExtrinsicParams,
	},
	ext::sp_runtime::MultiSignature,
	Config,
};
use tesseract_primitives::{ByzantineHandler, IsmpProvider};

#[async_trait::async_trait]
impl<R, P> ByzantineHandler for BeefyHost<R, P>
where
	R: subxt::Config + Send + Sync + Clone,
	P: subxt::Config + Send + Sync + Clone,
	<P::ExtrinsicParams as ExtrinsicParams<P::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<P, PlainTip>>,
	P::Signature: From<MultiSignature> + Send + Sync,
	P::AccountId:
		From<sp_core::crypto::AccountId32> + Into<P::Address> + Clone + 'static + Send + Sync,
	H256: From<<P as Config>::Hash>,
{
	async fn check_for_byzantine_attack(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
		event: StateMachineUpdated,
	) -> Result<(), anyhow::Error> {
		let is_syncing = self.client.client.rpc().system_health().await?.is_syncing;
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.client.state_machine_id().state_id,
				consensus_state_id: self.client.state_machine_id().consensus_state_id,
			},
			height: event.latest_height,
		};

		let Some(block_hash) =
			self.client.client.rpc().block_hash(Some(event.latest_height.into())).await?
		else {
			// If block header is not found and node is fully synced, veto the state commitment
			if !is_syncing {
				log::info!(
					"Vetoing state commitment for {} on {}",
					self.client.state_machine_id().state_id,
					counterparty.state_machine_id().state_id
				);
				counterparty.veto_state_commitment(height).await?;
				return Ok(())
			} else {
				Err(anyhow!("Node is still syncing, cannot fetch finalized block"))?
			}
		};
		let header = self
			.client
			.client
			.rpc()
			.header(Some(block_hash))
			.await?
			.ok_or_else(|| anyhow!("Failed to get block header in byzantine handler"))?;

		let header = SubstrateHeader::<u32, P::Hasher>::decode(&mut &*header.encode())?;

		let finalized_state_commitment =
			counterparty.query_state_machine_commitment(height).await?;

		if finalized_state_commitment.state_root != header.state_root.into() {
			log::info!(
				"Vetoing state commitment for {} on {}",
				self.client.state_machine_id().state_id,
				counterparty.state_machine_id().state_id
			);
			counterparty.veto_state_commitment(height).await?;
		}

		Ok(())
	}
}
