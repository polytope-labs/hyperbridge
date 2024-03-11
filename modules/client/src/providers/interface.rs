#![allow(async_fn_in_trait)]

use crate::types::{BoxStream, EventMetadata};
use core::time::Duration;
use ethers::{prelude::H256, types::H160};
use ismp::{
    consensus::{ConsensusStateId, StateCommitment, StateMachineHeight, StateMachineId},
    events::{Event, StateMachineUpdated},
    messaging::Message,
    router::{Post, PostResponse},
};
use ismp_solidity_abi::evm_host::PostRequestHandledFilter;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

#[derive(Eq, PartialEq, Clone)]
pub enum RequestOrResponse {
    Request(Post),
    Response(PostResponse),
}

/// Holds an event along with relevant metadata about the event
#[derive(Serialize, Deserialize, Clone)]
pub struct WithMetadata<T> {
    /// The event metdata
    pub meta: EventMetadata,
    /// The event in question
    pub event: T,
}

pub trait Client: Clone + Send + Sync + 'static {
    /// Query the latest block height
    async fn query_latest_block_height(&self) -> Result<u64, anyhow::Error>;

    /// Returns the State Machine ID
    fn state_machine_id(&self) -> StateMachineId;

    /// Returns the timestamp from the ISMP host of a State machine
    async fn query_timestamp(&self) -> Result<Duration, anyhow::Error>;

    /// Query request receipt from a ISMP host given the hash of the request
    async fn query_request_receipt(&self, request_hash: H256) -> Result<H160, anyhow::Error>;

    // Queries state proof for some keys
    async fn query_state_proof(&self, at: u64, key: Vec<Vec<u8>>)
        -> Result<Vec<u8>, anyhow::Error>;

    // Query the response receipt from the ISMP host on the destination chain
    async fn query_response_receipt(&self, request_commitment: H256)
        -> Result<H160, anyhow::Error>;

    // Returns the event stream of this chain that yields when it finds an event that contains the
    // given post or response
    async fn ismp_events_stream(
        &self,
        item: RequestOrResponse,
    ) -> Result<BoxStream<WithMetadata<Event>>, anyhow::Error>;

    /// Should return all the events emitted between the given block range
    async fn query_ismp_event(
        &self,
        range: RangeInclusive<u64>,
    ) -> Result<Vec<WithMetadata<Event>>, anyhow::Error>;

    // Returns a stream of the PostRequestHandled on the ISMP host of this chain
    async fn post_request_handled_stream(
        &self,
        commitment: H256,
    ) -> Result<BoxStream<WithMetadata<PostRequestHandledFilter>>, anyhow::Error>;

    async fn query_state_machine_commitment(
        &self,
        id: StateMachineHeight,
    ) -> Result<StateCommitment, anyhow::Error>;

    // Get state machine hyperbridge consensus state machine height
    async fn state_machine_update_notification(
        &self,
        counterparty_state_id: StateMachineId,
    ) -> Result<BoxStream<WithMetadata<StateMachineUpdated>>, anyhow::Error>;

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

    /// Return the encoded unsigned transaction bytes for this message
    fn encode(&self, msg: Message) -> Result<Vec<u8>, anyhow::Error>;

    /// Submit message to chain
    async fn submit(&self, msg: Message) -> Result<EventMetadata, anyhow::Error>;

    /// Query the timestamp at which the client was last updated
    async fn query_state_machine_update_time(
        &self,
        height: StateMachineHeight,
    ) -> Result<Duration, anyhow::Error>;

    /// Query the challenge period for client
    async fn query_challenge_period(&self, id: ConsensusStateId)
        -> Result<Duration, anyhow::Error>;
}

pub async fn wait_for_challenge_period<C: Client>(
    client: &C,
    last_consensus_update: Duration,
    challenge_period: Duration,
) -> anyhow::Result<()> {
    wasm_timer::Delay::new(challenge_period).await?;
    let current_timestamp = client.query_timestamp().await?;
    let mut delay = current_timestamp.saturating_sub(last_consensus_update);

    while delay <= challenge_period {
        wasm_timer::Delay::new(challenge_period.saturating_sub(delay)).await?;
        let current_timestamp = client.query_timestamp().await?;
        delay = current_timestamp.saturating_sub(last_consensus_update);
    }

    Ok(())
}
