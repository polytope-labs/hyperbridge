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

use anyhow::Context;
use std::sync::Arc;

use polkadot_sdk::sp_core::{bytes::from_hex, sr25519, Pair};
use serde::{Deserialize, Serialize};
use subxt::{
	backend::legacy::LegacyRpcMethods,
	config::{ExtrinsicParams, HashFor, Header},
	ext::subxt_rpcs::RpcClient,
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature, H256},
	OnlineClient,
};

use ismp::{consensus::ConsensusStateId, host::StateMachine};
use pallet_ismp::child_trie::{
	request_commitment_storage_key, request_receipt_storage_key, response_commitment_storage_key,
	response_receipt_storage_key,
};
use substrate_state_machine::HashAlgorithm;
use tesseract_primitives::{IsmpProvider, StateMachineUpdated, StreamError};

pub use crate::provider::system_events_key;

mod byzantine;
pub mod calls;
pub mod config;
pub mod extrinsic;
mod provider;

#[cfg(feature = "testing")]
mod testing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateConfig {
	/// Hyperbridge network
	#[serde(with = "serde_hex_utils::as_string")]
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
	pub signer: String,
	/// Initial height from which to start querying messages
	pub initial_height: Option<u64>,
	/// Max concurrent rpc requests allowed
	pub max_concurent_queries: Option<u64>,
	/// Frequency at which state machine updates will be queried in seconds
	pub poll_interval: Option<u64>,
	/// Decimals for the fee token on this substrate chain
	pub fee_token_decimals: Option<u8>,
}

/// Core substrate client.
pub struct SubstrateClient<C: subxt::Config> {
	/// Subxt client for the substrate chain
	pub client: OnlineClient<C>,
	/// Legacy subxt rpc client
	pub rpc: LegacyRpcMethods<C>,
	/// Raw rpc client
	pub rpc_client: RpcClient,
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
	/// Initial height from which to start querying messages
	initial_height: u64,
	/// Max concurrent rpc requests allowed
	max_concurent_queries: Option<u64>,
	/// Producer for state machine updated stream
	state_machine_update_sender: Arc<
		tokio::sync::Mutex<
			Option<tokio::sync::broadcast::Sender<Result<StateMachineUpdated, StreamError>>>,
		>,
	>,
	/// Substrate config
	config: SubstrateConfig,
}

impl<C> SubstrateClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Signature: From<MultiSignature> + Send + Sync,
	C::AccountId: From<AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C>>::Params: Send + Sync + DefaultParams,
	H256: From<HashFor<C>>,
{
	pub async fn new(config: SubstrateConfig) -> Result<Self, anyhow::Error> {
		let max_rpc_payload_size = config.max_rpc_payload_size.unwrap_or(300u32 * 1024 * 1024);
		let (client, rpc_client) =
			subxt_utils::client::ws_client::<C>(&config.rpc_ws, max_rpc_payload_size).await?;
		let rpc = LegacyRpcMethods::<C>::new(rpc_client.clone());
		// If latest height of the state machine on the counterparty is not provided in config
		// Set it to the latest parachain height
		let initial_height = if let Some(initial_height) = config.initial_height {
			initial_height
		} else {
			rpc.chain_get_header(None)
				.await?
				.expect("block header should be available")
				.number()
				.into()
		};
		let bytes =
			from_hex(&config.signer).context("Signer must be a valid hex-encoded String")?;
		let signer = sr25519::Pair::from_seed_slice(&bytes)?;
		let mut consensus_state_id: ConsensusStateId = Default::default();
		consensus_state_id
			.copy_from_slice(config.consensus_state_id.clone().unwrap_or("DOT0".into()).as_bytes());
		let address = signer.public().0.to_vec();
		Ok(Self {
			client,
			rpc,
			rpc_client,
			consensus_state_id,
			state_machine: config.state_machine,
			hashing: config.hashing.clone().unwrap_or(HashAlgorithm::Keccak),
			signer,
			address,
			initial_height,
			max_concurent_queries: config.max_concurent_queries,
			state_machine_update_sender: Arc::new(tokio::sync::Mutex::new(None)),
			config,
		})
	}

	pub fn signer(&self) -> sr25519::Pair {
		self.signer.clone()
	}

	pub fn account(&self) -> C::AccountId {
		let binding = self.signer.public();
		let public_key_slice: &[u8] = binding.as_ref();

		let public_key_array: [u8; 32] =
			public_key_slice.try_into().expect("Public key must be 32 bytes");

		let account_id = subxt::utils::AccountId32::from(public_key_array);

		account_id.into()
	}

	pub async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let name = counterparty.name();
		if self.config.initial_height.is_none() {
			self.initial_height = self.query_finalized_height().await?.into();
		}
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
			rpc: self.rpc.clone(),
			rpc_client: self.rpc_client.clone(),
			consensus_state_id: self.consensus_state_id,
			state_machine: self.state_machine,
			hashing: self.hashing.clone(),
			signer: self.signer.clone(),
			address: self.address.clone(),
			initial_height: self.initial_height,
			max_concurent_queries: self.max_concurent_queries,
			state_machine_update_sender: self.state_machine_update_sender.clone(),
			config: self.config.clone(),
		}
	}
}
