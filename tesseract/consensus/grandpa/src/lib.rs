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

use ::polkadot_sdk::sp_runtime::traits::{One, Zero};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use subxt::{
	config::{ExtrinsicParams, HashFor, Header},
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature, H256},
};

use grandpa_prover::{GrandpaProver, ProverOptions, GRANDPA_CURRENT_SET_ID};
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use tesseract_primitives::IsmpHost;
use tesseract_substrate::{SubstrateClient, SubstrateConfig};

mod host;

/// Default maximum block range to prove finality for, roughly 4 hours of blocks
/// on a typical Substrate chain with 6-second block time.
const DEFAULT_BLOCK_RANGE: u32 = 2400;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrandpaConfig {
	/// substrate config options
	pub substrate: SubstrateConfig,
	/// Host config
	pub grandpa: HostConfig,
}

impl GrandpaConfig {
	pub async fn into_client<H, C>(&self) -> anyhow::Result<Arc<dyn IsmpHost>>
	where
		H: subxt::Config + Send + Sync + Clone,
		C: subxt::Config + Send + Sync + Clone,
		<H::Header as Header>::Number:
			Ord + Zero + finality_grandpa::BlockNumberOps + One + From<u32>,
		u32: From<<H::Header as Header>::Number>,
		H256: From<HashFor<H>>,
		H::Header: codec::Decode,
		<H::Hasher as subxt::config::Hasher>::Output: From<HashFor<H>>,
		HashFor<H>: From<<H::Hasher as subxt::config::Hasher>::Output>,
		HashFor<H>: From<H256>,
		<H::ExtrinsicParams as ExtrinsicParams<H>>::Params: Send + Sync + DefaultParams,
		H::Signature: From<MultiSignature> + Send + Sync,
		H::AccountId: From<AccountId32> + Into<H::Address> + Clone + 'static + Send + Sync,
		<C::ExtrinsicParams as ExtrinsicParams<C>>::Params: Send + Sync + DefaultParams,
		C::Signature: From<MultiSignature> + Send + Sync,
		C::AccountId: From<AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
		<C::ExtrinsicParams as ExtrinsicParams<C>>::Params: Send + Sync + DefaultParams,
		H256: From<HashFor<C>>,
	{
		let host = GrandpaHost::<H, C>::new(&self).await?;
		Ok(Arc::new(host))
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
	/// RPC url for a standalone chain or relay chain
	pub rpc: String,
	/// slot duration of the chain
	pub slot_duration: u64,
	/// Update frequency
	pub consensus_update_frequency: Option<u64>,
	/// para ids
	pub para_ids: Vec<u32>,
	/// Maximum block range to prove finality for
	pub max_block_range: Option<u32>,
}

#[derive(Clone)]
pub struct GrandpaHost<H: subxt::Config, C: subxt::Config> {
	/// Consensus state id on counterparty chain
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this chain.
	pub state_machine: StateMachine,
	/// Grandpa prover
	pub prover: GrandpaProver<H>,
	/// Grandpa config
	pub config: GrandpaConfig,
	/// The underlying substrate client
	pub substrate_client: SubstrateClient<C>,
}

impl<H, C> GrandpaHost<H, C>
where
	H: subxt::Config + Send + Sync + Clone,
	C: subxt::Config + Send + Sync + Clone,
	<H::Header as Header>::Number: Ord + Zero + From<u32>,
	u32: From<<H::Header as Header>::Number>,
	H256: From<HashFor<H>>,
	H::Header: codec::Decode,
	<H::ExtrinsicParams as ExtrinsicParams<H>>::Params: Send + Sync + DefaultParams,
	H::Signature: From<MultiSignature> + Send + Sync,
	H::AccountId: From<AccountId32> + Into<H::Address> + Clone + 'static + Send + Sync,

	<C::ExtrinsicParams as ExtrinsicParams<C>>::Params: Send + Sync + DefaultParams,
	C::Signature: From<MultiSignature> + Send + Sync,
	C::AccountId: From<AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
	H256: From<HashFor<C>>,
{
	pub async fn new(config: &GrandpaConfig) -> Result<Self, anyhow::Error> {
		let prover = GrandpaProver::new(ProverOptions {
			ws_url: config.grandpa.rpc.clone(),
			para_ids: config.grandpa.para_ids.clone(),
			state_machine: config.substrate.state_machine,
			max_rpc_payload_size: 150 * 1024 * 1024,
			max_block_range: config.grandpa.max_block_range.unwrap_or(DEFAULT_BLOCK_RANGE),
		})
		.await?;
		let substrate_client = SubstrateClient::<C>::new(config.substrate.clone()).await?;
		Ok(GrandpaHost {
			consensus_state_id: {
				let mut consensus_state_id: ConsensusStateId = Default::default();
				consensus_state_id.copy_from_slice(
					config
						.substrate
						.consensus_state_id
						.clone()
						.expect("Expected consensus state id")
						.as_bytes(),
				);
				consensus_state_id
			},
			state_machine: config.substrate.state_machine,
			substrate_client,
			prover,
			config: config.clone(),
		})
	}

	pub async fn should_sync(&self, consensus_state_set_id: u64) -> Result<bool, anyhow::Error> {
		let current_set_id: u64 = {
			let block_hash = self
				.prover
				.rpc
				.chain_get_block_hash(None)
				.await?
				.ok_or_else(|| anyhow!("Failed to query latest block hash"))?;
			let raw_id = self
				.prover
				.client
				.storage()
				.at(block_hash)
				.fetch_raw(&GRANDPA_CURRENT_SET_ID[..])
				.await
				.ok()
				.flatten()
				.expect("Failed to fetch current set id");
			codec::Decode::decode(&mut &*raw_id)?
		};

		Ok(current_set_id > consensus_state_set_id)
	}
}
