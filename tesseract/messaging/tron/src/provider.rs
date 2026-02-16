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

//! [`IsmpProvider`] and [`ByzantineHandler`] implementations for [`TronClient`].
//!
//! Every read operation is delegated to the inner [`EvmClient`] which talks to
//! TRON's Ethereum-compatible JSON-RPC endpoint.  Only [`IsmpProvider::submit`]
//! is overridden to route transactions through the TRON native HTTP API
//! (`/wallet/triggersmartcontract` → sign → `/wallet/broadcasttransaction`).

use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use ismp::{
	consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
	events::{Event, StateCommitmentVetoed},
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use pallet_ismp_host_executive::HostParam;
use primitive_types::{H256, U256};
use tesseract_primitives::{
	BoxStream, ByzantineHandler, EstimateGasReturnParams, IsmpProvider, Query, Signature,
	StateMachineUpdated, StateProofQueryType, TxResult,
};

use crate::TronClient;

#[async_trait::async_trait]
impl ByzantineHandler for TronClient {
	async fn check_for_byzantine_attack(
		&self,
		coprocessor: StateMachine,
		counterparty: Arc<dyn IsmpProvider>,
		challenge_event: StateMachineUpdated,
	) -> Result<(), anyhow::Error> {
		self.evm
			.check_for_byzantine_attack(coprocessor, counterparty, challenge_event)
			.await
	}

	async fn state_machine_updates(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<Vec<StateMachineUpdated>>, anyhow::Error> {
		self.evm.state_machine_updates(counterparty_state_id).await
	}
}

#[async_trait::async_trait]
impl IsmpProvider for TronClient {
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		id: ConsensusStateId,
	) -> Result<Vec<u8>, anyhow::Error> {
		self.evm.query_consensus_state(at, id).await
	}

	async fn query_latest_height(&self, id: StateMachineId) -> Result<u32, anyhow::Error> {
		self.evm.query_latest_height(id).await
	}

	async fn query_finalized_height(&self) -> Result<u64, anyhow::Error> {
		self.evm.query_finalized_height().await
	}

	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, anyhow::Error> {
		self.evm.query_state_machine_commitment(height).await
	}

	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, anyhow::Error> {
		self.evm.query_state_machine_update_time(height).await
	}

	async fn query_challenge_period(&self, id: StateMachineId) -> Result<Duration, anyhow::Error> {
		self.evm.query_challenge_period(id).await
	}

	async fn query_timestamp(&self) -> Result<Duration, anyhow::Error> {
		self.evm.query_timestamp().await
	}

	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		counterparty: StateMachine,
	) -> Result<Vec<u8>, anyhow::Error> {
		self.evm.query_requests_proof(at, keys, counterparty).await
	}

	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		counterparty: StateMachine,
	) -> Result<Vec<u8>, anyhow::Error> {
		self.evm.query_responses_proof(at, keys, counterparty).await
	}

	async fn query_state_proof(
		&self,
		at: u64,
		keys: StateProofQueryType,
	) -> Result<Vec<u8>, anyhow::Error> {
		self.evm.query_state_proof(at, keys).await
	}

	async fn query_ismp_events(
		&self,
		previous_height: u64,
		event: StateMachineUpdated,
	) -> Result<Vec<Event>, anyhow::Error> {
		self.evm.query_ismp_events(previous_height, event).await
	}

	fn name(&self) -> String {
		format!("TRON-{}", self.evm.chain_id)
	}

	fn state_machine_id(&self) -> StateMachineId {
		self.evm.state_machine_id()
	}

	fn block_max_gas(&self) -> u64 {
		self.evm.block_max_gas()
	}

	fn initial_height(&self) -> u64 {
		self.evm.initial_height()
	}

	async fn estimate_gas(
		&self,
		msg: Vec<Message>,
	) -> Result<Vec<EstimateGasReturnParams>, anyhow::Error> {
		self.evm.estimate_gas(msg).await
	}

	async fn query_request_fee_metadata(&self, hash: H256) -> Result<U256, anyhow::Error> {
		self.evm.query_request_fee_metadata(hash).await
	}

	async fn query_request_receipt(&self, hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		self.evm.query_request_receipt(hash).await
	}

	async fn query_response_receipt(&self, hash: H256) -> Result<Vec<u8>, anyhow::Error> {
		self.evm.query_response_receipt(hash).await
	}

	async fn query_response_fee_metadata(&self, hash: H256) -> Result<U256, anyhow::Error> {
		self.evm.query_response_fee_metadata(hash).await
	}

	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, anyhow::Error> {
		self.evm.state_machine_update_notification(counterparty_state_id).await
	}

	async fn state_commitment_vetoed_notification(
		&self,
		from: u64,
		height: StateMachineHeight,
	) -> BoxStream<StateCommitmentVetoed> {
		self.evm.state_commitment_vetoed_notification(from, height).await
	}

	//
	// This is the **only** method that diverges from EvmClient.
	//
	// Instead of building an ethers `TypedTransaction` and calling
	// `eth_sendRawTransaction`, we route through the TRON native API:
	//
	//   1. `generate_contract_calls` (reused from tesseract-evm) → produces ethers `FunctionCall`
	//      objects with ABI-encoded calldata
	//   2. Extract the raw calldata bytes (`call.calldata()`)
	//   3. POST `/wallet/triggersmartcontract` with the calldata as `data`
	//   4. Sign the returned unsigned tx (secp256k1 over SHA-256)
	//   5. POST `/wallet/broadcasttransaction`
	//   6. Poll `/wallet/gettransactioninfobyid` for the receipt

	async fn submit(
		&self,
		messages: Vec<Message>,
		_coprocessor: StateMachine,
	) -> Result<TxResult, anyhow::Error> {
		self.queue
			.as_ref()
			.ok_or_else(|| anyhow!("Transaction submission pipeline was not initialized"))?
			.send(messages)
			.await?
	}

	fn request_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.request_commitment_full_key(commitment)
	}

	fn request_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.request_receipt_full_key(commitment)
	}

	fn response_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.response_commitment_full_key(commitment)
	}

	fn response_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>> {
		self.evm.response_receipt_full_key(commitment)
	}

	fn address(&self) -> Vec<u8> {
		self.evm.address()
	}

	fn sign(&self, msg: &[u8]) -> Signature {
		self.evm.sign(msg)
	}

	async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		self.evm.set_latest_finalized_height(counterparty).await
	}

	async fn set_initial_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), anyhow::Error> {
		// Delegate to the inner EvmClient.  This uses ethers to send an
		// Ethereum-format transaction, which may or may not be accepted by
		// the TRON node's JSON-RPC endpoint.
		//
		// In practice the initial consensus state is set during contract
		// deployment (via the TronBox migration script), so this code path
		// is rarely hit at runtime.
		self.evm.set_initial_consensus_state(message).await
	}

	async fn veto_state_commitment(&self, height: StateMachineHeight) -> Result<(), anyhow::Error> {
		self.evm.veto_state_commitment(height).await
	}

	async fn query_host_params(
		&self,
		state_machine: StateMachine,
	) -> Result<HostParam<u128>, anyhow::Error> {
		self.evm.query_host_params(state_machine).await
	}

	fn max_concurrent_queries(&self) -> usize {
		self.evm.max_concurrent_queries()
	}

	async fn fee_token_decimals(&self) -> Result<u8, anyhow::Error> {
		self.evm.fee_token_decimals().await
	}
}
