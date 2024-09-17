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
use std::sync::Arc;

use ismp::{consensus::ConsensusStateId, host::StateMachine};
use pallet_ismp::child_trie::{
	request_commitment_storage_key, request_receipt_storage_key, response_commitment_storage_key,
	response_receipt_storage_key,
};
use tesseract_primitives::{IsmpProvider, StateMachineUpdated, StreamError};

use serde::{Deserialize, Serialize};
use subxt::ext::sp_core::{bytes::from_hex, crypto, sr25519, Pair, H256};

use substrate_state_machine::HashAlgorithm;
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
	},
	ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
	OnlineClient,
};

mod byzantine;
pub mod calls;
pub mod config;
pub mod extrinsic;
mod provider;

#[cfg(feature = "testing")]
pub use subxt_utils::gargantua as runtime;
#[cfg(feature = "testing")]
mod testing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateConfig {
	/// Hyperbridge network
	#[serde(with = "serde_utils::as_string")]
	pub state_machine: StateMachine,
	/// The hashing algorithm that substrate chain uses.
	pub hashing: Option<HashAlgorithm>,
	/// Consensus state id
	pub consensus_state_id: Option<String>,
	/// Websocket RPC url for the chain
	pub rpc_ws: String,
	/// Maximum size in bytes for the rpc payloads, both requests & responses.
	pub max_rpc_payload_size: Option<u32>,
	/// Relayer account seed
	pub signer: Option<String>,
	/// Latest state machine height
	pub latest_height: Option<u64>,
	/// Max concurrent rpc requests allowed
	pub max_concurent_queries: Option<u64>,
}

/// Core substrate client.
pub struct SubstrateClient<C: subxt::Config> {
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
	/// Max concurrent rpc requests allowed
	max_concurent_queries: Option<u64>,
	/// Producer for state machine updated stream
	state_machine_update_sender: Arc<
		tokio::sync::Mutex<
			Option<tokio::sync::broadcast::Sender<Result<StateMachineUpdated, StreamError>>>,
		>,
	>,
}

impl<C> SubstrateClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::Signature: From<MultiSignature> + Send + Sync,
	C::AccountId: From<crypto::AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
{
	pub async fn new(config: SubstrateConfig) -> Result<Self, anyhow::Error> {
		let max_rpc_payload_size = config.max_rpc_payload_size.unwrap_or(300u32 * 1024 * 1024);
		let client =
			subxt_utils::client::ws_client::<C>(&config.rpc_ws, max_rpc_payload_size).await?;
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
			client,
			consensus_state_id,
			state_machine: config.state_machine,
			hashing: config.hashing.clone().unwrap_or(HashAlgorithm::Keccak),
			signer,
			address,
			initial_height: latest_height,
			max_concurent_queries: config.max_concurent_queries,
			state_machine_update_sender: Arc::new(tokio::sync::Mutex::new(None)),
		})
	}

	pub fn signer(&self) -> sr25519::Pair {
		self.signer.clone()
	}

	pub fn account(&self) -> C::AccountId {
		MultiSigner::Sr25519(self.signer.public()).into_account().into()
	}

	pub async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let name = counterparty.name();
		self.initial_height = self.query_finalized_height().await?.into();
		log::info!(
			"Initialized height for {:?}->{name} at {}",
			self.state_machine,
			self.initial_height
		);
		Ok(())
	}

	pub fn req_commitments_key(&self, commitment: H256) -> Vec<u8> {
		request_commitment_storage_key(commitment)
	}

	pub fn res_commitments_key(&self, commitment: H256) -> Vec<u8> {
		response_commitment_storage_key(commitment)
	}

	pub fn req_receipts_key(&self, commitment: H256) -> Vec<u8> {
		request_receipt_storage_key(commitment)
	}

	pub fn res_receipt_key(&self, commitment: H256) -> Vec<u8> {
		response_receipt_storage_key(commitment)
	}
}

impl<C: subxt::Config> Clone for SubstrateClient<C> {
	fn clone(&self) -> Self {
		Self {
			client: self.client.clone(),
			consensus_state_id: self.consensus_state_id,
			state_machine: self.state_machine,
			hashing: self.hashing.clone(),
			signer: self.signer.clone(),
			address: self.address.clone(),
			initial_height: self.initial_height,
			max_concurent_queries: self.max_concurent_queries,
			state_machine_update_sender: self.state_machine_update_sender.clone(),
		}
	}
}
