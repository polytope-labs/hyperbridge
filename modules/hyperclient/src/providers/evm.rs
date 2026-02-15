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

use crate::{providers::interface::Client, types::BoxStream};
use alloy::{
	primitives::{keccak256, Address, B256, U256 as AlloyU256},
	providers::{Provider, ProviderBuilder, RootProvider},
	rpc::types::{Filter, Log},
	transports::http::{Client as HttpClient, Http},
};
use codec::{Decode, Encode};
use ethereum_triedb::StorageProof;
use evm_state_machine::{
	presets::{
		REQUEST_COMMITMENTS_SLOT, REQUEST_RECEIPTS_SLOT, RESPONSE_COMMITMENTS_SLOT,
		RESPONSE_RECEIPTS_SLOT,
	},
	state_comitment_key,
};
use polkadot_sdk::sp_mmr_primitives::utils::NodesUtils;

use super::interface::Query;
use crate::{
	providers::interface::WithMetadata,
	types::{EventMetadata, SubstrateStateProof},
};
use anyhow::{anyhow, Context, Error};
use core::time::Duration;
use evm_state_machine::types::EvmStateProof;
use futures::{stream, StreamExt};
use ismp::{
	consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
	events::{Event, RequestResponseHandled, StateMachineUpdated},
	host::StateMachine,
	messaging::{Message, ResponseMessage, TimeoutMessage},
	router::{Request, RequestResponse, Response},
};
use ismp_solidity_abi::{
	evm_host::{EvmHost, EvmHostEvents, EvmHostInstance},
	handler::{
		GetResponseLeaf, GetResponseMessage, Handler, HandlerInstance, PostRequestLeaf,
		PostRequestMessage, PostRequestTimeoutMessage, PostResponseLeaf, PostResponseMessage,
		PostResponseTimeoutMessage, Proof,
	},
};
use mmr_primitives::mmr_position_to_k_index;
use pallet_ismp::offchain::{LeafIndexAndPos, Proof as MmrProof};
use primitive_types::{H160, H256, U256};
use std::{collections::BTreeMap, ops::RangeInclusive, sync::Arc};

#[cfg(all(target_arch = "wasm32", feature = "web"))]
use gloo_timers::future::*;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::*;
#[cfg(all(target_arch = "wasm32", feature = "nodejs"))]
use wasmtimer::tokio::*;

/// Alloy provider type alias
pub type AlloyProvider = RootProvider<Http<HttpClient>>;

#[derive(Debug, Clone)]
pub struct EvmClient {
	// A WS rpc url of the EVM chain
	pub rpc_url: String,
	// Alloy provider instance
	pub client: Arc<AlloyProvider>,
	// Identifies the state machine this EVM client represents
	pub state_machine: StateMachine,
	// This is the Consensus State ID of the chain (e.g. BSC0)
	pub consensus_state_id: ConsensusStateId,
	// Address of the ISMP host of this state machine
	pub host_address: H160,
	// The ISMP handler address
	pub ismp_handler: H160,
}

impl EvmClient {
	// Creates an instance of an EVM client
	pub async fn new(
		rpc_url: String,
		consensus_state_id: ConsensusStateId,
		host_address: H160,
		state_machine: StateMachine,
	) -> Result<Self, anyhow::Error> {
		let url = rpc_url.parse()?;
		let client = Arc::new(ProviderBuilder::new().on_http(url));
		let host_addr: Address = Address::from_slice(&host_address.0);
		let host = EvmHostInstance::new(host_addr, client.clone());
		let host_params = host.hostParams().call().await?;
		let handler_address: H160 = host_params._0.handler.0 .0.into();

		Ok(Self {
			rpc_url,
			client,
			state_machine,
			consensus_state_id,
			host_address,
			ismp_handler: handler_address,
		})
	}

	pub fn request_commitment_key(&self, key: H256) -> H256 {
		let key = derive_map_key(key.0.to_vec(), REQUEST_COMMITMENTS_SLOT);
		let number = U256::from_big_endian(key.0.as_slice()) + U256::from(1);
		let bytes = number.to_big_endian();
		H256::from(bytes)
	}

	pub fn response_commitment_key(&self, key: H256) -> H256 {
		let key = derive_map_key(key.0.to_vec(), RESPONSE_COMMITMENTS_SLOT);
		let number = U256::from_big_endian(key.0.as_slice()) + U256::from(1);
		let bytes = number.to_big_endian();
		H256::from(bytes)
	}

