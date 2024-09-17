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

//! [`IsmpProvider`] implementation

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{anyhow, Error};
use codec::{Decode, Encode};
use futures::{stream::FuturesOrdered, FutureExt};
use hex_literal::hex;
use ismp::{
	consensus::{ConsensusClientId, StateCommitment, StateMachineHeight, StateMachineId},
	events::{Event, StateCommitmentVetoed},
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use pallet_ismp::{
	child_trie::{
		request_commitment_storage_key, response_commitment_storage_key, CHILD_TRIE_PREFIX,
	},
	mmr::ProofKeys,
};
use pallet_ismp_host_executive::HostParam;
use pallet_ismp_relayer::withdrawal::Signature;
use pallet_ismp_rpc::BlockNumberOrHash;
use subxt::ext::sp_core::{
	storage::{ChildInfo, StorageData, StorageKey},
	Pair, H160, H256, U256,
};

use substrate_state_machine::{StateMachineProof, SubstrateStateProof};
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
	},
	ext::{
		sp_core::crypto::AccountId32,
		sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
	},
	rpc::types::DryRunResult,
	rpc_params,
	tx::TxPayload,
};

use subxt_utils::{host_params_storage_key, send_extrinsic, state_machine_update_time_storage_key};
use tesseract_primitives::{
	wait_for_challenge_period, BoxStream, EstimateGasReturnParams, IsmpProvider, Query,
	StateMachineUpdated, StateProofQueryType, TxReceipt,
};

use crate::{
	calls::RequestMetadata,
	extrinsic::{send_unsigned_extrinsic, system_dry_run_unsigned, Extrinsic, InMemorySigner},
	SubstrateClient,
};

