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

use anyhow::anyhow;
use futures::{Stream, StreamExt};
pub use ismp::events::StateMachineUpdated;
use ismp::{
	consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
	events::{Event, StateCommitmentVetoed},
	host::StateMachine,
	messaging::{CreateConsensusState, Keccak256, Message},
	router::PostRequest,
};
use pallet_ismp_host_executive::HostParam;
use pallet_ismp_relayer::withdrawal::Key;
pub use pallet_ismp_relayer::withdrawal::{Signature, WithdrawalProof};
use pallet_state_coprocessor::impls::GetRequestsWithProof;
use parity_scale_codec::{Decode, Encode};
use primitive_types::{H256, U256};
use sp_core::keccak_256;
use std::{
	fmt::{Debug, Display, Formatter},
	ops::{Add, Mul},
	pin::Pin,
	sync::Arc,
	time::Duration,
};

use tracing::instrument;

/// Ideal Currency unit denominated in 18 decimals
#[derive(Copy, Clone, Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct Cost(pub U256);

impl Mul<U256> for Cost {
	type Output = Cost;

	fn mul(self, rhs: U256) -> Self::Output {
		Cost(self.0 * rhs)
	}
}

impl Add<Cost> for Cost {
	type Output = Self;

	fn add(self, rhs: Cost) -> Self::Output {
		Cost(self.0 + rhs.0)
	}
}

impl Add<U256> for Cost {
	type Output = Self;

	fn add(self, rhs: U256) -> Self::Output {
		Cost(self.0 + rhs)
	}
}

impl Cost {
	pub fn display(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let val_as_str = self.0.to_string();
		let mut characters = val_as_str.chars().collect::<Vec<_>>();
		// pad with zeros if length is less than 18
		if characters.len() <= 18 {
			let rem = 18 - characters.len();
			(0..=rem).into_iter().for_each(|_| characters.insert(0, '0'));
		}
		// Insert decimal point
		let pointer = characters.len().saturating_sub(18);
		characters.insert(pointer, '.');
		let value = characters.into_iter().collect::<String>();
		f.write_str(&value)
	}
}

impl Debug for Cost {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		self.display(f)
	}
}

impl Display for Cost {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		self.display(f)
	}
}

impl From<U256> for Cost {
	fn from(value: U256) -> Self {
		Cost(value)
	}
}

#[derive(Copy, Clone, Debug, Default)]
pub struct EstimateGasReturnParams {
	pub execution_cost: Cost,
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

/// A type tha should be returned when messages are submitted successfully
#[derive(Debug, Clone, Copy)]
pub enum TxReceipt {
	/// Request variant
	Request { query: Query, height: u64 },
	/// Response variant
	Response { query: Query, request_commitment: H256, height: u64 },
}

impl TxReceipt {
	pub fn height(&self) -> u64 {
		match self {
			TxReceipt::Request { height, .. } => *height,
			TxReceipt::Response { height, .. } => *height,
		}
	}

	pub fn source(&self) -> StateMachine {
		match self {
			TxReceipt::Request { query, .. } => query.source_chain,
			TxReceipt::Response { query, .. } => query.source_chain,
		}
	}
}

/// A type that represents the location where state proof queries should be directed
#[derive(Debug, Clone)]
pub enum StateProofQueryType {
	/// Query the proof for these keys from the ismp module
	Ismp(Vec<Vec<u8>>),
	/// Query the proof for these keys from the global state
	Arbitrary(Vec<Vec<u8>>),
}

/// Cloneable error, used in place of `anyhow::Error`` which does not implement `Clone` required by
/// tokio::sync::broadcast
#[derive(Clone, Debug)]
pub struct StreamError(pub String);

impl From<StreamError> for anyhow::Error {
	fn from(value: StreamError) -> Self {
		anyhow!("{value:?}")
	}
}

impl From<anyhow::Error> for StreamError {
	fn from(value: anyhow::Error) -> Self {
		Self(format!("{value:?}"))
	}
}

/// Stream alias
pub type BoxStream<I> = Pin<Box<dyn Stream<Item = Result<I, StreamError>> + Send + 'static>>;

pub struct Hasher;

impl Keccak256 for Hasher {
	fn keccak256(bytes: &[u8]) -> H256 {
		keccak_256(bytes).into()
	}
}

#[derive(Debug, Default)]
pub struct TxResult {
	pub receipts: Vec<TxReceipt>,
	pub unsuccessful: Vec<Message>,
}

#[async_trait::async_trait]
pub trait IsmpProvider: ByzantineHandler + Send + Sync {
	/// Query the latest consensus state of a client
	async fn query_consensus_state(
		&self,
		at: Option<u64>,
		id: ConsensusStateId,
	) -> Result<Vec<u8>, anyhow::Error>;

	/// Query the latest height at which some state machine was last updated
	async fn query_latest_height(&self, id: StateMachineId) -> Result<u32, anyhow::Error>;

	/// Query the finalized latest height of this host
	async fn query_finalized_height(&self) -> Result<u64, anyhow::Error>;