	pub fn request_receipt_key(&self, key: H256) -> H256 {
		derive_map_key(key.0.to_vec(), REQUEST_RECEIPTS_SLOT)
	}

	pub fn response_receipt_key(&self, key: H256) -> H256 {
		derive_map_key(key.0.to_vec(), RESPONSE_RECEIPTS_SLOT)
	}

	fn address_to_h160(addr: Address) -> H160 {
		H160::from_slice(addr.as_slice())
	}

	fn h160_to_address(&self, h: H160) -> Address {
		Address::from_slice(&h.0)
	}

	fn h256_to_b256(h: H256) -> B256 {
		B256::from_slice(&h.0)
	}

	fn b256_to_h256(b: B256) -> H256 {
		H256::from_slice(b.as_slice())
	}
}

fn derive_map_key(mut key: Vec<u8>, slot: u64) -> H256 {
	let bytes = U256::from(slot as u64).to_big_endian();
	key.extend_from_slice(&bytes);
	let hash = keccak256(&key);
	H256::from_slice(hash.as_slice())
}

fn parse_ismp_event(log: &Log) -> Option<(EvmHostEvents, EventMetadata)> {
	let meta = EventMetadata {
		block_hash: H256::from_slice(log.block_hash?.as_slice()),
		transaction_hash: H256::from_slice(log.transaction_hash?.as_slice()),
		block_number: log.block_number?,
	};

	let event = EvmHostEvents::decode_log(log.inner.clone(), true).ok()?;
	Some((event, meta))
}

impl Client for EvmClient {
	async fn query_latest_block_height(&self) -> Result<u64, anyhow::Error> {
		Ok(self.client.get_block_number().await?)
	}

	fn state_machine_id(&self) -> StateMachineId {
		StateMachineId { state_id: self.state_machine, consensus_state_id: self.consensus_state_id }
	}

	async fn query_timestamp(&self) -> Result<Duration, anyhow::Error> {
		let host_addr = self.h160_to_address(self.host_address);
		let host = EvmHostInstance::new(host_addr, self.client.clone());
		let result = host.timestamp().call().await?;
		Ok(Duration::from_secs(result._0.to::<u64>()))
	}

