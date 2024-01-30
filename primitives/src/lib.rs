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

//! Traits and types required to compose the tesseract relayer
pub mod config;
#[cfg(feature = "testing")]
pub mod mocks;
pub mod queue;

use futures::Stream;
pub use ismp::events::StateMachineUpdated;
use ismp::{
	consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
	events::Event,
	host::StateMachine,
	messaging::{ConsensusMessage, CreateConsensusState, Message},
	router::Post,
};
pub use pallet_relayer_fees::withdrawal::{Signature, WithdrawalProof};
use primitive_types::{H256, U256};
use std::{pin::Pin, sync::Arc, time::Duration};

#[derive(Copy, Clone, Debug, Default)]
pub struct EstimateGasReturnParams {
	pub execution_cost: U256,
	pub successful_execution: bool,
}

/// Provides an interface for accessing new events and ISMP data on the chain which must be
/// relayed to the counterparty chain.

#[derive(Copy, Clone, Debug)]
pub struct Query {
	pub source_chain: StateMachine,
	pub dest_chain: StateMachine,
	pub nonce: u64,
	pub commitment: H256,
}

/// Stream alias
pub type BoxStream<I> = Pin<Box<dyn Stream<Item = Result<I, anyhow::Error>> + Send>>;

#[async_trait::async_trait]
pub trait IsmpProvider: Send + Sync {
	/// Query the latest consensus state of a client
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		id: ConsensusStateId,
	) -> Result<Vec<u8>, anyhow::Error>;

	/// Query the latest height at which some state machine was last updated
	async fn query_latest_height(&self, id: StateMachineId) -> Result<u32, anyhow::Error>;

	/// Query the State machine commitment at the provided height
	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, anyhow::Error>;

	/// Query the timestamp at which the client was last updated
	async fn query_consensus_update_time(
		&self,
		id: ConsensusStateId,
	) -> Result<Duration, anyhow::Error>;

	/// Query the challenge period for client
	async fn query_challenge_period(&self, id: ConsensusStateId)
		-> Result<Duration, anyhow::Error>;

	/// Query the latest timestamp for chain
	async fn query_timestamp(&self) -> Result<Duration, anyhow::Error>;

	/// Query a requests proof
	/// Return the scale encoded proof
	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
	) -> Result<Vec<u8>, anyhow::Error>;

	/// Query a responses proof
	/// Return the scale encoded proof
	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
	) -> Result<Vec<u8>, anyhow::Error>;

	/// Query state proof for some keys, return scaled encoded proof
	async fn query_state_proof(
		&self,
		at: u64,
		keys: Vec<Vec<u8>>,
	) -> Result<Vec<u8>, anyhow::Error>;

	/// Query all ismp events on naive that can be processed for a [`StateMachineUpdated`]
	/// event on the counterparty
	async fn query_ismp_events(
		&self,
		previous_height: u64,
		event: StateMachineUpdated,
	) -> Result<Vec<Event>, anyhow::Error>;

	/// Name of this chain, used in logs.
	fn name(&self) -> String;

	/// State Machine Id for this client which would be it's state machine id
	/// on the counterparty chain
	fn state_machine_id(&self) -> StateMachineId;

	/// Should return a numerical value for the max gas allowed for transactions in a block.
	fn block_max_gas(&self) -> u64;

	/// Should return the initial height at which events should be queried
	fn initial_height(&self) -> u64;

	/// Should return a numerical estimate of the gas to be consumed for a batch of messages.
	async fn estimate_gas(
		&self,
		msg: Vec<Message>,
	) -> Result<Vec<EstimateGasReturnParams>, anyhow::Error>;

	/// Should return fee relayer would be recieving to relay a request mesage giving a hash
	/// (message commiment)
	async fn get_message_request_fee_metadata(&self, hash: H256) -> Result<U256, anyhow::Error>;

	/// Should return fee relayer would be recieving to relay a responce mesage giving a hash
	/// (message commiment)
	async fn query_message_response_fee_metadata(&self, hash: H256) -> Result<U256, anyhow::Error>;

	/// Return a stream that watches for updates to [`counterparty_state_id`], yields when new
	/// [`StateMachineUpdated`] event is observed for [`counterparty_state_id`]
	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, anyhow::Error>;

	/// This should be used to submit new messages [`Vec<Message>`] from a counterparty chain to
	/// this chain.
	///
	/// Should only return Ok if the transaction was successfully inserted into a block.
	async fn submit(&self, messages: Vec<Message>) -> Result<(), anyhow::Error>;

	/// This method should return the key used to be used to query the state proof for the request
	/// commitment
	fn request_commitment_full_key(&self, commitment: H256) -> Vec<u8>;

	/// This method should return the key used to be used to query the state proof for the request
	/// receipt
	fn request_receipt_full_key(&self, commitment: H256) -> Vec<u8>;

	/// This method should return the key used to be used to query the state proof for the response
	/// commitment
	fn response_commitment_full_key(&self, commitment: H256) -> Vec<u8>;

	/// This method should return the key used to be used to query the state proof for the response
	/// receipt
	fn response_receipt_full_key(&self, commitment: H256) -> Vec<u8>;

	/// Relayer's address on this chain
	fn address(&self) -> Vec<u8>;

	/// Sign a prehashed message using the Relayer's private key
	fn sign(&self, msg: &[u8]) -> Signature;

	/// Initialize a nonce for the chain
	async fn initialize_nonce(&self) -> Result<NonceProvider, anyhow::Error>;
	/// Set the nonce provider for the chain
	fn set_nonce_provider(&mut self, nonce_provider: NonceProvider);

	/// Set the initial consensus state for a given consensus state id on this chain
	async fn set_initial_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), anyhow::Error>;

	/// Temporary: Submit a message to freeze the State Machine
	async fn freeze_state_machine(&self, id: StateMachineId) -> Result<(), anyhow::Error>;
}