	/// Query the State machine commitment at the provided height
	async fn query_state_machine_commitment(
		&self,
		height: StateMachineHeight,
	) -> Result<StateCommitment, anyhow::Error>;

	/// Query the timestamp at which the client was last updated
	async fn query_state_machine_update_time(
		&self,
		height: StateMachineHeight,
	) -> Result<Duration, anyhow::Error>;

	/// Query the challenge period for client
	async fn query_challenge_period(&self, id: StateMachineId) -> Result<Duration, anyhow::Error>;

	/// Query the latest timestamp for chain
	async fn query_timestamp(&self) -> Result<Duration, anyhow::Error>;

	/// Query a requests proof
	/// Return the scale encoded proof
	async fn query_requests_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		counterparty: StateMachine,
	) -> Result<Vec<u8>, anyhow::Error>;

	/// Query a responses proof
	/// Return the scale encoded proof
	async fn query_responses_proof(
		&self,
		at: u64,
		keys: Vec<Query>,
		counterparty: StateMachine,
	) -> Result<Vec<u8>, anyhow::Error>;

	/// Query state proof for some keys, return scaled encoded proof
	async fn query_state_proof(
		&self,
		at: u64,
		keys: StateProofQueryType,
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
	/// NOTE: Results must be returned in the same order as the messages
	async fn estimate_gas(
		&self,
		msg: Vec<Message>,
	) -> Result<Vec<EstimateGasReturnParams>, anyhow::Error>;

	/// Should return fee relayer would be recieving to relay a request mesage giving a hash
	/// (message commiment)
	/// Should return Erc20 standard type with 18 decimals value
	async fn query_request_fee_metadata(&self, hash: H256) -> Result<U256, anyhow::Error>;

	/// Should return the relayer delivered this request
	/// if it has been delivered
	async fn query_request_receipt(&self, _hash: H256) -> Result<Vec<u8>, anyhow::Error>;

	/// Should return the relayer delivered this response
	/// if it has been delivered
	async fn query_response_receipt(&self, _hash: H256) -> Result<Vec<u8>, anyhow::Error>;

	/// Should return fee relayer would be recieving to relay a responce mesage giving a hash
	/// (message commiment)
	/// Should return Erc20 standard type with 18 decimals value
	async fn query_response_fee_metadata(&self, hash: H256) -> Result<U256, anyhow::Error>;

	/// Return a stream that watches for updates to [`counterparty_state_id`], yields when new
	/// [`StateMachineUpdated`] event is observed for [`counterparty_state_id`]
	async fn state_machine_update_notification(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<StateMachineUpdated>, anyhow::Error>;

	/// Return a stream that watches for state machine commitment vetoes, starting at [`from`]
	/// yields when a [`StateCommitmentVetoed`] event is observed for [`height`]
	async fn state_commitment_vetoed_notification(
		&self,
		from: u64,
		height: StateMachineHeight,
	) -> BoxStream<StateCommitmentVetoed>;

	/// This should be used to submit new messages [`Vec<Message>`] from a counterparty chain to
	/// this chain.
	///
	/// Should only return Ok if the transaction was successfully inserted into a block.
	/// Should return a list of requests and responses that where successfully processed
	async fn submit(
		&self,
		messages: Vec<Message>,
		coprocessor: StateMachine,
	) -> Result<TxResult, anyhow::Error>;

	/// This method should return the key used to be used to query the state proof for the request
	/// commitment
	fn request_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>>;

	/// This method should return the key used to be used to query the state proof for the request
	/// receipt
	fn request_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>>;

	/// This method should return the key used to be used to query the state proof for the response
	/// commitment
	fn response_commitment_full_key(&self, commitment: H256) -> Vec<Vec<u8>>;

	/// This method should return the key used to be used to query the state proof for the response
	/// receipt
	fn response_receipt_full_key(&self, commitment: H256) -> Vec<Vec<u8>>;

	/// Relayer's address on this chain
	fn address(&self) -> Vec<u8>;

	/// Sign a prehashed message using the Relayer's private key
	fn sign(&self, msg: &[u8]) -> Signature;

	/// Set the initial height with the finalized height on counterparty
	async fn set_latest_finalized_height(
		&mut self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error>;

	/// Set the initial consensus state for a given consensus state id on this chain
	async fn set_initial_consensus_state(
		&self,
		message: CreateConsensusState,
	) -> Result<(), anyhow::Error>;

	/// Temporary: Veto a misrepresentative state commiment at the provided height
	async fn veto_state_commitment(&self, height: StateMachineHeight) -> Result<(), anyhow::Error>;

	/// Fetch the host params for given state machine
	async fn query_host_params(
		&self,
		state_machine: StateMachine,
	) -> Result<HostParam<u128>, anyhow::Error>;

	/// The max number of concurrent queries that can be made to the rpc node
	fn max_concurrent_queries(&self) -> usize {
		10
	}

	async fn fee_token_decimals(&self) -> Result<u8, anyhow::Error>;
}

/// Provides an interface for handling byzantine behaviour. Implementations of this should watch for
/// eclipse attacks, as well as invalid state transitions.
#[async_trait::async_trait]
pub trait ByzantineHandler {
	/// Check the state machine update event for byzantine behaviour and challenge it.
	async fn check_for_byzantine_attack(
		&self,
		coprocessor: StateMachine,
		counterparty: Arc<dyn IsmpProvider>,
		challenge_event: StateMachineUpdated,
	) -> Result<(), anyhow::Error>;

	/// Return a stream that watches for updates to [`counterparty_state_id`], yields when new
	/// [`Vec<StateMachineUpdated>`] event is observed for [`counterparty_state_id`]
	async fn state_machine_updates(
		&self,
		counterparty_state_id: StateMachineId,
	) -> Result<BoxStream<Vec<StateMachineUpdated>>, anyhow::Error>;
}

/// Provides an interface for the chain to the relayer core for submitting Ismp messages as well as
#[async_trait::async_trait]
pub trait IsmpHost: Send + Sync {
	/// Begin the task of submitting [`ConsensusMessage`](ismp::messaging::ConsensusMessage) to the
	/// counterparty chain. Implementations are free to submit these messages however frequently
	/// they like. This method should never return unless it encounters an unrecoverable error, in
	/// which case the consensus relayer will be shut down.
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error>;

	/// Query the trusted, intitial consensus state for this host
	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error>;

	/// Return the instance of the [`IsmpProvider`] associated with this host
	fn provider(&self) -> Arc<dyn IsmpProvider>;
}

#[async_trait::async_trait]
pub trait HyperbridgeClaim {
	async fn available_amount(
		&self,
		_client: Arc<dyn IsmpProvider>,
		_chain: &StateMachine,
	) -> anyhow::Result<U256> {
		Ok(U256::from(0))
	}
	async fn accumulate_fees(&self, proof: WithdrawalProof) -> anyhow::Result<()>;
	async fn withdraw_funds(
		&self,
		client: Arc<dyn IsmpProvider>,
		chain: StateMachine,
	) -> anyhow::Result<WithdrawFundsResult>;
	/// Check if this key has been claimed
	async fn check_claimed(&self, key: Key) -> anyhow::Result<bool>;
}

#[async_trait::async_trait]
pub trait HandleGetResponse {
	async fn submit_get_response(&self, _msg: GetRequestsWithProof) -> anyhow::Result<()> {
		Ok(())
	}

	async fn dry_run_submission(&self, _msg: GetRequestsWithProof) -> anyhow::Result<()> {
		Ok(())
	}
}

#[derive(Encode, Decode, Clone)]
pub struct WithdrawFundsResult {
	/// Post request emitted by the withdraw request
	pub post: PostRequest,
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

pub async fn wait_for_challenge_period(
	client: Arc<dyn IsmpProvider>,
	last_consensus_update: Duration,
	counterparty_state_id: StateMachineId,
) -> anyhow::Result<()> {
	let challenge_period = client.query_challenge_period(counterparty_state_id).await?;
	if challenge_period != Duration::ZERO {
		log::info!(
			"Waiting for challenge period {challenge_period:?} for {} on {}",
			counterparty_state_id.state_id,
			client.name()
		);
	}

	tokio::time::sleep(challenge_period).await;
	let current_timestamp = client.query_timestamp().await?;
	let mut delay = current_timestamp.saturating_sub(last_consensus_update);

	while delay <= challenge_period {
		tokio::time::sleep(challenge_period - delay).await;
		let current_timestamp = client.query_timestamp().await?;
		delay = current_timestamp.saturating_sub(last_consensus_update);
	}
	Ok(())
}

#[instrument(name = "Waiting for state machine update on hyperbridge", skip_all)]
pub async fn wait_for_state_machine_update(
	state_id: StateMachineId,
	hyperbridge: Arc<dyn IsmpProvider>,
	counterparty: Arc<dyn IsmpProvider>,
	height: u64,
) -> anyhow::Result<u64> {
	let latest_height = hyperbridge.query_latest_height(state_id).await?.into();
	if latest_height >= height {
		observe_challenge_period(counterparty, hyperbridge, latest_height).await?;
		return Ok(latest_height);
	}

	let mut stream = hyperbridge.state_machine_update_notification(state_id).await?;

	while let Some(res) = stream.next().await {
		match res {
			Ok(event) =>
				if event.latest_height >= height {
					return Ok(event.latest_height);
				},
			Err(err) => {
				log::error!("State machine update stream returned an error {err:?}")
			},
		}
	}

	Err(anyhow::anyhow!("State Machine update stream returned None"))
}

#[instrument(name = "Waiting for challenge period to elapse on hyperbridge", skip_all)]
pub async fn observe_challenge_period(
	chain: Arc<dyn IsmpProvider>,
	hyperbridge: Arc<dyn IsmpProvider>,
	height: u64,
) -> anyhow::Result<()> {
	let height = StateMachineHeight { id: chain.state_machine_id(), height };
	let last_consensus_update = hyperbridge.query_state_machine_update_time(height).await?;
	wait_for_challenge_period(hyperbridge, last_consensus_update, chain.state_machine_id()).await?;
	Ok(())
}
