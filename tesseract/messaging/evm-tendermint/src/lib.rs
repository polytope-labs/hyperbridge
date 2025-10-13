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

//! Tendermint EVM Client for ISMP

use anyhow::Error;
use codec::Encode;
use evm_state_machine::{
	presets::{REQUEST_COMMITMENTS_SLOT, RESPONSE_COMMITMENTS_SLOT},
	types::EvmKVProof,
};
use ismp::{
	consensus::{ConsensusStateId, StateMachineHeight, StateMachineId},
	events::{Event, StateCommitmentVetoed},
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use primitive_types::U256;
use sp_core::{H160, H256};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tendermint_ics23_primitives::proof_ops_to_commitment_proof_bytes;
use tendermint_primitives::keys::EvmStoreKeys;
use tendermint_prover::CometBFTClient;
use tesseract_evm::{EvmClient, EvmConfig};
use tesseract_primitives::{
	BoxStream, ByzantineHandler, EstimateGasReturnParams, IsmpProvider, Query, Signature,
	StateMachineUpdated, StateProofQueryType, TxResult,
};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TendermintEvmClientConfig {
	/// EVM config
	#[serde(flatten)]
	pub evm_config: EvmConfig,
	/// Tendermint Json Rpc URL
	pub rpc_url: String,
}

#[derive(Clone)]
pub struct TendermintEvmClient<T: EvmStoreKeys> {
	pub inner: EvmClient,
	prover: std::sync::Arc<CometBFTClient>,
	_phantom: std::marker::PhantomData<T>,
}

impl<T: EvmStoreKeys> TendermintEvmClient<T> {
	pub async fn new(inner: EvmClient, cometbft_rpc_url: String) -> anyhow::Result<Self> {
		let prover = std::sync::Arc::new(CometBFTClient::new(&cometbft_rpc_url).await?);
		Ok(Self { inner, prover, _phantom: std::marker::PhantomData })
	}
}

#[async_trait::async_trait]
impl<T: EvmStoreKeys> IsmpProvider for TendermintEvmClient<T> {
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		id: ConsensusStateId,
	) -> Result<Vec<u8>, Error> {
		self.inner.query_consensus_state(at, id).await
	}

	async fn query_latest_height(&self, id: StateMachineId) -> Result<u32, Error> {
		self.inner.query_latest_height(id).await
	}

	async fn query_finalized_height(&self) -> Result<u64, Error> {
		self.inner.query_finalized_height().await
	}

	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<ismp::consensus::StateCommitment, Error> {
		self.inner.query_state_machine_commitment(height).await
	}

	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, Error> {
		self.inner.query_state_machine_update_time(height).await
	}

	async fn query_challenge_period(&self, id: StateMachineId) -> Result<Duration, Error> {
		self.inner.query_challenge_period(id).await
	}

	async fn query_timestamp(&self) -> Result<Duration, Error> {
		self.inner.query_timestamp().await
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let contract_addr: [u8; 20] = self.inner.config.ismp_host.0;
		let storage_keys: Vec<Vec<u8>> = keys
			.into_iter()
			.map(|q| {
				let slot_hash = tesseract_evm::derive_map_key(
					q.commitment.0.to_vec(),
					REQUEST_COMMITMENTS_SLOT,
				);
				T::storage_key(&contract_addr, slot_hash.0)
			})
			.collect();

		let responses = self
			.prover
			.abci_query_keys(&T::store_key(), storage_keys, at - 1)
			.await
			.map_err(|e| anyhow::anyhow!("abci_query_keys error: {e:?}"))?;

		let proofs: Vec<EvmKVProof> = responses
			.into_iter()
			.map(|resp| {
				let proof = proof_ops_to_commitment_proof_bytes(resp.proof).unwrap_or_default();
				EvmKVProof { value: resp.value, proof }
			})
			.collect();

		Ok(proofs.encode())
	}

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		_counterparty: StateMachine,
	) -> Result<Vec<u8>, Error> {
		let contract_addr: [u8; 20] = self.inner.config.ismp_host.0;
		let storage_keys: Vec<Vec<u8>> = keys
			.into_iter()
			.map(|q| {
				let slot_hash = tesseract_evm::derive_map_key(
					q.commitment.0.to_vec(),
					RESPONSE_COMMITMENTS_SLOT,
				);
				T::storage_key(&contract_addr, slot_hash.0)
			})
			.collect();

		let responses = self
			.prover
			.abci_query_keys(&T::store_key(), storage_keys, at - 1)
			.await
			.map_err(|e| anyhow::anyhow!("abci_query_keys error: {e:?}"))?;

		let proofs: Vec<EvmKVProof> = responses
			.into_iter()
			.map(|resp| {
				let proof = proof_ops_to_commitment_proof_bytes(resp.proof).unwrap_or_default();
				EvmKVProof { value: resp.value, proof }
			})
			.collect();

		Ok(proofs.encode())
	}

	async fn query_state_proof(
		&self,
		at: u64,
		keys: StateProofQueryType,
	) -> Result<Vec<u8>, Error> {
		let mut proofs: Vec<EvmKVProof> = Vec::new();
		match keys {
			StateProofQueryType::Ismp(keys) => {
				let contract_addr: [u8; 20] = self.inner.config.ismp_host.0;
				// Validate lengths first to avoid Result in iterator
				if keys.iter().any(|key| key.len() != 32) {
					return Err(anyhow::anyhow!("All ISMP keys must have a length of 32 bytes",));
				}
				let storage_keys: Vec<Vec<u8>> = keys
					.into_iter()
					.map(|key| {
						let slot = H256::from_slice(&key);
						T::storage_key(&contract_addr, slot.0)
					})
					.collect();

				let responses = self
					.prover
					.abci_query_keys(&T::store_key(), storage_keys, at - 1)
					.await
					.map_err(|e| anyhow::anyhow!("abci_query_keys error: {e:?}"))?;

				proofs = responses
					.into_iter()
					.map(|resp| {
						let proof =
							proof_ops_to_commitment_proof_bytes(resp.proof).unwrap_or_default();
						EvmKVProof { value: resp.value, proof }
					})
					.collect();
			},
			StateProofQueryType::Arbitrary(keys) => {
				let mut grouped: BTreeMap<[u8; 20], Vec<[u8; 32]>> = BTreeMap::new();
				for key in keys.into_iter() {
					if key.len() != 52 {
						anyhow::bail!(
							"All arbitrary keys must have a length of 52 bytes, found {}",
							key.len()
						);
					}
					let contract_address = H160::from_slice(&key[..20]);
					let slot = H256::from_slice(&key[20..]);
					grouped.entry(contract_address.0).or_default().push(slot.0);
				}

				for (addr, slots) in grouped.into_iter() {
					let storage_keys: Vec<Vec<u8>> =
						slots.into_iter().map(|slot| T::storage_key(&addr, slot)).collect();

					let responses = self
						.prover
						.abci_query_keys(&T::store_key(), storage_keys, at - 1)
						.await
						.map_err(|e| anyhow::anyhow!("abci_query_keys error: {e:?}"))?;

					proofs.extend(responses.into_iter().map(|resp| {
						let proof =
							proof_ops_to_commitment_proof_bytes(resp.proof).unwrap_or_default();
						EvmKVProof { value: resp.value, proof }
					}));
				}
			},
		}
		Ok(proofs.encode())
	}

	async fn query_ismp_events(
		&self,
		previous_height: u64,
		event: StateMachineUpdated,
	) -> Result<Vec<Event>, Error> {
		let adjusted_event = StateMachineUpdated {
			state_machine_id: event.state_machine_id,
			latest_height: event.latest_height.saturating_sub(1),
		};
		self.inner.query_ismp_events(previous_height, adjusted_event).await
	}

	fn name(&self) -> String {
		self.inner.name()
	}

	fn state_machine_id(&self) -> StateMachineId {
		self.inner.state_machine_id()
	}

	fn block_max_gas(&self) -> u64 {
		self.inner.block_max_gas()
	}

	fn initial_height(&self) -> u64 {
		self.inner.initial_height()
	}

	async fn estimate_gas(&self, msg: Vec<Message>) -> Result<Vec<EstimateGasReturnParams>, Error> {
		self.inner.estimate_gas(msg).await
	}

	async fn query_request_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		self.inner.query_request_fee_metadata(hash).await
	}

	async fn query_request_receipt(&self, hash: H256) -> Result<Vec<u8>, Error> {
		self.inner.query_request_receipt(hash).await
	}

	async fn query_response_receipt(&self, hash: H256) -> Result<Vec<u8>, Error> {
		self.inner.query_response_receipt(hash).await
	}

	async fn query_response_fee_metadata(&self, hash: H256) -> Result<U256, Error> {
		self.inner.query_response_fee_metadata(hash).await
	}

	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, Error> {
		self.inner.state_machine_update_notification(counterparty_state_id).await
	}

	async fn state_commitment_vetoed_notification(
		&self,
		from: u64,
		height: StateMachineHeight,
	) -> BoxStream<StateCommitmentVetoed> {
		self.inner.state_commitment_vetoed_notification(from, height).await
	}

	async fn submit(
		&self,
		messages: Vec<Message>,
		coprocessor: StateMachine,
	) -> Result<TxResult, Error> {
		self.inner.submit(messages, coprocessor).await
	}

	fn request_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.inner.request_commitment_full_key(commitment)
	}

	fn request_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.inner.request_receipt_full_key(commitment)
	}

	fn response_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.inner.response_commitment_full_key(commitment)
	}

	fn response_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.inner.response_receipt_full_key(commitment)
	}

	fn address(&self) -> Vec<u8> {
		self.inner.address()
	}

	fn sign(&self, msg: &[u8]) -> Signature {
		self.inner.sign(msg)
	}

	async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), Error> {
		self.inner.set_latest_finalized_height(counterparty).await
	}

	async fn set_initial_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), Error> {
		self.inner.set_initial_consensus_state(message).await
	}

	async fn veto_state_commitment(&self, height: StateMachineHeight) -> Result<(), Error> {
		self.inner.veto_state_commitment(height).await
	}

	async fn query_host_params(
		&self,
		state_machine: StateMachine,
	) -> Result<pallet_ismp_host_executive::HostParam<u128>, Error> {
		self.inner.query_host_params(state_machine).await
	}

	fn max_concurrent_queries(&self) -> usize {
		self.inner.max_concurrent_queries()
	}

	async fn fee_token_decimals(&self) -> Result<u8, Error> {
		self.inner.fee_token_decimals().await
	}
}

#[async_trait::async_trait]
impl<T: EvmStoreKeys> ByzantineHandler for TendermintEvmClient<T> {
	async fn check_for_byzantine_attack(
		&self,
		coprocessor: StateMachine,
		counterparty: Arc<dyn IsmpProvider>,
		challenge_event: StateMachineUpdated,
	) -> Result<(), Error> {
		self.inner
			.check_for_byzantine_attack(coprocessor, counterparty, challenge_event)
			.await
	}

	async fn state_machine_updates(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<Vec<StateMachineUpdated>>, Error> {
		self.inner.state_machine_updates(counterparty_state_id).await
	}
}