	async fn query_request_receipt(&self, request_hash: H256) -> Result<H160, Error> {
		let host_addr = self.h160_to_address(self.host_address);
		let host = EvmHostInstance::new(host_addr, self.client.clone());
		let result = host.requestReceipts(Self::h256_to_b256(request_hash)).call().await?;
		Ok(Self::address_to_h160(result._0))
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let host_addr = self.h160_to_address(self.host_address);
		let keys: Vec<B256> = keys
			.into_iter()
			.map(|query| Self::h256_to_b256(self.request_commitment_key(query.commitment)))
			.collect();

		let proof = self.client.get_proof(host_addr, keys).block_id(at.into()).await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.to_vec()).collect(),
			storage_proof: {
				let storage_proofs = proof.storage_proof.into_iter().map(|proof| {
					StorageProof::new(proof.proof.into_iter().map(|bytes| bytes.to_vec()))
				});
				let merged_proofs = StorageProof::merge(storage_proofs);
				vec![(
					self.host_address.0.to_vec(),
					merged_proofs.into_nodes().into_iter().collect(),
				)]
				.into_iter()
				.collect()
			},
		};
		Ok(proof.encode())
	}

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let host_addr = self.h160_to_address(self.host_address);
		let keys: Vec<B256> = keys
			.into_iter()
			.map(|query| Self::h256_to_b256(self.response_commitment_key(query.commitment)))
			.collect();

		let proof = self.client.get_proof(host_addr, keys).block_id(at.into()).await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.to_vec()).collect(),
			storage_proof: {
				let storage_proofs = proof.storage_proof.into_iter().map(|proof| {
					StorageProof::new(proof.proof.into_iter().map(|bytes| bytes.to_vec()))
				});
				let merged_proofs = StorageProof::merge(storage_proofs);
				vec![(
					self.host_address.0.to_vec(),
					merged_proofs.into_nodes().into_iter().collect(),
				)]
				.into_iter()
				.collect()
			},
		};
		Ok(proof.encode())
	}

	async fn query_state_proof(&self, at: u64, keys: Vec<Vec<u8>>) -> Result<Vec<u8>, Error> {
		use codec::Encode;
		let mut map: BTreeMap<Vec<u8>, Vec<Vec<u8>>> = BTreeMap::new();
		let host_addr = self.h160_to_address(self.host_address);
		let locations: Vec<B256> =
			keys.iter().map(|key| B256::from_slice(key.as_slice())).collect();

		let proof = self.client.get_proof(host_addr, locations).block_id(at.into()).await?;
		let mut storage_proofs = vec![];
		for proof in proof.storage_proof {
			storage_proofs
				.push(StorageProof::new(proof.proof.into_iter().map(|bytes| bytes.to_vec())));
		}

		let storage_proof = StorageProof::merge(storage_proofs);
		map.insert(self.host_address.0.to_vec(), storage_proof.into_nodes().into_iter().collect());

		let state_proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.to_vec()).collect(),
			storage_proof: map,
		};
		Ok(state_proof.encode())
	}

	async fn query_response_receipt(&self, request_commitment: H256) -> Result<H160, Error> {
		let host_addr = self.h160_to_address(self.host_address);
		let host = EvmHostInstance::new(host_addr, self.client.clone());
		let result = host.responseReceipts(Self::h256_to_b256(request_commitment)).call().await?;
		Ok(Self::address_to_h160(result.relayer))
	}

	async fn query_ismp_event(
		&self,
		range: RangeInclusive<u64>,
	) -> Result<Vec<WithMetadata<Event>>, anyhow::Error> {
		let host_addr = self.h160_to_address(self.host_address);
		let filter = Filter::new()
			.address(host_addr)
			.from_block(*range.start())
			.to_block(*range.end());

		let logs = self.client.get_logs(&filter).await?;

		logs.into_iter()
			.filter_map(|log| {
				let (event, meta) = parse_ismp_event(&log)?;
				let ismp_event: Result<Event, _> = event.try_into();
				ismp_event.ok().map(|e| {
					Ok(WithMetadata { meta, event: e })
				})
			})
			.collect::<Result<Vec<_>, anyhow::Error>>()
	}

	async fn ismp_events_stream(
		&self,
		_commitment: H256,
		_initial_height: u64,
	) -> Result<BoxStream<WithMetadata<Event>>, Error> {
		Err(anyhow!("Ismp stream unavailable for evm client"))
	}

	async fn post_request_handled_stream(
		&self,
		commitment: H256,
		initial_height: u64,
	) -> Result<BoxStream<WithMetadata<RequestResponseHandled>>, Error> {
		let client = self.clone();
		let stream =
			stream::unfold((initial_height, client), move |(latest_height, client)| async move {
				let state_machine = client.state_machine;
				tracing::trace!("Sleeping for {}", "12s");
				sleep(Duration::from_secs(12)).await;
				let block_number = match client.client.get_block_number().await {
					Ok(number) => number,
					Err(err) =>
						return Some((
							Err(anyhow::anyhow!("{}", err)).context(format!(
                            "Error encountered fetching latest block number for {state_machine:?}"
                        )),
							(latest_height, client),
						)),
				};
				tracing::trace!(
					"Starting to query for PostRequestHandled: {initial_height}..{block_number}"
				);

				// in case we get old heights, best to ignore them
				if block_number < latest_height {
					return Some((Ok(None), (block_number, client)));
				}

				let host_addr = client.h160_to_address(client.host_address);
				let filter = Filter::new()
					.address(host_addr)
					.from_block(latest_height)
					.to_block(block_number);

				let logs = match client.client.get_logs(&filter).await {
					Ok(logs) => logs,
					Err(err) =>
						return Some((
							Err(anyhow::anyhow!("{}", err))
								.context(format!("Failed to query events on {state_machine:?}")),
							(latest_height, client),
						)),
				};

				let events = logs
					.into_iter()
					.filter_map(|log| {
						let (ev, meta) = parse_ismp_event(&log)?;
						match ev {
							EvmHostEvents::PostRequestHandled(filter) => {
								let event_commitment = H256::from_slice(filter.commitment.as_slice());
								if event_commitment == commitment {
									return Some(WithMetadata {
										meta,
										event: RequestResponseHandled {
											commitment: event_commitment,
											relayer: filter.relayer.as_slice().to_vec(),
										},
									});
								}
								None
							},
							_ => None,
						}
					})
					.collect::<Vec<_>>();

				// we only want the highest event
				Some((Ok(events.last().cloned()), (block_number + 1, client)))
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

	async fn query_latest_state_machine_height(
		&self,
		_state_machine: StateMachineId,
	) -> Result<u64, anyhow::Error> {
		let host_addr = self.h160_to_address(self.host_address);
		let host = EvmHostInstance::new(host_addr, self.client.clone());
		let para_id = match _state_machine.state_id {
			StateMachine::Polkadot(para_id) | StateMachine::Kusama(para_id) => para_id,
			id => Err(anyhow!("Unknown state machine id {id:?}"))?,
		};
		let height = host.latestStateMachineHeight(para_id.into()).call().await?;

		Ok(height._0.to::<u64>())
	}

	async fn state_machine_update_notification(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<WithMetadata<StateMachineUpdated>>, Error> {
		let initial_height = self.client.get_block_number().await?;
		let stream = stream::unfold(
			(initial_height, self.clone()),
			move |(latest_height, client)| async move {
				let state_machine = client.state_machine;
				tracing::trace!("Sleeping for {}", "30s");
				sleep(Duration::from_secs(30)).await;
				let block_number = match client.client.get_block_number().await {
					Ok(number) => number,
					Err(err) =>
						return Some((
							Err(anyhow::anyhow!("{}", err)).context(format!(
                            "Error encountered fetching latest block number for {state_machine:?}"
                        )),
							(latest_height, client),
						)),
				};

				// in case we get old heights, best to ignore them
				if block_number < latest_height {
					return Some((Ok(None), (block_number, client)));
				}

				let host_addr = client.h160_to_address(client.host_address);
				let filter = Filter::new()
					.address(host_addr)
					.from_block(latest_height)
					.to_block(block_number);

				let logs = match client.client.get_logs(&filter).await {
					Ok(logs) => logs,
					Err(err) =>
						return Some((
							Err(anyhow::anyhow!("{}", err))
								.context(format!("Failed to query events on {state_machine:?}")),
							(latest_height, client),
						)),
				};

				let mut events = logs
					.into_iter()
					.filter_map(|log| {
						let (ev, meta) = parse_ismp_event(&log)?;
						let Event::StateMachineUpdated(event) = ev.try_into().ok()? else { return None };
						Some(WithMetadata { meta, event })
					})
					.collect::<Vec<_>>();
				// we only want the highest event
				events.sort_by(|a, b| a.event.latest_height.cmp(&b.event.latest_height));
				// we only want the highest event
				Some((Ok(events.last().cloned()), (block_number + 1, client)))
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

	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		let id = match height.id.state_id {
			StateMachine::Polkadot(para_id) => para_id,
			StateMachine::Kusama(para_id) => para_id,
			_ => Err(anyhow!(
				"Unknown State Machine: {:?} Expected polkadot or kusama state machine",
				height.id.state_id
			))?,
		};
		let (timestamp_key, overlay_key, state_root_key) =
			state_comitment_key(id.into(), height.height.into());
		let host_addr = self.h160_to_address(self.host_address);

		let timestamp = {
			let timestamp = self
				.client
				.get_storage_at(host_addr, Self::h256_to_b256(timestamp_key).into())
				.await?;
			U256::from_big_endian(&timestamp.to_be_bytes::<32>()).low_u64()
		};
		let overlay_root = self
			.client
			.get_storage_at(host_addr, Self::h256_to_b256(overlay_key).into())
			.await?;
		let state_root = self
			.client
			.get_storage_at(host_addr, Self::h256_to_b256(state_root_key).into())
			.await?;
		Ok(StateCommitment {
			timestamp,
			overlay_root: Some(H256::from_slice(&overlay_root.to_be_bytes::<32>())),
			state_root: H256::from_slice(&state_root.to_be_bytes::<32>()),
		})
	}

	fn request_commitment_full_key(&self, commitment: H256) -> Vec<u8> {
		self.request_commitment_key(commitment).0.to_vec()
	}

	fn request_receipt_full_key(&self, commitment: H256) -> Vec<u8> {
		self.request_receipt_key(commitment).0.to_vec()
	}

	fn response_commitment_full_key(&self, commitment: H256) -> Vec<u8> {
		self.response_commitment_key(commitment).0.to_vec()
	}

	fn response_receipt_full_key(&self, commitment: H256) -> Vec<u8> {
		self.response_receipt_key(commitment).0.to_vec()
	}

	fn encode(&self, msg: Message) -> Result<Vec<u8>, Error> {
		let handler_addr = self.h160_to_address(self.ismp_handler);
		let host_addr = self.h160_to_address(self.host_address);
		let contract = HandlerInstance::new(handler_addr, self.client.clone());
		match msg {
			Message::Timeout(TimeoutMessage::Post { timeout_proof, requests }) => {
				let post_requests = requests
					.into_iter()
					.filter_map(|req| match req {
						Request::Post(post) => Some(post.into()),
						Request::Get(_) => None,
					})
					.collect();

				let state_proof: SubstrateStateProof =
					match codec::Decode::decode(&mut timeout_proof.proof.as_slice()) {
						Ok(proof) => proof,
						_ => Err(anyhow!("Error decoding proof"))?,
					};
				let message = PostRequestTimeoutMessage {
					timeouts: post_requests,
					height: ismp_solidity_abi::shared_types::StateMachineHeight {
						stateMachineId: {
							match timeout_proof.height.id.state_id {
								StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
								_ => Err(anyhow!("Expected polkadot or kusama state machines"))?,
							}
						},
						height: timeout_proof.height.height.into(),
					},
					proof: state_proof.storage_proof().into_iter().map(|key| key.into()).collect(),
				};
				let call = contract.handlePostRequestTimeouts(host_addr, message);

				Ok(call.calldata().to_vec())
			},
			Message::Timeout(TimeoutMessage::PostResponse { timeout_proof, responses }) => {
				let post_responses = responses.into_iter().map(|res| res.into()).collect();

				let state_proof: SubstrateStateProof =
					match codec::Decode::decode(&mut timeout_proof.proof.as_slice()) {
						Ok(proof) => proof,
						_ => Err(anyhow!("Expected polkadot or kusama state machines"))?,
					};
				let message = PostResponseTimeoutMessage {
					timeouts: post_responses,
					height: ismp_solidity_abi::shared_types::StateMachineHeight {
						stateMachineId: {
							match timeout_proof.height.id.state_id {
								StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
								_ => Err(anyhow!("Expected polkadot or kusama state machines"))?,
							}
						},
						height: timeout_proof.height.height.into(),
					},
					proof: state_proof.storage_proof().into_iter().map(|key| key.into()).collect(),
				};
				let call = contract.handlePostResponseTimeouts(host_addr, message);
				Ok(call.calldata().to_vec())
			},
			Message::Request(msg) => {
				let membership_proof = MmrProof::<H256>::decode(&mut msg.proof.proof.as_slice())?;
				let mmr_size = NodesUtils::new(membership_proof.leaf_count).size();
				let k_and_leaf_indices = membership_proof
					.leaf_indices_and_pos
					.into_iter()
					.map(|LeafIndexAndPos { pos, leaf_index }| {
						let k_index = mmr_position_to_k_index(vec![pos], mmr_size)[0].1;
						(k_index, leaf_index)
					})
					.collect::<Vec<_>>();

				let mut leaves = msg
					.requests
					.into_iter()
					.zip(k_and_leaf_indices)
					.map(|(post, (k_index, leaf_index))| PostRequestLeaf {
						request: post.into(),
						index: leaf_index.into(),
						kIndex: k_index.into(),
					})
					.collect::<Vec<_>>();
				leaves.sort_by_key(|leaf| leaf.index);
				let post_message = PostRequestMessage {
					proof: Proof {
						height: ismp_solidity_abi::shared_types::StateMachineHeight {
							stateMachineId: {
								match msg.proof.height.id.state_id {
									StateMachine::Polkadot(id) | StateMachine::Kusama(id) =>
										id.into(),
									_ =>
										Err(anyhow!("Expected polkadot or kusama state machines"))?,
								}
							},
							height: msg.proof.height.height.into(),
						},
						multiproof: membership_proof.items.into_iter().map(|node| node.0.into()).collect(),
						leafCount: membership_proof.leaf_count.into(),
					},
					requests: leaves,
				};

				let call = contract.handlePostRequests(host_addr, post_message);
				Ok(call.calldata().to_vec())
			},
			Message::Response(ResponseMessage { datagram, proof, .. }) => {
				let membership_proof = MmrProof::<H256>::decode(&mut proof.proof.as_slice())?;
				let mmr_size = NodesUtils::new(membership_proof.leaf_count).size();
				let k_and_leaf_indices = membership_proof
					.leaf_indices_and_pos
					.into_iter()
					.map(|LeafIndexAndPos { pos, leaf_index }| {
						let k_index = mmr_position_to_k_index(vec![pos], mmr_size)[0].1;
						(k_index, leaf_index)
					})
					.collect::<Vec<_>>();

				match datagram {
					RequestResponse::Response(responses) => {
						let calldata =
							match responses[0] {
								Response::Post(_) => {
									let mut leaves = responses
										.into_iter()
										.zip(k_and_leaf_indices)
										.filter_map(|(res, (k_index, leaf_index))| match res {
											Response::Post(res) => Some(PostResponseLeaf {
												response: res.into(),
												index: leaf_index.into(),
												kIndex: k_index.into(),
											}),
											_ => None,
										})
										.collect::<Vec<_>>();
									leaves.sort_by_key(|leaf| leaf.index);
									let message = PostResponseMessage {
									proof: Proof {
										height: ismp_solidity_abi::shared_types::StateMachineHeight {
											stateMachineId: {
												match proof.height.id.state_id {
													StateMachine::Polkadot(id)
													| StateMachine::Kusama(id) => id.into(),
													_ => Err(anyhow!(
														"Expected polkadot or kusama state machines"
													))?,
												}
											},
											height: proof.height.height.into(),
										},
										multiproof: membership_proof
											.items
											.into_iter()
											.map(|node| node.0.into())
											.collect(),
										leafCount: membership_proof.leaf_count.into(),
									},
									responses: leaves,
								};

									let call = contract.handlePostResponses(host_addr, message);
									call.calldata().to_vec()
								},
								Response::Get(_) => {
									let mut leaves = responses
										.into_iter()
										.zip(k_and_leaf_indices)
										.filter_map(|(res, (k_index, leaf_index))| match res {
											Response::Get(res) => Some(GetResponseLeaf {
												response: res.into(),
												index: leaf_index.into(),
												kIndex: k_index.into(),
											}),
											_ => None,
										})
										.collect::<Vec<_>>();
									leaves.sort_by_key(|leaf| leaf.index);
									let message = GetResponseMessage {
										proof: Proof {
											height: ismp_solidity_abi::shared_types::StateMachineHeight {
												stateMachineId: {
													match proof.height.id.state_id {
														StateMachine::Polkadot(id)
														| StateMachine::Kusama(id) => id.into(),
														_ => Err(anyhow!(
															"Expected polkadot or kusama state machines"
														))?,
													}
												},
												height: proof.height.height.into(),
											},
											multiproof: membership_proof
												.items
												.into_iter()
												.map(|node| node.0.into())
												.collect(),
											leafCount: membership_proof.leaf_count.into(),
										},
										responses: leaves,
									};

									let call = contract.handleGetResponses(host_addr, message);
									call.calldata().to_vec()
								},
							};
						Ok(calldata)
					},
					RequestResponse::Request(..) => Err(anyhow!("Get requests cannot be relayed"))?,
				}
			},
			_ => Err(anyhow!("Unsupported message"))?,
		}
	}

	async fn submit(&self, _msg: Message) -> Result<EventMetadata, Error> {
		Err(anyhow!("Client cannot submit messages"))
	}

	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, Error> {
		let host_addr = self.h160_to_address(self.host_address);
		let host = EvmHostInstance::new(host_addr, self.client.clone());
		let value = host.stateMachineCommitmentUpdateTime(height.try_into()?).call().await?;
		Ok(Duration::from_secs(value._0.to::<u64>()))
	}

	async fn query_challenge_period(&self, _id: StateMachineId) -> Result<Duration, Error> {
		let host_addr = self.h160_to_address(self.host_address);
		let host = EvmHostInstance::new(host_addr, self.client.clone());
		let value = host.challengePeriod().call().await?;
		Ok(Duration::from_secs(value._0.to::<u64>()))
	}
}
