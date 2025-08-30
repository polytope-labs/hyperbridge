// Copyright (C) Polytope Labs Ltd.
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

use core::time::Duration;
use std::ops::RangeInclusive;

use anyhow::{anyhow, Context, Error};
use codec::{Decode, Encode};
use futures::{stream, StreamExt, TryStreamExt};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use gloo_timers::future::*;
use hashbrown::HashMap;
use hex_literal::hex;
use primitive_types::{H160, H256};
use serde::{Deserialize, Serialize};
use sp_core::storage::ChildInfo;
use subxt::{
	backend::legacy::LegacyRpcMethods,
	config::{HashFor, Header},
	ext::subxt_rpcs::{
		methods::legacy::{StorageData, StorageKey},
		rpc_params, RpcClient,
	},
	tx::Payload,
	OnlineClient,
};
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::*;
#[cfg(all(target_arch = "wasm32", feature = "nodejs"))]
use wasmtimer::tokio::*;

use ismp::{
	consensus::{StateCommitment, StateMachineHeight, StateMachineId},
	events::{Event, RequestResponseHandled, StateMachineUpdated},
	host::StateMachine,
	messaging::{hash_request, hash_response, Message},
	router::{Request, Response},
};
use pallet_ismp::{
	child_trie::{
		request_commitment_storage_key, response_commitment_storage_key, CHILD_TRIE_PREFIX,
	},
	offchain::ProofKeys,
	ResponseReceipt,
};
use substrate_state_machine::StateMachineProof;
use subxt_utils::{
	refine_subxt_error, state_machine_update_time_storage_key, values::messages_to_value,
};

use crate::{
	providers::interface::{Client, WithMetadata},
	types::{BoxStream, EventMetadata, HashAlgorithm, SubstrateStateProof},
	Keccak256,
};

use super::interface::Query;

/// Contains a scale encoded Mmr Proof or Trie proof
#[derive(Serialize, Deserialize)]
pub struct Proof {
	/// Scale encoded `MmrProof` or state trie proof `Vec<Vec<u8>>`
	pub proof: Vec<u8>,
	/// Height at which proof was recovered
	pub height: u32,
}

#[derive(Debug, Clone)]
pub struct SubstrateClient<C: subxt::Config + Clone> {
	/// RPC url of a hyperbridge node
	pub rpc_url: String,
	/// State machine
	pub state_machine: StateMachineId,
	/// An instance of Hyper bridge client using the default config
	pub client: OnlineClient<C>,
	pub rpc: LegacyRpcMethods<C>,
	pub rpc_client: RpcClient,
	pub hashing: HashAlgorithm,
}
impl<C> SubstrateClient<C>
where
	C: subxt::Config + Clone,
{
	pub async fn new(
		rpc_url: String,
		hashing: HashAlgorithm,
		consensus_state_id: [u8; 4],
		state_id: StateMachine,
	) -> Result<Self, Error> {
		let (client, rpc_client) =
			subxt_utils::client::ws_client(&rpc_url, 10 * 1024 * 1024).await?;
		let rpc = LegacyRpcMethods::<C>::new(rpc_client.clone());
		let state_machine = StateMachineId { state_id, consensus_state_id };

		Ok(Self { rpc_url, client, state_machine, hashing, rpc_client, rpc })
	}

	pub async fn latest_timestamp(&self) -> Result<Duration, Error> {
		let timestamp_key =
			hex!("f0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb").to_vec();
		let response = self
			.rpc
			.state_get_storage(&timestamp_key, None)
			.await?
			.ok_or_else(|| anyhow!("Failed to fetch timestamp"))?;
		let timestamp: u64 = codec::Decode::decode(&mut response.as_slice())?;

		Ok(Duration::from_millis(timestamp))
	}

	async fn query_ismp_events(
		&self,
		previous_height: u64,
		latest_height: u64,
	) -> Result<Vec<WithMetadata<Event>>, Error> {
		let range = (previous_height + 1)..=latest_height;
		if range.is_empty() {
			return Ok(Default::default());
		}

		#[derive(Clone, Hash, Debug, PartialEq, Eq, Copy, Serialize, Deserialize)]
		#[serde(untagged)]
		pub enum BlockNumberOrHash<Hash> {
			/// Block hash
			Hash(Hash),
			/// Block number
			Number(u32),
		}

		let params = rpc_params![
			BlockNumberOrHash::<H256>::Number(previous_height.saturating_add(1) as u32),
			BlockNumberOrHash::<H256>::Number(latest_height as u32)
		];
		let response: HashMap<String, Vec<WithMetadata<Event>>> =
			self.rpc_client.request("ismp_queryEventsWithMetadata", params).await?;
		let events = response.values().into_iter().cloned().flatten().collect();
		Ok(events)
	}
}

