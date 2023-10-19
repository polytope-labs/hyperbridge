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

pub use crate::provider::{filter_map_system_events, system_events_key};
use ismp::{consensus::ConsensusStateId, host::StateMachine, HashAlgorithm};
use parking_lot::Mutex;
use primitives::{IsmpHost, NonceProvider};
use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, sr25519, Pair};
use std::sync::Arc;
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
	},
	ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
	OnlineClient,
};

mod calls;
pub mod config;
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
	/// Latest state machine height.
	latest_height: Arc<Mutex<u64>>,
	/// Config
	config: SubstrateConfig,
	/// Nonce Provider
	nonce_provider: Option<NonceProvider>,
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
		let config_clone = config.clone();
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
			consensus_state_id,
			state_machine: config.state_machine,
			hashing: config.hashing,
			signer,
			latest_height: Arc::new(Mutex::new(latest_height)),
			config: config_clone,
			nonce_provider: None,
		})
	}

	pub fn set_latest_height(&mut self, height: u64) {
		self.latest_height = Arc::new(Mutex::new(height))
	}

	pub fn signer(&self) -> sr25519::Pair {
		self.signer.clone()
	}

	pub fn set_nonce_provider(&mut self, nonce_provider: NonceProvider) {
		self.nonce_provider = Some(nonce_provider);
	}

	pub fn account(&self) -> C::AccountId {
		MultiSigner::Sr25519(self.signer.public()).into_account().into()
	}

	pub async fn initialize_nonce(&self) -> Result<NonceProvider, anyhow::Error> {
		let nonce = self.client.tx().account_nonce(&self.account()).await?;
		Ok(NonceProvider::new(nonce))
	}

	pub async fn get_nonce(&self) -> Result<u64, anyhow::Error> {
		if let Some(nonce_provider) = self.nonce_provider.as_ref() {
			return Ok(nonce_provider.get_nonce().await)
		}
		Err(anyhow::anyhow!("Nonce provider not set on client"))
	}
}
