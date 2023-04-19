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

use futures::Stream;
use ismp::{
    consensus_client::{ConsensusClientId, StateMachineHeight, StateMachineId},
    host::ChainID,
    messaging::{ConsensusMessage, Message},
    router::{Request, Response},
};
use pallet_ismp::events::Event;
use std::{pin::Pin, time::Duration};

/// Provides an interface for accessing new events and ISMP data on the chain which must be
/// relayed to the counterparty chain.

pub struct Query {
    pub source_chain: ChainID,
    pub dest_chain: ChainID,
    pub nonce: u64,
}

pub struct StateMachineUpdated {
    pub state_machine_id: StateMachineId,
    pub latest_height: u64,
    pub previous_height: u64,
}

pub struct ChallengePeriodStarted {
    /// State machine update still in challenge period
    pub state_machine_height: StateMachineHeight,
    /// Height at which this event was emitted on the host chain
    pub host_chain_height: u64,
}

#[async_trait::async_trait]
pub trait IsmpProvider {
    /// Error type, just needs to implement standard error trait.
    type Error: std::error::Error + From<String> + Send + Sync + 'static;

    /// Transaction  Id type for this chain
    type TransactionId;

    /// Query the latest consensus state of a client
    async fn query_consensus_state(
        &self,
        at: u64,
        id: ConsensusClientId,
    ) -> Result<Vec<u8>, Self::Error>;

    /// Query the timestamp at which the client was last updated
    async fn query_consensus_update_time(
        &self,
        id: ConsensusClientId,
    ) -> Result<Duration, Self::Error>;

    /// Query a requests proof
    async fn query_requests_proof(&self, at: u64, keys: Vec<Query>)
        -> Result<Vec<u8>, Self::Error>;

    /// Query a responses proof
    async fn query_responses_proof(
        &self,
        at: u64,
        keys: Vec<Query>,
    ) -> Result<Vec<u8>, Self::Error>;

    /// Query a request timeout proof
    /// keys contains ismp canonical keys without any chain specific prefixes or suffixes
    async fn query_timeout_proof(
        &self,
        at: u64,
        keys: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, Self::Error>;

    /// Query all ismp events on host that can be processed for a [`StateMachineUpdated`]
    /// event on the counterparty
    async fn query_ismp_events<T: IsmpHost>(
        &self,
        event: StateMachineUpdated,
    ) -> Result<Vec<Event>, Self::Error>;

    /// Query requests
    async fn query_requests(&self, at: u64, keys: Vec<Query>) -> Result<Vec<Request>, Self::Error>;

    /// Query responses
    async fn query_responses(
        &self,
        at: u64,
        keys: Vec<Query>,
    ) -> Result<Vec<Response>, Self::Error>;
}

/// Interface for signing transactions
pub trait KeyProvider {
    /// Returns an account type supported by the chain
    fn account_id(&self) -> String;
}

/// Provides an interface for handling byzantine behaviour. Implementations of this should watch for
/// eclipse attacks, as well as invalid state transitions.
#[async_trait::async_trait]
pub trait ByzantineHandler {
    /// Returns the [`ConsensusMessage`] that caused the emission of  [`ChallengePeriodStarted`]
    /// event
    async fn query_consensus_message(
        &self,
        challenge_event: ChallengePeriodStarted,
    ) -> Result<ConsensusMessage, anyhow::Error>;

    /// Check the client message for byzantine behaviour and submit it to the chain if any.
    async fn check_for_byzantine_attack<C: IsmpHost>(
        &self,
        counterparty: &C,
        consensus_message: ConsensusMessage,
    ) -> Result<(), anyhow::Error>;
}

/// Provides an interface for the chain to the relayer core for submitting Ismp messages as well as
#[async_trait::async_trait]
pub trait IsmpHost: IsmpProvider + ByzantineHandler + KeyProvider + Send + Sync {
    /// Name of this chain, used in logs.
    fn name(&self) -> &str;

    /// Should return a numerical value for the max gas allowed for transactions in a block.
    fn block_max_gas(&self) -> u64;

    /// Should return a numerical estimate of the gas to be consumed for a batch of messages.
    async fn estimate_gas(&self, msg: Vec<Message>) -> Result<u64, Self::Error>;

    /// Return a stream that yields [`ConsensusMessage`] when a new consensus update can be sent to
    /// the counterparty
    async fn consensus_notification(
        &self,
    ) -> Pin<Box<dyn Stream<Item = ConsensusMessage> + Send + Sync>>;

    /// Return a stream that yields when new [`StateMachineUpdated`]
    /// event is observed
    async fn state_machine_update_notification(
        &self,
    ) -> Pin<Box<dyn Stream<Item = StateMachineUpdated> + Send + Sync>>;

    /// This should be used to submit new messages [`Vec<Message>`] from a counterparty chain to
    /// this chain.
    /// Should return the transaction id
    async fn submit(&self, messages: Vec<Message>) -> Result<Self::TransactionId, Self::Error>;
}
