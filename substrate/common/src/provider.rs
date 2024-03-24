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

use crate::{
	extrinsic::{
		send_extrinsic, send_unsigned_extrinsic, system_dry_run_unsigned, Extrinsic, InMemorySigner,
	},
	runtime::{self, api::runtime_types},
	SubstrateClient,
};
use anyhow::{anyhow, Error};
use codec::{Decode, Encode};

use futures::stream::{self, FuturesUnordered};
use hex_literal::hex;
use ismp::{
	consensus::{
		ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId,
	},
	events::Event,
	host::{Ethereum, StateMachine},
	messaging::CreateConsensusState,
};
use ismp_rpc::{BlockNumberOrHash, MmrProof};
use pallet_ismp::{primitives::SubstrateStateProof, ProofKeys};
use pallet_ismp_relayer::withdrawal::Signature;
use primitives::{
	BoxStream, EstimateGasReturnParams, IsmpHost, IsmpProvider, Query, StateMachineUpdated,
	TxReceipt,
};
use sp_core::{storage::StorageKey, Pair, H256, U256};
use std::{collections::HashMap, time::Duration};
use subxt::{
	config::{
		extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
	},
	ext::sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner},
	rpc::types::DryRunResult,
	rpc_params,
};
use tokio::time;

#[async_trait::async_trait]
impl<T, C> IsmpProvider for SubstrateClient<T, C>
where
	C: subxt::Config + Send + Sync + Clone,
	C::Header: Send + Sync,
	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::AccountId:
		From<sp_core::crypto::AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
	C::Signature: From<MultiSignature> + Send + Sync,
	T: IsmpHost + Send + Sync + 'static,
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
		let keys = ProofKeys::Requests(keys.into_iter().map(|key| key.commitment).collect());
		let params = rpc_params![at, keys];
		let response: ismp_rpc::Proof =
			self.client.rpc().request("ismp_queryMmrProof", params).await?;
		let proof: MmrProof<H256> = Decode::decode(&mut &*response.proof)?;
		Ok(proof.encode())
	}

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
	) -> Result<Vec<u8>, anyhow::Error> {
		let keys = ProofKeys::Responses(keys.into_iter().map(|key| key.commitment).collect());
		let params = rpc_params![at, keys];
		let response: ismp_rpc::Proof =
			self.client.rpc().request("ismp_queryMmrProof", params).await?;
		let proof: MmrProof<H256> = Decode::decode(&mut &*response.proof)?;
		Ok(proof.encode())
	}

	async fn query_state_proof(
		&self,
		at: u64,
		keys: Vec<Vec<u8>>,
	) -> Result<Vec<u8>, anyhow::Error> {
		let params = rpc_params![at, keys];
		let response: ismp_rpc::Proof =
			self.client.rpc().request("ismp_queryStateProof", params).await?;

		let storage_proof: Vec<Vec<u8>> = Decode::decode(&mut &*response.proof)?;
		let proof = SubstrateStateProof { hasher: self.hashing.clone(), storage_proof };
		Ok(proof.encode())
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
			let processes: FuturesUnordered<
				tokio::task::JoinHandle<Result<EstimateGasReturnParams, Error>>,
			> = chunk
				.into_iter()
				.map(|msg| {
					let call = vec![msg].encode();
					let extrinsic = Extrinsic::new("Ismp", "validate_messages", call);
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
				.collect::<FuturesUnordered<_>>();

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
		Ok(Default::default())
	}

	async fn query_response_fee_metadata(&self, _hash: H256) -> Result<U256, anyhow::Error> {
		Ok(Default::default())
	}

	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, anyhow::Error> {
		use futures::StreamExt;
		let interval = time::interval(Duration::from_secs(10));
		let stream = stream::unfold(
			(self.initial_height, interval, self.clone()),
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
					return Some((Ok(None), (latest_height, interval, client)))
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

	async fn submit(
		&self,
		messages: Vec<ismp::messaging::Message>,
	) -> Result<Vec<TxReceipt>, anyhow::Error> {
		let mut futs = vec![];
		for msg in messages {
			let call = vec![msg].encode();
			let extrinsic = Extrinsic::new("Ismp", "handle", call);
			futs.push(send_unsigned_extrinsic(&self.client, extrinsic))
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

	fn sign(&self, msg: &[u8]) -> primitives::Signature {
		let signature = self.signer.sign(msg).0.to_vec();
		Signature::Sr25519 { public_key: self.address.clone(), signature }
	}

	async fn set_latest_finalized_height<P: IsmpProvider + 'static>(
		&mut self,
		counterparty: &P,
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

	async fn freeze_state_machine(&self, id: StateMachineId) -> Result<(), Error> {
		let signer = InMemorySigner {
			account_id: MultiSigner::Sr25519(self.signer.public()).into_account().into(),
			signer: self.signer.clone(),
		};

		let call = id.encode();
		let call = Extrinsic::new("StateMachineManager", "freeze_state_machine", call);
		send_extrinsic(&self.client, signer, call).await?;
		Ok(())
	}

	async fn query_host_manager_address(&self) -> Result<Vec<u8>, anyhow::Error> {
		Ok(pallet_ismp_relayer::MODULE_ID.to_vec())
	}

	fn max_concurrent_queries(&self) -> usize {
		50
	}
}

// The storage key needed to access events.
pub fn system_events_key() -> StorageKey {
	let mut storage_key = sp_core::twox_128(b"System").to_vec();
	storage_key.extend(sp_core::twox_128(b"Events").to_vec());
	StorageKey(storage_key)
}

impl From<runtime_types::ismp::host::StateMachine> for StateMachine {
	fn from(value: runtime_types::ismp::host::StateMachine) -> Self {
		match value {
			runtime_types::ismp::host::StateMachine::Ethereum(
				runtime_types::ismp::host::Ethereum::ExecutionLayer,
			) => StateMachine::Ethereum(Ethereum::ExecutionLayer),
			runtime_types::ismp::host::StateMachine::Ethereum(
				runtime_types::ismp::host::Ethereum::Base,
			) => StateMachine::Ethereum(Ethereum::Base),
			runtime_types::ismp::host::StateMachine::Ethereum(
				runtime_types::ismp::host::Ethereum::Optimism,
			) => StateMachine::Ethereum(Ethereum::Optimism),
			runtime_types::ismp::host::StateMachine::Ethereum(
				runtime_types::ismp::host::Ethereum::Arbitrum,
			) => StateMachine::Ethereum(Ethereum::Arbitrum),
			runtime_types::ismp::host::StateMachine::Kusama(id) => StateMachine::Kusama(id),
			runtime_types::ismp::host::StateMachine::Polkadot(id) => StateMachine::Polkadot(id),
			runtime_types::ismp::host::StateMachine::Polygon => StateMachine::Polygon,
			runtime_types::ismp::host::StateMachine::Bsc => StateMachine::Bsc,
			runtime_types::ismp::host::StateMachine::Beefy(id) => StateMachine::Beefy(id),
			runtime_types::ismp::host::StateMachine::Grandpa(id) => StateMachine::Grandpa(id),
		}
	}
}

impl From<StateMachine> for runtime_types::ismp::host::StateMachine {
	fn from(value: StateMachine) -> Self {
		match value {
			StateMachine::Ethereum(Ethereum::ExecutionLayer) =>
				runtime_types::ismp::host::StateMachine::Ethereum(
					runtime_types::ismp::host::Ethereum::ExecutionLayer,
				),
			StateMachine::Ethereum(Ethereum::Base) =>
				runtime_types::ismp::host::StateMachine::Ethereum(
					runtime_types::ismp::host::Ethereum::Base,
				),
			StateMachine::Ethereum(Ethereum::Optimism) =>
				runtime_types::ismp::host::StateMachine::Ethereum(
					runtime_types::ismp::host::Ethereum::Optimism,
				),
			StateMachine::Ethereum(Ethereum::Arbitrum) =>
				runtime_types::ismp::host::StateMachine::Ethereum(
					runtime_types::ismp::host::Ethereum::Arbitrum,
				),
			StateMachine::Kusama(id) => runtime_types::ismp::host::StateMachine::Kusama(id),
			StateMachine::Polkadot(id) => runtime_types::ismp::host::StateMachine::Polkadot(id),
			StateMachine::Polygon => runtime_types::ismp::host::StateMachine::Polygon,
			StateMachine::Bsc => runtime_types::ismp::host::StateMachine::Bsc,
			StateMachine::Beefy(id) => runtime_types::ismp::host::StateMachine::Beefy(id),
			StateMachine::Grandpa(id) => runtime_types::ismp::host::StateMachine::Grandpa(id),
		}
	}
}

impl From<StateMachineId> for runtime_types::ismp::consensus::StateMachineId {
	fn from(value: StateMachineId) -> Self {
		runtime_types::ismp::consensus::StateMachineId {
			state_id: value.state_id.into(),
			consensus_state_id: value.consensus_state_id,
		}
	}
}

impl From<StateMachineHeight> for runtime_types::ismp::consensus::StateMachineHeight {
	fn from(value: StateMachineHeight) -> Self {
		runtime_types::ismp::consensus::StateMachineHeight {
			id: value.id.into(),
			height: value.height.into(),
		}
	}
}