impl<C: subxt::Config + Clone> Client for SubstrateClient<C> {
	async fn query_latest_block_height(&self) -> Result<u64, Error> {
		let block_hash = self
			.rpc
			.chain_get_block_hash(None)
			.await?
			.ok_or_else(|| anyhow!("Failed to query latest block hash"))?;
		Ok(self.client.blocks().at(block_hash).await?.number().into())
	}

	fn state_machine_id(&self) -> StateMachineId {
		self.state_machine
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		self.latest_timestamp().await
	}

	async fn query_request_receipt(&self, request_hash: H256) -> Result<H160, Error> {
		let child_storage_key = ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
		let storage_key: StorageKey = self.request_receipt_full_key(request_hash);
		let params = rpc_params![child_storage_key, storage_key];

		let response: Option<StorageData> =
			self.rpc_client.request("childstate_getStorage", params).await?;
		if let Some(data) = response {
			let relayer = Vec::decode(&mut &*data)?;
			Ok(H160::from_slice(&relayer[..20]))
		} else {
			Ok(Default::default())
		}
	}

	async fn query_state_proof(&self, at: u64, keys: Vec<Vec<u8>>) -> Result<Vec<u8>, Error> {
		/// Contains a scale encoded Mmr Proof or Trie proof
		#[derive(Serialize, Deserialize)]
		pub struct RpcProof {
			/// Scale encoded `MmrProof` or state trie proof `Vec<Vec<u8>>`
			pub proof: Vec<u8>,
			/// Height at which proof was recovered
			pub height: u32,
		}

		let params = rpc_params![at, keys];
		let response: RpcProof =
			self.rpc_client.request("ismp_queryChildTrieProof", params).await?;
		let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*response.proof)?;
		let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
			hasher: self.hashing.clone(),
			storage_proof,
		});
		Ok(proof.encode())
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		counterparty: StateMachine,
	) -> Result<Vec<u8>, anyhow::Error> {
		if keys.is_empty() {
			Err(anyhow!("No queries provided"))?
		}
		match counterparty {
			// Use mmr proofs for queries going to EVM chains
			s if s.is_evm() => {
				let keys =
					ProofKeys::Requests(keys.into_iter().map(|key| key.commitment).collect());
				let params = rpc_params![at, keys];
				let response: Proof = self.rpc_client.request("mmr_queryProof", params).await?;
				Ok(response.proof)
			},
			// Use child trie proofs for queries going to substrate chains
			s if s.is_substrate() => {
				let keys: Vec<_> = keys
					.into_iter()
					.map(|key| request_commitment_storage_key(key.commitment))
					.collect();
				let params = rpc_params![at, keys];
				let response: Proof =
					self.rpc_client.request("ismp_queryChildTrieProof", params).await?;
				let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*response.proof)?;
				let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
					hasher: self.hashing.clone(),
					storage_proof,
				});
				Ok(proof.encode())
			},
			s => Err(anyhow::anyhow!("Unsupported state machine {s:?} !")),
		}
	}

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		counterparty: StateMachine,
	) -> Result<Vec<u8>, anyhow::Error> {
		if keys.is_empty() {
			Err(anyhow!("No queries provided"))?
		}

		match counterparty {
			// Use mmr proofs for queries going to EVM chains
			s if s.is_evm() => {
				let keys =
					ProofKeys::Responses(keys.into_iter().map(|key| key.commitment).collect());
				let params = rpc_params![at, keys];
				let response: Proof = self.rpc_client.request("mmr_queryProof", params).await?;
				Ok(response.proof)
			},
			// Use child trie proofs for queries going to substrate chains
			s if s.is_substrate() => {
				let keys: Vec<_> = keys
					.into_iter()
					.map(|key| response_commitment_storage_key(key.commitment))
					.collect();
				let params = rpc_params![at, keys];
				let response: Proof =
					self.rpc_client.request("ismp_queryChildTrieProof", params).await?;
				let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*response.proof)?;
				let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
					hasher: self.hashing.clone(),
					storage_proof,
				});
				Ok(proof.encode())
			},
			s => Err(anyhow::anyhow!("Unsupported state machine {s:?} !")),
		}
	}

	async fn query_response_receipt(&self, request_commitment: H256) -> Result<H160, Error> {
		let key = self.response_receipt_full_key(request_commitment);
		let child_storage_key = ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
		let storage_key: StorageKey = key;
		let params = rpc_params![child_storage_key, storage_key];

		let response: Option<StorageData> =
			self.rpc_client.request("childstate_getStorage", params).await?;
		if let Some(data) = response {
			let receipt = ResponseReceipt::decode(&mut &*data)?;
			Ok(H160::from_slice(&receipt.relayer[..20]))
		} else {
			Ok(Default::default())
		}
	}

	async fn ismp_events_stream(
		&self,
		commitment: H256,
		initial_height: u64,
	) -> Result<BoxStream<WithMetadata<Event>>, Error> {
		let stream =
			stream::unfold((initial_height, self.clone()), move |(latest_height, client)| {
				let commitment = commitment.clone();
				async move {
					tracing::trace!("Sleeping for 30s");
					sleep(Duration::from_secs(30)).await;

					let header = match client.rpc.chain_get_header(None).await {
						Ok(Some(header)) => header,
						Ok(None) => return Some((Ok(None), (latest_height, client))),
						Err(_err) => {
							tracing::error!(
								"Error encountered while querying latest head: {_err:?}"
							);
							return Some((Ok(None), (latest_height, client)));
						},
					};

					let events =
						match client.query_ismp_events(latest_height, header.number().into()).await
						{
							Ok(e) => e,
							Err(_err) => {
								tracing::error!(
									"Error encountered while querying ismp events {_err:?}"
								);
								return Some((Ok(None), (latest_height, client)));
							},
						};

					let event = events.into_iter().find_map(|event| {
						let value = match event.event.clone() {
							Event::PostRequest(post) =>
								Some(hash_request::<Keccak256>(&Request::Post(post.clone()))),
							Event::PostResponse(resp) =>
								Some(hash_response::<Keccak256>(&Response::Post(resp))),
							Event::PostRequestHandled(post) => Some(post.commitment),
							Event::PostResponseHandled(resp) => Some(resp.commitment),
							Event::GetResponse(response) =>
								Some(hash_request::<Keccak256>(&Request::Get(response.get))),
							_ => None,
						};

						if value == Some(commitment.clone()) {
							Some(event)
						} else {
							None
						}
					});

					let value = match event {
						Some(event) => Some((Ok(Some(event)), (header.number().into(), client))),
						None => Some((Ok(None), (header.number().into(), client))),
					};

					return value;
				}
			})
			.filter_map(|item| async move {
				match item {
					Ok(None) => None,
					Ok(Some(event)) => Some(Ok(event)),
					Err(err) => Some(Err(err)),
				}
			});

		Ok(Box::pin(stream))
	}

	async fn post_request_handled_stream(
		&self,
		commitment: H256,
		initial_height: u64,
	) -> Result<BoxStream<WithMetadata<RequestResponseHandled>>, Error> {
		let stream = self.ismp_events_stream(commitment, initial_height).await?.try_filter_map(
			|event| async move {
				let ret = if let Event::PostRequestHandled(handled) = event.event {
					Some(WithMetadata { event: handled, meta: event.meta })
				} else {
					None
				};

				Ok(ret)
			},
		);

		Ok(Box::pin(stream))
	}

	async fn query_latest_state_machine_height(
		&self,
		state_machine: StateMachineId,
	) -> Result<u64, anyhow::Error> {
		let params = rpc_params![state_machine];
		let response: u64 =
			self.rpc_client.request("ismp_queryStateMachineLatestHeight", params).await?;

		Ok(response)
	}

	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		// calculate key manually because sp_io uses host functions that are not available in the
		// browser
		let key = [
			pallet_ismp::child_trie::STATE_COMMITMENTS_KEY.to_vec(),
			ethers::utils::keccak256(&height.encode()).to_vec(),
		]
		.concat();

		let child_storage_key = ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
		let storage_key: StorageKey = key;
		let params = rpc_params![child_storage_key, storage_key, Option::<HashFor::<C>>::None];

		let response: Option<StorageData> =
			self.rpc_client.request("childstate_getStorage", params).await?;
		let data =
			response.ok_or_else(|| anyhow!("State commitment not present for state machine"))?;
		let commitment = Decode::decode(&mut &*data)?;
		Ok(commitment)
	}

	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<WithMetadata<StateMachineUpdated>>, Error> {
		let block_hash = self
			.rpc
			.chain_get_block_hash(None)
			.await?
			.ok_or_else(|| anyhow!("Failed to query latest block hash"))?;
		let initial_height: u64 = self.client.blocks().at(block_hash).await?.number().into();
		let stream = stream::unfold(
			(initial_height, self.clone()),
			move |(latest_height, client)| async move {
				tracing::trace!("Sleeping for 30s");
				sleep(Duration::from_secs(30)).await;

				let header = match client.rpc.chain_get_header(None).await {
					Ok(Some(header)) => header,
					Ok(None) => return Some((Ok(None), (latest_height, client))),
					Err(_err) => {
						tracing::error!("Error encountered while querying latest head: {_err:?}");
						return Some((Ok(None), (latest_height, client)));
					},
				};

				let events = match client
					.query_ismp_events(latest_height, header.number().into())
					.await
				{
					Ok(e) => e,
					Err(_err) => {
						tracing::error!("Error encountered while querying ismp events {_err:?}");
						return Some((Ok(None), (latest_height, client)));
					},
				};

				let event = events
					.into_iter()
					.filter_map(|event| match event.event {
						Event::StateMachineUpdated(e)
							if e.state_machine_id == counterparty_state_id =>
							Some((e, event.meta)),
						_ => None,
					})
					.max_by(|x, y| x.0.latest_height.cmp(&y.0.latest_height));

				let value = match event {
					Some((event, meta)) => Some((
						Ok(Some(WithMetadata { event, meta })),
						(header.number().into(), client),
					)),
					None => Some((Ok(None), (header.number().into(), client))),
				};

				return value;
			},
		)
		.filter_map(|item| async move {
			match item {
				Ok(None) => None,
				Ok(Some(event)) => Some(Ok(event)),
				Err(err) => Some(Err(err)),
			}
		});

		Ok(Box::pin(stream))
	}

	fn request_commitment_full_key(&self, commitment: H256) -> Vec<u8> {
		pallet_ismp::child_trie::request_commitment_storage_key(commitment)
	}

	fn request_receipt_full_key(&self, commitment: H256) -> Vec<u8> {
		pallet_ismp::child_trie::request_receipt_storage_key(commitment)
	}

	fn response_commitment_full_key(&self, commitment: H256) -> Vec<u8> {
		pallet_ismp::child_trie::response_commitment_storage_key(commitment)
	}

	fn response_receipt_full_key(&self, commitment: H256) -> Vec<u8> {
		pallet_ismp::child_trie::response_receipt_storage_key(commitment)
	}

	fn encode(&self, msg: Message) -> Result<Vec<u8>, Error> {
		let call = vec![messages_to_value(vec![msg])];
		if let Some(_) =
			self.client.metadata().pallet_by_name_err("Ismp")?.call_hash("handle_unsigned")
		{
			let extrinsic = subxt::dynamic::tx("Ismp", "handle_unsigned", call);
			let ext = self.client.tx().create_unsigned(&extrinsic)?;
			Ok(ext.into_encoded())
		} else {
			let extrinsic = subxt::dynamic::tx("Ismp", "handle", call);

			let call_data = extrinsic.encode_call_data(&self.client.metadata())?;
			Ok(call_data)
		}
	}

	async fn query_ismp_event(
		&self,
		range: RangeInclusive<u64>,
	) -> Result<Vec<WithMetadata<Event>>, anyhow::Error> {
		self.query_ismp_events(*range.start(), *range.end()).await
	}

	async fn submit(&self, msg: Message) -> Result<EventMetadata, Error> {
		let call = vec![messages_to_value(vec![msg])];

		let hyper_bridge_timeout_extrinsic = subxt::dynamic::tx("Ismp", "handle_unsigned", call);

		let ext = self.client.tx().create_unsigned(&hyper_bridge_timeout_extrinsic)?;
		let in_block = ext.submit_and_watch().await?;
		let ext_hash = in_block.extrinsic_hash();

		let tx_in_block = in_block.wait_for_finalized().await;

		let extrinsic = match tx_in_block {
			Ok(p) => p,
			Err(err) => Err(refine_subxt_error(err)).context(format!(
				"Error waiting for unsigned extrinsic in block with hash {ext_hash:?}"
			))?,
		};
		let block_hash = extrinsic.block_hash();

		match extrinsic.wait_for_success().await {
			Ok(_) => {},
			Err(err) => Err(refine_subxt_error(err))
				.context(format!("Error executing unsigned extrinsic {ext_hash:?}"))?,
		};

		let header = self
			.rpc
			.chain_get_header(Some(block_hash))
			.await?
			.ok_or_else(|| anyhow!("Inconsistent node state."))?;

		let event = EventMetadata {
			block_hash: H256::from_slice(block_hash.as_ref()),
			transaction_hash: H256::from_slice(ext_hash.as_ref()),
			block_number: header.number().into(),
		};

		Ok(event)
	}

	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, Error> {
		let key = state_machine_update_time_storage_key(height);
		let block_hash = self
			.rpc
			.chain_get_block_hash(None)
			.await?
			.ok_or_else(|| anyhow!("Failed to query latest block hash"))?;

		let raw_value =
			self.client.storage().at(block_hash).fetch_raw(key).await?.ok_or_else(|| {
				anyhow!("State machine update for {:?} not found at block {:?}", height, block_hash)
			})?;

		let value = Decode::decode(&mut &*raw_value)?;

		Ok(Duration::from_secs(value))
	}

	async fn query_challenge_period(&self, id: StateMachineId) -> Result<Duration, Error> {
		let params = rpc_params![id];
		let response: u64 = self.rpc_client.request("ismp_queryChallengePeriod", params).await?;

		Ok(Duration::from_secs(response))
	}
}
