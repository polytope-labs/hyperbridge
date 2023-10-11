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

use crate::extrinsic::Extrinsic;
use ismp::{consensus::ConsensusStateId, host::StateMachine, HashAlgorithm};
use parking_lot::Mutex;
use primitives::{queue::PipelineQueue, IsmpHost};
use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, sr25519, Pair};
use std::sync::Arc;
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
	},
	ext::sp_runtime::MultiSignature,
	OnlineClient,
};

mod calls;
pub mod config;
pub mod ext_queue;
pub mod extrinsic;
mod host;
mod provider;
pub mod runtime;
#[cfg(feature = "testing")]
mod testing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateConfig {
	/// State machine Identifier for this client.
	pub state_machine: StateMachine,
	/// The hashing algorithm that substrate chain uses.
	pub hashing: HashAlgorithm,
	/// Consensus state id
	pub consensus_state_id: String,
	/// RPC url for the chain
	pub chain_rpc_ws: String,
	/// Relayer account seed
	pub signer: String,
	/// Latest state machine height
	pub latest_height: Option<u64>,
}

/// Core substrate client.
pub struct SubstrateClient<I, C: subxt::Config> {
	/// Ismp host implementation
	pub host: I,
	/// Subxt client for the substrate chain
	pub client: OnlineClient<C>,
	/// Consensus state Id
	consensus_state_id: ConsensusStateId,
	/// State machine Identifier for this client.
	state_machine: StateMachine,
	/// The hashing algorithm that substrate chain uses.
	hashing: HashAlgorithm,
	/// Private key of the signing account
	pub signer: sr25519::Pair,
	/// Sender for extrinsic
	queue: Option<PipelineQueue<Extrinsic>>,
	/// Latest state machine height.
	latest_height: Arc<Mutex<u64>>,
}

impl<T, C> SubstrateClient<T, C>
where
	T: IsmpHost,
	C: subxt::Config + Send + Sync + Clone,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::Signature: From<MultiSignature> + Send + Sync,
	C::AccountId:
		From<sp_core::crypto::AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
{
	pub async fn new(host: T, config: SubstrateConfig) -> Result<Self, anyhow::Error> {
		let client = OnlineClient::<C>::from_url(&config.chain_rpc_ws).await?;
		// If latest height of the state machine on the counterparty is not provided in config
		// Set it to the latest parachain height
		let latest_height = if let Some(latest_height) = config.latest_height {
			latest_height
		} else {
			client
				.rpc()
				.header(None)
				.await?
				.expect("block header should be available")
				.number()
				.into()
		};
		let bytes = from_hex(&config.signer)?;
		let signer = sr25519::Pair::from_seed_slice(&bytes)?;
		let mut consensus_state_id: ConsensusStateId = Default::default();
		consensus_state_id.copy_from_slice(config.consensus_state_id.as_bytes());

		Ok(Self {
			host,
			client,
			queue: None,
			consensus_state_id,
			state_machine: config.state_machine,
			hashing: config.hashing,
			signer,
			latest_height: Arc::new(Mutex::new(latest_height)),
		})
	}

	pub fn signer(&self) -> sr25519::Pair {
		self.signer.clone()
	}

	pub fn set_queue(&mut self, queue: PipelineQueue<Extrinsic>) {
		self.queue = Some(queue);
	}
}