#[async_trait::async_trait]
impl<C> IsmpProvider for SubstrateClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::AccountId: From<AccountId32> + Into<C::Address> + Clone + Send + Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
	H256: From<<C as subxt::Config>::Hash>,
{
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		id: ConsensusClientId,
	) -> Result<Vec<u8>, anyhow::Error> {
		let params = rpc_params![at, id];
		let response = self.client.rpc().request("ismp_queryConsensusState", params).await?;

		Ok(response)
	}

	async fn query_latest_height(&self, id: StateMachineId) -> Result<u32, anyhow::Error> {
		let params = rpc_params![id];
		let response =
			self.client.rpc().request("ismp_queryStateMachineLatestHeight", params).await?;

		Ok(response)
	}

	async fn query_finalized_height(&self) -> Result<u64, anyhow::Error> {
		let finalized = self.client.rpc().finalized_head().await?;
		let block = self
			.client
			.rpc()
			.header(Some(finalized))
			.await?
			.ok_or_else(|| anyhow!("Finalized header should exist {finalized:?}"))?;
		Ok(block.number().into())
	}

	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, anyhow::Error> {
		let key = state_machine_update_time_storage_key(height);
		let block = self.client.blocks().at_latest().await?;
		let raw_value =
			self.client.storage().at(block.hash()).fetch_raw(&key).await?.ok_or_else(|| {
				anyhow!(
					"State machine update for {:?} not found at block {:?}",
					height,
					block.hash()
				)
			})?;

		let value = Decode::decode(&mut &*raw_value)?;

		Ok(Duration::from_secs(value))
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
		// We use the counterparty chain's state machine id to know what kind of proof is required
		// Necessary for when substrate chains are using tesseract to communicate with hyperbridge
		// The destination chain in the request does not reflect the kind of proof needed
		match counterparty {
			// Use mmr proofs for queries going to EVM chains
			s if s.is_evm() => {
				let keys =
					ProofKeys::Requests(keys.into_iter().map(|key| key.commitment).collect());
				let params = rpc_params![at, keys];
				let response: pallet_ismp_rpc::Proof =
					self.client.rpc().request("ismp_queryMmrProof", params).await?;
				Ok(response.proof)
			},
			// Use child trie proofs for queries going to substrate chains
			s if s.is_substrate() => {
				let keys: Vec<_> = keys
					.into_iter()
					.map(|key| request_commitment_storage_key(key.commitment))
					.collect();
				let params = rpc_params![at, keys];
				let response: pallet_ismp_rpc::Proof =
					self.client.rpc().request("ismp_queryChildTrieProof", params).await?;
				let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*response.proof)?;
				let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
					hasher: self.hashing.clone(),
					storage_proof,
				});
				Ok(proof.encode())
			},
			s => Err(anyhow::anyhow!("Unsupported state machine {s:?}!")),
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
				let response: pallet_ismp_rpc::Proof =
					self.client.rpc().request("ismp_queryMmrProof", params).await?;
				Ok(response.proof)
			},
			// Use child trie proofs for queries going to substrate chains
			s if s.is_substrate() => {
				let keys: Vec<_> = keys
					.into_iter()
					.map(|key| response_commitment_storage_key(key.commitment))
					.collect();
				let params = rpc_params![at, keys];
				let response: pallet_ismp_rpc::Proof =
					self.client.rpc().request("ismp_queryChildTrieProof", params).await?;
				let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*response.proof)?;
				let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
					hasher: self.hashing.clone(),
					storage_proof,
				});
				Ok(proof.encode())
			},
			s => Err(anyhow::anyhow!("Unsupported state machine {s:?}!")),
		}
	}

	async fn query_state_proof(
		&self,
		at: u64,
		keys: StateProofQueryType,
	) -> Result<Vec<u8>, anyhow::Error> {
		match keys {
			StateProofQueryType::Ismp(keys) => {
				let params = rpc_params![at, keys];
				let response: pallet_ismp_rpc::Proof =
					self.client.rpc().request("ismp_queryChildTrieProof", params).await?;
				let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*response.proof)?;
				let proof = SubstrateStateProof::OverlayProof(StateMachineProof {
					hasher: self.hashing.clone(),
					storage_proof,
				});
				Ok(proof.encode())
			},
			StateProofQueryType::Arbitrary(keys) => {
				let params = rpc_params![at, keys];
				let response: pallet_ismp_rpc::Proof =
					self.client.rpc().request("ismp_queryStateProof", params).await?;

				let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*response.proof)?;
				let proof = SubstrateStateProof::StateProof(StateMachineProof {
					hasher: self.hashing.clone(),
					storage_proof,
				});
				Ok(proof.encode())
			},
		}
	}

	async fn query_ismp_events(
		&self,
		previous_height: u64,
		event: StateMachineUpdated,
	) -> Result<Vec<Event>, anyhow::Error> {
		let range = (previous_height + 1)..=event.latest_height;
		if range.is_empty() {
			return Ok(Default::default());
		}

		let mut events = vec![];
		let chunk_size = 100;
		let chunks = range.end().saturating_sub(*range.start()) / chunk_size;
		for i in 0..=chunks {
			let start = (i * chunk_size) + *range.start();
			let end = if i == chunks { *range.end() } else { start + chunk_size - 1 };
			let params = rpc_params![
				BlockNumberOrHash::<H256>::Number(start as u32),
				BlockNumberOrHash::<H256>::Number(end as u32)
			];
			let response = self
				.client
				.rpc()
				.request::<HashMap<String, Vec<Event>>>("ismp_queryEvents", params)
				.await;
			match response {
				Ok(response) => {
					let batch = response.values().into_iter().cloned().flatten();
					events.extend(batch)
				},
				Err(err) => {
					log::error!(
						"Error while querying events in range {}..{} from {:?}: {err:?}",
						start,
						end,
						self.state_machine
					);
				},
			}
		}

		Ok(events)
	}

	fn name(&self) -> String {
		format!("{:?}", self.state_machine)
	}

	fn state_machine_id(&self) -> StateMachineId {
		StateMachineId { state_id: self.state_machine, consensus_state_id: self.consensus_state_id }
	}

	fn block_max_gas(&self) -> u64 {
		Default::default()
	}

	fn initial_height(&self) -> u64 {
		self.initial_height
	}

	async fn estimate_gas(
		&self,
		messages: Vec<ismp::messaging::Message>,
	) -> Result<Vec<EstimateGasReturnParams>, anyhow::Error> {
		use tokio_stream::StreamExt;
		let batch_size = 50;
		let mut gas_estimates = vec![];
		for chunk in messages.chunks(batch_size) {
			let processes: FuturesOrdered<
				tokio::task::JoinHandle<Result<EstimateGasReturnParams, Error>>,
			> = chunk
				.into_iter()
				.map(|msg| {
					let call = vec![msg].encode();
					let extrinsic = Extrinsic::new("Ismp", "handle_unsigned", call);
					let client = self.client.clone();
					tokio::spawn(async move {
						let result = system_dry_run_unsigned(&client, extrinsic).await?;
						match result {
							DryRunResult::Success => Ok::<_, Error>(EstimateGasReturnParams {
								execution_cost: Default::default(),
								successful_execution: true,
							}),
							_ => Ok(EstimateGasReturnParams {
								execution_cost: Default::default(),
								successful_execution: false,
							}),
						}
					})
				})
				.collect::<FuturesOrdered<_>>();

			let estimates = processes
				.collect::<Result<Vec<_>, _>>()
				.await?
				.into_iter()
				.collect::<Result<Vec<_>, _>>()?;

			gas_estimates.extend(estimates);
		}

		Ok(gas_estimates)
	}

	async fn query_request_fee_metadata(&self, _hash: H256) -> Result<U256, anyhow::Error> {
		let key = self.req_commitments_key(_hash);
		let child_storage_key = ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
		let storage_key = StorageKey(key);
		let params = rpc_params![child_storage_key, storage_key, Option::<C::Hash>::None];

		let response: Option<StorageData> =
			self.client.rpc().request("childstate_getStorage", params).await?;
		let data = response.ok_or_else(|| anyhow!("Request fee metadata query returned None"))?;
		let leaf_meta = RequestMetadata::decode(&mut &*data.0)?;
		Ok(leaf_meta.meta.fee.into())
	}

	async fn query_request_receipt(&self, _hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		let key = self.req_receipts_key(_hash);
		let child_storage_key = ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
		let storage_key = StorageKey(key);
		let params = rpc_params![child_storage_key, storage_key, Option::<C::Hash>::None];

		let response: Option<StorageData> =
			self.client.rpc().request("childstate_getStorage", params).await?;
		if let Some(data) = response {
			let relayer = Vec::<u8>::decode(&mut &*data.0)?;
			Ok(relayer)
		} else {
			Ok(H160::zero().0.to_vec())
		}
	}

	async fn query_response_receipt(&self, _hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		let key = self.res_receipt_key(_hash);
		let child_storage_key = ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
		let storage_key = StorageKey(key);
		let params = rpc_params![child_storage_key, storage_key, Option::<C::Hash>::None];

		let response: Option<StorageData> =
			self.client.rpc().request("childstate_getStorage", params).await?;
		if let Some(data) = response {
			let relayer = pallet_ismp::ResponseReceipt::decode(&mut &*data.0)?.relayer;
			Ok(relayer)
		} else {
			Ok(H160::zero().0.to_vec())
		}
	}

	async fn query_response_fee_metadata(&self, _hash: H256) -> Result<U256, anyhow::Error> {
		let key = self.res_commitments_key(_hash);
		let child_storage_key = ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
		let storage_key = StorageKey(key);
		let params = rpc_params![child_storage_key, storage_key, Option::<C::Hash>::None];

		let response: Option<StorageData> =
			self.client.rpc().request("childstate_getStorage", params).await?;
		let data = response.ok_or_else(|| anyhow!("Response fee metadata query returned None"))?;
		let leaf_meta = RequestMetadata::decode(&mut &*data.0)?;
		Ok(leaf_meta.meta.fee.into())
	}

	async fn state_commitment_vetoed_notification(
		&self,
		from: u64,
		update_height: StateMachineHeight,
	) -> BoxStream<StateCommitmentVetoed> {
		let client = self.clone();
		let (tx, recv) = tokio::sync::mpsc::channel(32);
		tokio::task::spawn(async move {
			let mut latest_height = from;
			let state_machine = client.state_machine;
			loop {
				// Kill task when receiver is dropped
				if tx.is_closed() {
					return
				}
				tokio::time::sleep(Duration::from_secs(10)).await;
				let header = match client.client.rpc().header(None).await {
					Ok(Some(header)) => header,
					_ => {
						if let Err(err) = tx
							.send(Err(anyhow!(
								"Error encountered while fething finalized head"
							).into()))
							.await
						{
							log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
							return
						}
						continue;
					},
				};

				if header.number().into() <= latest_height {
					continue;
				}

				let event = StateMachineUpdated {
					state_machine_id: client.state_machine_id(),
					latest_height: header.number().into(),
				};

				let events = match client.query_ismp_events(latest_height, event).await {
					Ok(e) => e,
					Err(err) => {
						if let Err(err) = tx
							.send(Err(anyhow!(
								"Error encountered while querying ismp events {err:?}"
							).into()))
							.await
						{
							log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
							return
						}
						latest_height = header.number().into();
						continue;
					},
				};

				let event = events
					.into_iter()
					.find_map(|event| match event {
						Event::StateCommitmentVetoed(e)
							if e.height == update_height =>
							Some(e),
						_ => None,
					});

				match event {
					Some(event) => {
						if let Err(err) = tx.send(Ok(event.clone())).await {
							log::trace!(target: "tesseract", "Failed to send state commitment veto event over channel on {state_machine:?} - {:?} \n {err:?}", update_height.id.state_id);
							return
						};
					},
					None => {},
				};

				latest_height = header.number().into();
			}
		}.boxed());

		Box::pin(tokio_stream::wrappers::ReceiverStream::new(recv))
	}

	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, anyhow::Error> {
		use futures::StreamExt;
		let client = self.clone();
		let mut mutex = self.state_machine_update_sender.lock().await;
		let is_empty = mutex.is_none();
		let (tx, recv) = if is_empty {
			let (tx_og, recv) = tokio::sync::broadcast::channel(512);
			*mutex = Some(tx_og.clone());
			(tx_og, recv)
		} else {
			let tx = mutex.as_ref().expect("Not empty").clone();
			let recv = tx.subscribe();
			(tx, recv)
		};
		let latest_height = client.query_finalized_height().await?;
		let challenge_period = self.query_challenge_period(counterparty_state_id).await?;

		if is_empty {
			tokio::task::spawn(async move {
				let mut latest_height = latest_height;
				let state_machine = client.state_machine;
				loop {
					tokio::time::sleep(Duration::from_secs(10)).await;
					let header = match client.client.rpc().finalized_head().await {
						Ok(hash) => match client.client.rpc().header(Some(hash)).await {
							Ok(Some(header)) => header,
							_ => {
								if let Err(err) = tx
									.send(Err(anyhow!(
										"Error encountered while fetching finalized head"
									).into()))
								{
									log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
									return
								}
								continue;
							},
						},
						Err(err) => {
							if let Err(err) = tx
								.send(Err(anyhow!(
									"Error encountered while fetching finalized head: {err:?}"
								).into()))
							{
								log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
								return
							}
							continue;
						},
					};

					if header.number().into() <= latest_height {
						continue;
					}

					let event = StateMachineUpdated {
						state_machine_id: client.state_machine_id(),
						latest_height: header.number().into(),
					};

					let events = match client.query_ismp_events(latest_height, event).await {
						Ok(e) => e,
						Err(err) => {
							if let Err(err) = tx
								.send(Err(anyhow!(
									"Error encountered while querying ismp events {err:?}"
								).into()))
							{
								log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
								return
							}
							latest_height = header.number().into();
							continue;
						},
					};

					let event = events
						.into_iter()
						.filter_map(|event| match event {
							Event::StateMachineUpdated(e)
								if e.state_machine_id == counterparty_state_id =>
								Some(e),
							_ => None,
						})
						.max_by(|x, y| x.latest_height.cmp(&y.latest_height));

					match event {
						Some(event) => {
							// We wait for the challenge period and see if the update will be vetoed before yielding
							let commitment_height = StateMachineHeight { id: counterparty_state_id, height: event.latest_height };
							let state_machine_update_time = match client.query_state_machine_update_time(commitment_height).await {
								Ok(val) => val,
								Err(err) => {
									if let Err(err) = tx
										.send(Err(anyhow!(
											"Error encountered while querying state_machine_update_time {err:?}"
										).into()))
									{
										log::error!(target: "tesseract", "Failed to send message over channel on {state_machine:?} \n {err:?}");
										return
									}
									latest_height = header.number().into();
									continue;
								}
							};

							let mut state_commitment_vetoed_stream = client.state_commitment_vetoed_notification(latest_height, commitment_height).await;

							let provider = Arc::new(client.clone());
							tokio::select! {
								_res = wait_for_challenge_period(provider, state_machine_update_time, challenge_period) => {
									match _res {
										Ok(_) => {
											if let Err(err) = tx.send(Ok(event.clone())) {
												log::trace!(target: "tesseract", "Failed to send state machine update over channel on {state_machine:?} - {:?} \n {err:?}", counterparty_state_id.state_id);
												return
											};
										}
										Err(err) => {
											log::error!(target: "tesseract", "Error waiting for challenge period in {state_machine:?} - {:?} update stream \n {err:?}", counterparty_state_id.state_id);
										}
									}
								}
								_res = state_commitment_vetoed_stream.next() => {
									match _res {
										Some(Ok(_)) => {
											log::error!(target: "tesseract", "State Commitment for {event:?} was vetoed on {state_machine}");
										}
										_ => {
											log::error!(target: "tesseract", "Error in state machine vetoed stream {state_machine:?} - {:?}", counterparty_state_id.state_id);
										}
									}
								}
							};
						},
						None => {},
					};

					latest_height = header.number().into();
				}
			}.boxed());
		}

		let stream = tokio_stream::wrappers::BroadcastStream::new(recv).filter_map(|res| async {
			match res {
				Ok(res) => Some(res),
				Err(err) => Some(Err(anyhow!("{err:?}").into())),
			}
		});

		Ok(Box::pin(stream))
	}

	async fn submit(&self, messages: Vec<Message>) -> Result<Vec<TxReceipt>, anyhow::Error> {
		let mut futs = vec![];
		for msg in messages {
			let is_consensus_message = matches!(&msg, Message::Consensus(_));
			let call = vec![msg].encode();
			let extrinsic = Extrinsic::new("Ismp", "handle_unsigned", call);
			// We don't compress consensus messages
			if is_consensus_message {
				futs.push(send_unsigned_extrinsic(&self.client, extrinsic, false));
				continue;
			}
			let encoded_call = extrinsic.encode_call_data(&self.client.metadata())?;
			let uncompressed_len = encoded_call.len();
			let max_compressed_size = zstd_safe::compress_bound(uncompressed_len);
			let mut buffer = vec![0u8; max_compressed_size];
			let compressed_call_len = zstd_safe::compress(&mut buffer[..], &encoded_call, 3)
				.map_err(|_| anyhow!("Call compression failed"))?;
			// If compression saving is less than 15% submit the uncompressed call
			if (uncompressed_len.saturating_sub(compressed_call_len) * 100 / uncompressed_len) <
				20usize
			{
				log::trace!(target: "tesseract", "Submitting uncompressed call: compressed:{}kb, uncompressed:{}kb", compressed_call_len / 1000,  uncompressed_len / 1000);
				futs.push(send_unsigned_extrinsic(&self.client, extrinsic, false))
			} else {
				let compressed_call = buffer[0..compressed_call_len].to_vec();
				let call = (compressed_call, uncompressed_len as u32).encode();
				let extrinsic = Extrinsic::new("CallDecompressor", "decompress_call", call);
				log::trace!(target: "tesseract", "Submitting compressed call: compressed:{}kb, uncompressed:{}kb", compressed_call_len / 1000,  uncompressed_len / 1000);
				futs.push(send_unsigned_extrinsic(&self.client, extrinsic, false))
			}
		}
		futures::future::join_all(futs)
			.await
			.into_iter()
			.collect::<Result<Vec<_>, _>>()?;
		Ok(Default::default())
	}

	async fn query_challenge_period(&self, id: StateMachineId) -> Result<Duration, anyhow::Error> {
		let params = rpc_params![id];
		let response: u64 = self.client.rpc().request("ismp_queryChallengePeriod", params).await?;

		Ok(Duration::from_secs(response))
	}

	async fn query_timestamp(&self) -> Result<Duration, anyhow::Error> {
		let timestamp_key =
			hex!("f0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb").to_vec();
		let response = self
			.client
			.rpc()
			.storage(&timestamp_key, None)
			.await?
			.ok_or_else(|| anyhow!("Failed to fetch timestamp"))?;
		let timestamp: u64 = codec::Decode::decode(&mut response.0.as_slice())?;

		Ok(Duration::from_millis(timestamp))
	}

	fn request_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		vec![self.req_commitments_key(commitment)]
	}

	fn request_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		vec![self.req_receipts_key(commitment)]
	}

	fn response_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		vec![self.res_commitments_key(commitment)]
	}

	fn response_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		vec![self.res_receipt_key(commitment)]
	}

	fn address(&self) -> Vec<u8> {
		self.address.clone()
	}

	fn sign(&self, msg: &[u8]) -> tesseract_primitives::Signature {
		let signature = self.signer.sign(msg).0.to_vec();
		Signature::Sr25519 { public_key: self.address.clone(), signature }
	}

	async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		self.set_latest_finalized_height(counterparty).await
	}

	async fn set_initial_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), Error> {
		self.create_consensus_state(message).await?;
		Ok(())
	}

	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, Error> {
		let key = pallet_ismp::child_trie::state_commitment_storage_key(height);
		let child_storage_key = ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
		let storage_key = StorageKey(key);
		let params = rpc_params![child_storage_key, storage_key, Option::<C::Hash>::None];

		let response: Option<StorageData> =
			self.client.rpc().request("childstate_getStorage", params).await?;
		let data =
			response.ok_or_else(|| anyhow!("State commitment not present for state machine"))?;
		let commitment = Decode::decode(&mut &*data.0)?;

		Ok(commitment)
	}

	async fn veto_state_commitment(&self, height: StateMachineHeight) -> Result<(), Error> {
		let signer = InMemorySigner {
			account_id: MultiSigner::Sr25519(self.signer.public()).into_account().into(),
			signer: self.signer.clone(),
		};

		let call = height.encode();
		let call = Extrinsic::new("Fishermen", "veto_state_commitment", call);
		send_extrinsic(&self.client, signer, call).await?;
		Ok(())
	}

	async fn query_host_params(
		&self,
		state_machine: StateMachine,
	) -> Result<HostParam<u128>, anyhow::Error> {
		let key = host_params_storage_key(state_machine);
		let raw_params = self
			.client
			.storage()
			.at_latest()
			.await?
			.fetch_raw(&key)
			.await?
			.ok_or_else(|| anyhow!("Missing host params for {state_machine:?}"))?;

		let params = Decode::decode(&mut &*raw_params)?;
		Ok(params)
	}

	fn max_concurrent_queries(&self) -> usize {
		self.max_concurent_queries.unwrap_or(10) as usize
	}
}

// The storage key needed to access events.
pub fn system_events_key() -> StorageKey {
	let mut storage_key = sp_core::twox_128(b"System").to_vec();
	storage_key.extend(sp_core::twox_128(b"Events").to_vec());
	StorageKey(storage_key)
}
