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

pub use crate::provider::system_events_key;
use crate::rpc_wrapper::ClientWrapper;
use anyhow::Context;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use pallet_ismp::primitives::HashAlgorithm;
use primitives::{config::Chain, IsmpHost, IsmpProvider};
use reconnecting_jsonrpsee_ws_client::{Client, FixedInterval, PingConfig};
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
	/// Hyperbridge network
	pub chain: Chain,
	/// The hashing algorithm that substrate chain uses.
	pub hashing: Option<HashAlgorithm>,
	/// Consensus state id
	pub consensus_state_id: Option<String>,
	/// Websocket RPC url for the chain
	pub rpc_ws: String,
	/// Relayer account seed
	pub signer: Option<String>,
	/// Latest state machine height
	pub latest_height: Option<u64>,
}

impl SubstrateConfig {
	pub fn state_machine(&self) -> StateMachine {
		self.chain.state_machine()
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
}

impl<T, C> SubstrateClient<T, C>
where
	T: IsmpHost + 'static,
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
			// retry every second
			.retry_policy(FixedInterval::from_millis(1000))
			.enable_ws_ping(
				PingConfig::new()
					.ping_interval(Duration::from_secs(6))
					.inactive_limit(Duration::from_secs(30)),
			)
			.build(config.rpc_ws.clone())
			.await
			.context(format!("Failed to connect to substrate rpc {}", config.rpc_ws))?;
		let client = OnlineClient::<C>::from_rpc_client(Arc::new(ClientWrapper(raw_client)))
			.await
			.context("Failed to query from substrate rpc")?;
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
		let bytes = config
			.signer
			.and_then(|seed| from_hex(&seed).ok())
			.unwrap_or(H256::random().0.to_vec());
		let signer = sr25519::Pair::from_seed_slice(&bytes)?;
		let mut consensus_state_id: ConsensusStateId = Default::default();
		consensus_state_id
			.copy_from_slice(config.consensus_state_id.unwrap_or("PARA".into()).as_bytes());
		let address = signer.public().0.to_vec();
		Ok(Self {
			host,
			client,
			consensus_state_id,
			state_machine: config.chain.state_machine(),
			hashing: config.hashing.clone().unwrap_or(HashAlgorithm::Keccak),
			signer,
			address,
			initial_height: latest_height,
			config: config_clone,
		})
	}

	pub fn signer(&self) -> sr25519::Pair {
		self.signer.clone()
	}

	pub fn account(&self) -> C::AccountId {
		MultiSigner::Sr25519(self.signer.public()).into_account().into()
	}

	pub async fn set_latest_finalized_height<P: IsmpProvider + 'static>(
		&mut self,
		counterparty: &P,
	) -> Result<(), anyhow::Error> {
		let id = self.state_machine_id();
		self.initial_height = counterparty.query_latest_height(id).await?.into();

		Ok(())
	}

	pub fn req_commitments_key(&self, commitment: H256) -> Vec<u8> {
		let addr = runtime::api::storage().ismp().request_commitments(&commitment);
		self.client.storage().address_bytes(&addr).expect("Infallible")
	}

	pub fn res_commitments_key(&self, commitment: H256) -> Vec<u8> {
		let addr = runtime::api::storage().ismp().response_commitments(&commitment);
		self.client.storage().address_bytes(&addr).expect("Infallible")
	}

	pub fn req_receipts_key(&self, commitment: H256) -> Vec<u8> {
		let addr = runtime::api::storage().ismp().request_receipts(&commitment);
		self.client.storage().address_bytes(&addr).expect("Infallible")
	}

	pub fn res_receipt_key(&self, commitment: H256) -> Vec<u8> {
		let addr = runtime::api::storage().ismp().response_receipts(&commitment);
		self.client.storage().address_bytes(&addr).expect("Infallible")
	}
}
