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
use crate::rpc_wrapper::ClientWrapper;
use hex_literal::hex;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use pallet_ismp::primitives::HashAlgorithm;
use primitives::{IsmpHost, NonceProvider};
use reconnecting_jsonrpsee_ws_client::{Client, ExponentialBackoff, PingConfig};
use serde::{Deserialize, Serialize};
use sp_core::{bytes::from_hex, sr25519, Pair, H256};
use std::{sync::Arc, time::Duration};
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
	},
	ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
	OnlineClient,
};

pub mod calls;
pub mod config;
pub mod extrinsic;
mod host;
mod provider;
pub mod rpc_wrapper;
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

impl SubstrateConfig {
	pub fn state_machine(&self) -> StateMachine {
		self.state_machine
	}
}

/// Core substrate client.
pub struct SubstrateClient<I, C: subxt::Config> {
	/// Ismp naive implementation
	pub host: Option<I>,
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
	/// Public Address
	pub address: Vec<u8>,
	/// Latest state machine height.
	initial_height: u64,
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
	pub async fn new(host: Option<T>, config: SubstrateConfig) -> Result<Self, anyhow::Error> {
		let config_clone = config.clone();
		let raw_client = Client::builder()
			.retry_policy(ExponentialBackoff::from_millis(100))
			.enable_ws_ping(
				PingConfig::new()
					.ping_interval(Duration::from_secs(6))
					.inactive_limit(Duration::from_secs(30)),
			)
			.build(config.chain_rpc_ws)
			.await?;
		let client =
			OnlineClient::<C>::from_rpc_client(Arc::new(ClientWrapper(raw_client))).await?;
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
		let address = signer.public().0.to_vec();
		Ok(Self {
			host,
			client,
			consensus_state_id,
			state_machine: config.state_machine,
			hashing: config.hashing,
			signer,
			address,
			initial_height: latest_height,
			config: config_clone,
			nonce_provider: None,
		})
	}

	pub fn signer(&self) -> sr25519::Pair {
		self.signer.clone()
	}

	pub fn account(&self) -> C::AccountId {
		MultiSigner::Sr25519(self.signer.public()).into_account().into()
	}

	pub async fn get_nonce(&self) -> Result<u64, anyhow::Error> {
		if let Some(nonce_provider) = self.nonce_provider.as_ref() {
			return Ok(nonce_provider.get_nonce().await)
		}
		Err(anyhow::anyhow!("Nonce provider not set on client"))
	}

	pub fn req_commitments_key(&self, commitment: H256) -> Vec<u8> {
		let mut key =
			hex!("103895530afb23bb607661426d55eb8bbd3caa596ab5c98b359f0ffc7d17e376").to_vec();
		key.extend_from_slice(commitment.as_bytes());
		key
	}

	pub fn res_commitments_key(&self, commitment: H256) -> Vec<u8> {
		let mut key =
			hex!("103895530afb23bb607661426d55eb8b8fdfbc1b10c58ed36779810ffdba8e79").to_vec();
		key.extend_from_slice(commitment.as_bytes());
		key
	}

	pub fn req_receipts_key(&self, commitment: H256) -> Vec<u8> {
		let mut key =
			hex!("103895530afb23bb607661426d55eb8b0484aecefe882c3ce64e6f82507f715a").to_vec();
		key.extend_from_slice(commitment.as_bytes());
		key
	}

	pub fn res_receipt_key(&self, commitment: H256) -> Vec<u8> {
		let mut key =
			hex!("103895530afb23bb607661426d55eb8b554b72b7162725f9457d35ecafb8b02f").to_vec();
		key.extend_from_slice(commitment.as_bytes());
		key
	}
}