/// Provides an interface for handling byzantine behaviour. Implementations of this should watch for
/// eclipse attacks, as well as invalid state transitions.
#[async_trait::async_trait]
pub trait ByzantineHandler {
	/// Returns the [`ConsensusMessage`] that caused the emission of  [`StateMachineUpdated`]
	/// event
	async fn query_consensus_message(
		&self,
		challenge_event: StateMachineUpdated,
	) -> Result<ConsensusMessage, anyhow::Error>;

	/// Check the client message for byzantine behaviour and submit it to the chain if any.
	async fn check_for_byzantine_attack<C: IsmpHost + IsmpProvider>(
		&self,
		counterparty: &C,
		consensus_message: ConsensusMessage,
	) -> Result<(), anyhow::Error>;
}

/// Provides an interface for the chain to the relayer core for submitting Ismp messages as well as
#[async_trait::async_trait]
pub trait IsmpHost: ByzantineHandler + Clone + Send + Sync {
	/// Return a stream that yields [`ConsensusMessage`] when a new consensus update
	/// can be sent to the counterparty
	async fn consensus_notification<C>(
		&self,
		counterparty: C,
	) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
	where
		C: IsmpHost + IsmpProvider + Clone + 'static;

	/// Get a trusted consensus state for this host
	async fn get_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error>;
}

#[async_trait::async_trait]
pub trait HyperbridgeClaim {
	async fn accumulate_fees(&self, proof: WithdrawalProof) -> anyhow::Result<()>;
	async fn withdraw_funds<C: IsmpProvider>(
		&self,
		counterparty: &C,
		chain: StateMachine,
		gas_limit: u64,
	) -> anyhow::Result<WithdrawFundsResult>;
}

pub struct WithdrawFundsResult {
	/// Post request emitted by the withdraw request
	pub post: Post,
	/// Block height at which the post request was emitted
	pub block: u64,
}

#[derive(Clone, Debug)]
pub struct NonceProvider {
	nonce: Arc<tokio::sync::Mutex<u64>>,
}

impl NonceProvider {
	pub fn new(nonce: u64) -> Self {
		Self { nonce: Arc::new(tokio::sync::Mutex::new(nonce)) }
	}

	pub async fn get_nonce(&self) -> u64 {
		let mut guard = self.nonce.lock().await;
		let nonce = *guard;
		*guard = nonce + 1;
		nonce
	}

	pub async fn read_nonce(&self) -> u64 {
		let guard = self.nonce.lock().await;
		let nonce = *guard;
		nonce
	}
}
pub async fn wait_for_challenge_period<C: IsmpProvider>(
	client: &C,
	last_consensus_update: Duration,
	challenge_period: Duration,
) -> anyhow::Result<()> {
	tokio::time::sleep(challenge_period + Duration::from_secs(60)).await;
	loop {
		let current_timestamp = client.query_timestamp().await?;
		if current_timestamp.saturating_sub(last_consensus_update) <= challenge_period {
			tokio::time::sleep(Duration::from_secs(60)).await;
		} else {
			break
		}
	}
	Ok(())
}
