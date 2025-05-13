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
use codec::{Decode, Encode};
use ethereum_triedb::StorageProof;
use ethers::prelude::Middleware;
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
use ethers::{
	prelude::ProviderExt,
	providers::{Http, Provider},
	utils::keccak256,
};
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
	evm_host::{EvmHost, EvmHostEvents},
	handler::{
		GetResponseLeaf, GetResponseMessage, Handler, PostRequestLeaf, PostRequestMessage,
		PostRequestTimeoutMessage, PostResponseLeaf, PostResponseMessage,
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

#[derive(Debug, Clone)]
pub struct EvmClient {
	// A WS rpc url of the EVM chain
	pub rpc_url: String,
	// Ethers provider instance
	pub client: Arc<Provider<Http>>,
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
		let client = Arc::new(Provider::<Http>::connect(&rpc_url.clone()).await);
		let host = EvmHost::new(host_address.0, client.clone());
		let handler_address = host.host_params().await?.handler;

		Ok(Self {
			rpc_url,
			client,
			state_machine,
			consensus_state_id,
			host_address,
			ismp_handler: handler_address.0.into(),
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
}

fn derive_map_key(mut key: Vec<u8>, slot: u64) -> H256 {
	let bytes = U256::from(slot as u64).to_big_endian();
	key.extend_from_slice(&bytes);
	keccak256(&key).into()
}

impl Client for EvmClient {
	async fn query_latest_block_height(&self) -> Result<u64, anyhow::Error> {
		Ok(self.client.get_block_number().await?.as_u64())
	}

	fn state_machine_id(&self) -> StateMachineId {
		StateMachineId { state_id: self.state_machine, consensus_state_id: self.consensus_state_id }
	}

	async fn query_timestamp(&self) -> Result<Duration, anyhow::Error> {
		let host = EvmHost::new(self.host_address.0, self.client.clone());
		let current_host_time = host.timestamp().call().await?;
		Ok(Duration::from_secs(current_host_time.as_u64()))
	}

	async fn query_request_receipt(&self, request_hash: H256) -> Result<H160, Error> {
		let host = EvmHost::new(self.host_address.0, self.client.clone());
		let relayer = host.request_receipts(request_hash.0).call().await?;
		Ok(relayer.0.into())
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let keys = keys
			.into_iter()
			.map(|query| self.request_commitment_key(query.commitment).0.into())
			.collect();

		let proof = self
			.client
			.get_proof(ethers::types::H160(self.host_address.0), keys, Some(at.into()))
			.await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
			storage_proof: {
				let storage_proofs = proof.storage_proof.into_iter().map(|proof| {
					StorageProof::new(proof.proof.into_iter().map(|bytes| bytes.0.into()))
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
		let keys = keys
			.into_iter()
			.map(|query| self.response_commitment_key(query.commitment).0.into())
			.collect();
		let proof = self
			.client
			.get_proof(ethers::types::H160(self.host_address.0), keys, Some(at.into()))
			.await?;
		let proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
			storage_proof: {
				let storage_proofs = proof.storage_proof.into_iter().map(|proof| {
					StorageProof::new(proof.proof.into_iter().map(|bytes| bytes.0.into()))
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
		let locations = keys.iter().map(|key| H256::from_slice(key).0.into()).collect();
		let proof = self
			.client
			.get_proof(ethers::types::H160(self.host_address.0), locations, Some(at.into()))
			.await?;
		let mut storage_proofs = vec![];
		for proof in proof.storage_proof {
			storage_proofs
				.push(StorageProof::new(proof.proof.into_iter().map(|bytes| bytes.0.into())));
		}

		let storage_proof = StorageProof::merge(storage_proofs);
		map.insert(self.host_address.0.to_vec(), storage_proof.into_nodes().into_iter().collect());

		let state_proof = EvmStateProof {
			contract_proof: proof.account_proof.into_iter().map(|bytes| bytes.0.into()).collect(),
			storage_proof: map,
		};
		Ok(state_proof.encode())
	}

	async fn query_response_receipt(&self, request_commitment: H256) -> Result<H160, Error> {
		let host = EvmHost::new(self.host_address.0, self.client.clone());
		let response_receipt = host.response_receipts(request_commitment.0).call().await?;

		Ok(response_receipt.relayer.0.into())
	}

	async fn query_ismp_event(
		&self,
		range: RangeInclusive<u64>,
	) -> Result<Vec<WithMetadata<Event>>, anyhow::Error> {
		let contract = EvmHost::new(self.host_address.0, self.client.clone());
		contract
			.events()
			.address(ethers::types::H160(self.host_address.0).into())
			.from_block(*range.start())
			.to_block(*range.end())
			.query_with_meta()
			.await?
			.into_iter()
			.map(|(event, meta)| {
				Ok(WithMetadata {
					meta: EventMetadata {
						block_hash: meta.block_hash.0.into(),
						transaction_hash: meta.transaction_hash.0.into(),
						block_number: meta.block_number.as_u64(),
					},
					event: event.try_into()?,
				})
			})
			// only care about events that can be deserialized
			.filter(|event| event.is_ok())
			.collect::<Result<Vec<_>, _>>()
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
					Ok(number) => number.low_u64(),
					Err(err) =>
						return Some((
							Err(err).context(format!(
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

				let contract = EvmHost::new(client.host_address.0, client.client.clone());
				let results = match contract
					.events()
					.address(ethers::types::H160(client.host_address.0).into())
					.from_block(latest_height)
					.to_block(block_number)
					.query_with_meta()
					.await
				{
					Ok(events) => events,
					Err(err) =>
						return Some((
							Err(err)
								.context(format!("Failed to query events on {state_machine:?}")),
							(latest_height, client),
						)),
				};

				let events = results
					.into_iter()
					.filter_map(|(ev, meta)| match ev {
						EvmHostEvents::PostRequestHandledFilter(filter) => {
							if filter.commitment == commitment.0 {
								return Some(WithMetadata {
									meta: EventMetadata {
										block_hash: meta.block_hash.0.into(),
										transaction_hash: meta.transaction_hash.0.into(),
										block_number: meta.block_number.as_u64(),
									},
									event: RequestResponseHandled {
										commitment: filter.commitment.into(),
										relayer: filter.relayer.0.to_vec(),
									},
								});
							}

							None
						},
						_ => None,
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
		let contract = EvmHost::new(self.host_address.0, self.client.clone());
		let para_id = match _state_machine.state_id {
			StateMachine::Polkadot(para_id) | StateMachine::Kusama(para_id) => para_id,
			id => Err(anyhow!("Unknown state machine id {id:?}"))?,
		};
		let height = contract.latest_state_machine_height(para_id.into()).await?;

		Ok(height.low_u64())
	}

	async fn state_machine_update_notification(
		&self,
		_counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<WithMetadata<StateMachineUpdated>>, Error> {
		let initial_height = self.client.get_block_number().await?.as_u64();
		let stream = stream::unfold(
			(initial_height, self.clone()),
			move |(latest_height, client)| async move {
				let state_machine = client.state_machine;
				tracing::trace!("Sleeping for {}", "30s");
				sleep(Duration::from_secs(30)).await;
				let block_number = match client.client.get_block_number().await {
					Ok(number) => number.low_u64(),
					Err(err) =>
						return Some((
							Err(err).context(format!(
                            "Error encountered fetching latest block number for {state_machine:?}"
                        )),
							(latest_height, client),
						)),
				};

				// in case we get old heights, best to ignore them
				if block_number < latest_height {
					return Some((Ok(None), (block_number, client)));
				}

				let contract = EvmHost::new(client.host_address.0, client.client.clone());
				let results = match contract
					.events()
					.address(ethers::types::H160(client.host_address.0).into())
					.from_block(latest_height)
					.to_block(block_number)
					.query_with_meta()
					.await
				{
					Ok(events) => events,
					Err(err) =>
						return Some((
							Err(err)
								.context(format!("Failed to query events on {state_machine:?}")),
							(latest_height, client),
						)),
				};
				let mut events = results
					.into_iter()
					.filter_map(|(ev, meta)| {
						let Event::StateMachineUpdated(event) = ev.try_into().ok()? else { None? };
						Some(WithMetadata {
							meta: EventMetadata {
								block_hash: meta.block_hash.0.into(),
								transaction_hash: meta.transaction_hash.0.into(),
								block_number: meta.block_number.as_u64(),
							},
							event,
						})
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
		let timestamp = {
			let timestamp = self
				.client
				.get_storage_at(
					ethers::types::H160(self.host_address.0),
					timestamp_key.0.into(),
					None,
				)
				.await?;
			U256::from_big_endian(timestamp.as_bytes()).low_u64()
		};
		let overlay_root = self
			.client
			.get_storage_at(ethers::types::H160(self.host_address.0), overlay_key.0.into(), None)
			.await?;
		let state_root = self
			.client
			.get_storage_at(ethers::types::H160(self.host_address.0), state_root_key.0.into(), None)
			.await?;
		Ok(StateCommitment {
			timestamp,
			overlay_root: Some(overlay_root.0.into()),
			state_root: state_root.0.into(),
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
		let contract = Handler::new(self.ismp_handler.0, self.client.clone());
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
						state_machine_id: {
							match timeout_proof.height.id.state_id {
								StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
								_ => Err(anyhow!("Expected polkadot or kusama state machines"))?,
							}
						},
						height: timeout_proof.height.height.into(),
					},
					proof: state_proof.storage_proof().into_iter().map(|key| key.into()).collect(),
				};
				let call =
					contract.handle_post_request_timeouts(self.host_address.0.into(), message);

				Ok(call.tx.data().cloned().expect("Infallible").to_vec())
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
						state_machine_id: {
							match timeout_proof.height.id.state_id {
								StateMachine::Polkadot(id) | StateMachine::Kusama(id) => id.into(),
								_ => Err(anyhow!("Expected polkadot or kusama state machines"))?,
							}
						},
						height: timeout_proof.height.height.into(),
					},
					proof: state_proof.storage_proof().into_iter().map(|key| key.into()).collect(),
				};
				let call =
					contract.handle_post_response_timeouts(self.host_address.0.into(), message);
				Ok(call.tx.data().cloned().expect("Infallible").to_vec())
			},
			// Message::Timeout(TimeoutMessage::Get { requests }) => {
			// 	let get_requests = requests
			// 		.into_iter()
			// 		.filter_map(|req| match req {
			// 			Request::Get(get) => Some(get.into()),
			// 			_ => None,
			// 		})
			// 		.collect();

			// 	let message = GetTimeoutMessage { timeouts: get_requests };
			// 	let call = contract.handle_get_request_timeouts(self.host_address, message);

			// 	Ok(call.tx.data().cloned().expect("Infallible").to_vec())
			// },
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
						k_index: k_index.into(),
					})
					.collect::<Vec<_>>();
				leaves.sort_by_key(|leaf| leaf.index);
				let post_message = PostRequestMessage {
					proof: Proof {
						height: ismp_solidity_abi::shared_types::StateMachineHeight {
							state_machine_id: {
								match msg.proof.height.id.state_id {
									StateMachine::Polkadot(id) | StateMachine::Kusama(id) =>
										id.into(),
									_ =>
										Err(anyhow!("Expected polkadot or kusama state machines"))?,
								}
							},
							height: msg.proof.height.height.into(),
						},
						multiproof: membership_proof.items.into_iter().map(|node| node.0).collect(),
						leaf_count: membership_proof.leaf_count.into(),
					},
					requests: leaves,
				};

				let call = contract.handle_post_requests(self.host_address.0.into(), post_message);
				Ok(call.tx.data().cloned().expect("Infallible").to_vec())
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
												k_index: k_index.into(),
											}),
											_ => None,
										})
										.collect::<Vec<_>>();
									leaves.sort_by_key(|leaf| leaf.index);
									let message = PostResponseMessage {
									proof: Proof {
										height: ismp_solidity_abi::shared_types::StateMachineHeight {
											state_machine_id: {
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
											.map(|node| node.0)
											.collect(),
										leaf_count: membership_proof.leaf_count.into(),
									},
									responses: leaves,
								};

									let call = contract
										.handle_post_responses(self.host_address.0.into(), message);
									call.tx.data().cloned().expect("Infallible").to_vec()
								},
								Response::Get(_) => {
									let mut leaves = responses
										.into_iter()
										.zip(k_and_leaf_indices)
										.filter_map(|(res, (k_index, leaf_index))| match res {
											Response::Get(res) => Some(GetResponseLeaf {
												response: res.into(),
												index: leaf_index.into(),
												k_index: k_index.into(),
											}),
											_ => None,
										})
										.collect::<Vec<_>>();
									leaves.sort_by_key(|leaf| leaf.index);
									let message = GetResponseMessage {
										proof: Proof {
											height: ismp_solidity_abi::shared_types::StateMachineHeight {
												state_machine_id: {
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
												.map(|node| node.0)
												.collect(),
											leaf_count: membership_proof.leaf_count.into(),
										},
										responses: leaves,
									};

									let call = contract
										.handle_get_responses(self.host_address.0.into(), message);
									call.tx.data().cloned().expect("Infallible").to_vec()
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
		let contract = EvmHost::new(self.host_address.0, self.client.clone());
		let value =
			contract.state_machine_commitment_update_time(height.try_into()?).call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}

	async fn query_challenge_period(&self, _id: StateMachineId) -> Result<Duration, Error> {
		let contract = EvmHost::new(self.host_address.0, self.client.clone());
		let value = contract.challenge_period().call().await?;
		Ok(Duration::from_secs(value.low_u64()))
	}
}
