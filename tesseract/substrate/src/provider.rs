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
use futures::stream::{self, FuturesOrdered};
use hex_literal::hex;
use ismp::{
	consensus::{
		ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId,
	},
	events::Event,
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
use sp_core::{
	storage::{ChildInfo, StorageData, StorageKey},
	Pair, H160, H256, U256,
};
use substrate_state_machine::{StateMachineProof, SubstrateStateProof};
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
	},
	ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
	rpc::types::DryRunResult,
	rpc_params,
	tx::TxPayload,
};
use tokio::time;

use tesseract_primitives::{
	BoxStream, EstimateGasReturnParams, IsmpProvider, Query, StateMachineUpdated,
	StateProofQueryType, TxReceipt,
};

use crate::{
	calls::RequestMetadata,
	extrinsic::{
		send_extrinsic, send_unsigned_extrinsic, system_dry_run_unsigned, Extrinsic, InMemorySigner,
	},
	runtime::{self},
	SubstrateClient,
};

#[async_trait::async_trait]
impl<C> IsmpProvider for SubstrateClient<C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::AccountId: From<sp_core::crypto::AccountId32> + Into<C::Address> + Clone + Send + Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
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
		let block = self.client.blocks().at_latest().await?;
		let key = runtime::api::storage().ismp().state_machine_update_time(&height.into());
		let value = self.client.storage().at(block.hash()).fetch(&key).await?.ok_or_else(|| {
			anyhow!("State machine update for {:?} not found at block {:?}", height, block.hash())
		})?;

		Ok(Duration::from_secs(value))
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
	) -> Result<Vec<u8>, anyhow::Error> {
		if keys.is_empty() {
			Err(anyhow!("No queries provided"))?
		}
		match keys[0].dest_chain {
			// Use mmr proofs for queries going to EVM chains
			StateMachine::Ethereum(_) | StateMachine::Bsc | StateMachine::Polygon => {
				let keys =
					ProofKeys::Requests(keys.into_iter().map(|key| key.commitment).collect());
				let params = rpc_params![at, keys];
				let response: pallet_ismp_rpc::Proof =
					self.client.rpc().request("ismp_queryMmrProof", params).await?;
				Ok(response.proof)
			},
			// Use child trie proofs for queries going to substrate chains
			StateMachine::Polkadot(_) |
			StateMachine::Kusama(_) |
			StateMachine::Grandpa(_) |
			StateMachine::Beefy(_) => {
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
		}
	}

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
	) -> Result<Vec<u8>, anyhow::Error> {
		if keys.is_empty() {
			Err(anyhow!("No queries provided"))?
		}

		match keys[0].dest_chain {
			// Use mmr proofs for queries going to EVM chains
			StateMachine::Ethereum(_) | StateMachine::Bsc | StateMachine::Polygon => {
				let keys =
					ProofKeys::Responses(keys.into_iter().map(|key| key.commitment).collect());
				let params = rpc_params![at, keys];
				let response: pallet_ismp_rpc::Proof =
					self.client.rpc().request("ismp_queryMmrProof", params).await?;
				Ok(response.proof)
			},
			// Use child trie proofs for queries going to substrate chains
			StateMachine::Polkadot(_) |
			StateMachine::Kusama(_) |
			StateMachine::Grandpa(_) |
			StateMachine::Beefy(_) => {
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

		let params = rpc_params![
			BlockNumberOrHash::<H256>::Number(previous_height.saturating_add(1) as u32),
			BlockNumberOrHash::<H256>::Number(event.latest_height as u32)
		];
		let response: HashMap<String, Vec<Event>> =
			self.client.rpc().request("ismp_queryEvents", params).await?;
		let events = response.values().into_iter().cloned().flatten().collect();
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

	async fn query_request_receipt(&self, _hash: H256) -> Result<H160, anyhow::Error> {
		let key = self.req_receipts_key(_hash);
		let child_storage_key = ChildInfo::new_default(CHILD_TRIE_PREFIX).prefixed_storage_key();
		let storage_key = StorageKey(key);
		let params = rpc_params![child_storage_key, storage_key, Option::<C::Hash>::None];

		let response: Option<StorageData> =
			self.client.rpc().request("childstate_getStorage", params).await?;
		let data = response.ok_or_else(|| anyhow!("Request fee metadata query returned None"))?;
		let relayer = Vec::<u8>::decode(&mut &*data.0)?;
		Ok(H160::from_slice(&relayer[..20]))
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

	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, anyhow::Error> {
		use futures::StreamExt;
		let interval = time::interval(Duration::from_secs(10));
		let self_clone = self.clone();
		let stream = stream::unfold(
			(self_clone.initial_height, interval, self_clone),
			move |(latest_height, mut interval, client)| async move {
				interval.tick().await;
				let header = match client.client.rpc().finalized_head().await {
					Ok(hash) => match client.client.rpc().header(Some(hash)).await {
						Ok(Some(header)) => header,
						_ =>
							return Some((
								Err(anyhow!("Error encountered while fething finalized head")),
								(latest_height, interval, client),
							)),
					},
					Err(err) =>
						return Some((
							Err(anyhow!(
								"Error encountered while fetching finalized head: {err:?}"
							)),
							(latest_height, interval, client),
						)),
				};

				if header.number().into() <= latest_height {
					return Some((Ok(None), (latest_height, interval, client)));
				}

				let event = StateMachineUpdated {
					state_machine_id: client.state_machine_id(),
					latest_height: header.number().into(),
				};

				let events = match client.query_ismp_events(latest_height, event).await {
					Ok(e) => e,
					Err(err) =>
						return Some((
							Err(anyhow!("Error encountered while querying ismp events {err:?}")),
							(latest_height, interval, client),
						)),
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

				let value = match event {
					Some(event) =>
						Some((Ok(Some(event)), (header.number().into(), interval, client))),
					None => Some((Ok(None), (header.number().into(), interval, client))),
				};

				return value;
			},
		)
		.filter_map(|res| async move {
			match res {
				Ok(Some(update)) => Some(Ok(update)),
				Ok(None) => None,
				Err(err) => Some(Err(err)),
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
				continue
			}
			let encoded_call = extrinsic.encode_call_data(&self.client.metadata())?;
			let uncompressed_len = encoded_call.len();
			let max_compressed_size = zstd_safe::compress_bound(uncompressed_len);
			let mut buffer = vec![0u8; max_compressed_size];
			let compressed_call_len = zstd_safe::compress(&mut buffer[..], &encoded_call, 3)
				.map_err(|_| anyhow!("Call compression failed"))?;
			// If compression saving is less than 15% submit the uncompressed call
			if (uncompressed_len.saturating_sub(compressed_call_len) * 100 / uncompressed_len) <
				15usize
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

	async fn query_challenge_period(
		&self,
		id: ConsensusStateId,
	) -> Result<Duration, anyhow::Error> {
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
		let addr = runtime::api::storage().ismp().state_commitments(&height.into());
		let commitment = self
			.client
			.storage()
			.at_latest()
			.await?
			.fetch(&addr)
			.await?
			.ok_or_else(|| anyhow!("State commitment not present for state machine"))?;

		let commitment = StateCommitment {
			timestamp: commitment.timestamp,
			overlay_root: commitment.overlay_root,
			state_root: commitment.state_root,
		};
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
		let address = runtime::api::storage().host_executive().host_params(&state_machine.into());
		let params = self
			.client
			.storage()
			.at_latest()
			.await?
			.fetch(&address)
			.await?
			.ok_or_else(|| anyhow!("Missing host params for {state_machine:?}"))?;

		Ok(params.into())
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
