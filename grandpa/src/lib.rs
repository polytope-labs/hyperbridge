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

use grandpa_prover::GrandpaProver;
use hex_literal::hex;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use serde::{Deserialize, Serialize};
use sp_core::{crypto, H256};
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
	},
	ext::sp_runtime::{
		traits::{One, Zero},
		MultiSignature,
	},
	OnlineClient,
};
use tesseract_primitives::IsmpHost;
use tesseract_substrate::{SubstrateClient, SubstrateConfig};

mod host;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrandpaConfig {
	/// substrate config options
	pub substrate: SubstrateConfig,
	/// Host config
	pub host: HostConfig,
}

impl GrandpaConfig {
	pub async fn into_client<H, C>(&self) -> anyhow::Result<Arc<dyn IsmpHost>>
	where
		H: subxt::Config + Send + Sync + Clone,
		C: subxt::Config + Send + Sync + Clone,
		<H::Header as Header>::Number: Ord + Zero + finality_grandpa::BlockNumberOps + One,
		u32: From<<H::Header as Header>::Number>,
		sp_core::H256: From<H::Hash>,
		H::Header: codec::Decode,
		<H::Hasher as subxt::config::Hasher>::Output: From<H::Hash>,
		H::Hash: From<<H::Hasher as subxt::config::Hasher>::Output>,
		<H as subxt::Config>::Hash: From<sp_core::H256>,
		<H::ExtrinsicParams as ExtrinsicParams<H::Hash>>::OtherParams:
			Default + Send + Sync + From<BaseExtrinsicParamsBuilder<H, PlainTip>>,
		H::Signature: From<MultiSignature> + Send + Sync,
		H::AccountId: From<crypto::AccountId32> + Into<H::Address> + Clone + 'static + Send + Sync,
		<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
			Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
		C::Signature: From<MultiSignature> + Send + Sync,
		C::AccountId: From<crypto::AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
		H256: From<<C as subxt::Config>::Hash>,
	{
		let host = GrandpaHost::<H, C>::new(&self).await?;
		Ok(Arc::new(host))
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
	/// RPC url for a standalone chain or relay chain
	pub rpc: String,
	/// State machine Identifier for this client on it's counterparties.
	pub state_machine: StateMachine,
	/// Consensus state id on counterparty chain
	pub consensus_state_id: ConsensusStateId,
	/// slot duration of the chain
	pub slot_duration: u64,
	/// Update frequency
	pub consensus_update_frequency: Option<u64>,
	/// para ids
	pub para_ids: Vec<u32>,
	/// Raw storage key for the babe epoch start storage value
	pub babe_epoch_start_key: Option<Vec<u8>>,
	/// Raw Storage key for the current set id in pallet grandpa
	pub current_set_id_key: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct GrandpaHost<H: subxt::Config, C: subxt::Config> {
	/// Consensus state id on counterparty chain
	pub consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this chain.
	pub state_machine: StateMachine,
	/// Subxt client for the chain.
	pub client: OnlineClient<H>,
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
	<H::Header as Header>::Number: Ord + Zero,
	u32: From<<H::Header as Header>::Number>,
	sp_core::H256: From<H::Hash>,
	H::Header: codec::Decode,
	<H::ExtrinsicParams as ExtrinsicParams<H::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<H, PlainTip>>,
	H::Signature: From<MultiSignature> + Send + Sync,
	H::AccountId: From<crypto::AccountId32> + Into<H::Address> + Clone + 'static + Send + Sync,

	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::Signature: From<MultiSignature> + Send + Sync,
	C::AccountId: From<crypto::AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
	H256: From<<C as subxt::Config>::Hash>,
{
	pub async fn new(config: &GrandpaConfig) -> Result<Self, anyhow::Error> {
		let client = OnlineClient::from_url(&config.host.rpc).await?;
		let default_babe_epoch_start_key: [u8; 32] =
			hex!("1cb6f36e027abb2091cfb5110ab5087fe90e2fbf2d792cb324bffa9427fe1f0e");
		let default_current_set_id_key: [u8; 32] =
			hex!("5f9cc45b7a00c5899361e1c6099678dc8a2d09463effcc78a22d75b9cb87dffc");
		let prover = GrandpaProver::new(
			&config.host.rpc,
			config.host.para_ids.clone(),
			config.substrate.state_machine,
			config
				.host
				.babe_epoch_start_key
				.clone()
				.unwrap_or(default_babe_epoch_start_key.to_vec()),
			config
				.host
				.current_set_id_key
				.clone()
				.unwrap_or(default_current_set_id_key.to_vec()),
		)
		.await?;
		let substrate_client = SubstrateClient::<C>::new(config.substrate.clone()).await?;
		Ok(GrandpaHost {
			consensus_state_id: config.host.consensus_state_id.clone(),
			state_machine: config.substrate.state_machine,
			client,
			substrate_client,
			prover,
			config: config.clone(),
		})
	}

	pub async fn should_sync(&self, consensus_state_set_id: u64) -> Result<bool, anyhow::Error> {
		let current_set_id: u64 = {
			let raw_id = self
				.client
				.storage()
				.at_latest()
				.await?
				.fetch_raw(&self.prover.current_set_id[..])
				.await
				.ok()
				.flatten()
				.expect("Failed to fetch current set id");
			codec::Decode::decode(&mut &*raw_id)?
		};

		Ok(current_set_id > consensus_state_set_id)
	}
}
