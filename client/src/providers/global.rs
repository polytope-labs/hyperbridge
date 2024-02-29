use crate::types::{BoxStream, PostRequestHandledFilter, ResponseReceipt};
use core::time::Duration;
use ethers::{
    prelude::H256,
    providers::{SubscriptionStream, Ws},
    types::{Block, H160},
};
use ismp::{
    consensus::{StateCommitment, StateMachineHeight, StateMachineId},
    events::{Event, StateMachineUpdated},
    messaging::{Message, Proof},
    router::{Post, PostResponse},
};

#[derive(Eq, PartialEq, Clone)]
pub enum RequestOrResponse {
    Request(Post),
    Response(PostResponse),
}

// #[async_trait::async_trait]
pub trait Client: Clone + Send + Sync + 'static {
    /// Query the latest block height of a Chain (State Machine)
    #[allow(async_fn_in_trait)]
    async fn query_latest_block_height(&self) -> Result<u64, anyhow::Error>;

    /// Returns the State Machine ID
    fn state_machine_id(&self) -> StateMachineId;

    /// Returns the timestamp from the ISMP host of a State machine
    #[allow(async_fn_in_trait)]
    async fn query_timestamp(&self) -> Result<Duration, anyhow::Error>;

    /// Query request receipt from a ISMP host given the hash of the request
    #[allow(async_fn_in_trait)]
    async fn query_request_receipt(&self, request_hash: H256) -> Result<H160, anyhow::Error>;

    // Queries state proof for some keys
    #[allow(async_fn_in_trait)]
    async fn query_state_proof(&self, at: u64, key: Vec<Vec<u8>>)
        -> Result<Vec<u8>, anyhow::Error>;

    // Query the response receipt from the ISMP host on the destination chain
    #[allow(async_fn_in_trait)]
    async fn query_response_receipt(&self, request_commitment: H256)
        -> Result<H160, anyhow::Error>;

    // Returns the event stream of this chain that yields when it finds an event that contains the
    // given post or response
    #[allow(async_fn_in_trait)]
    async fn ismp_events_stream(
        &self,
        item: RequestOrResponse,
    ) -> Result<BoxStream<Event>, anyhow::Error>;

    // Returns a stream of the PostRequestHandled on the ISMP host of this chain
    #[allow(async_fn_in_trait)]
    async fn post_request_handled_stream(
        &self,
        commitment: H256,
    ) -> Result<BoxStream<PostRequestHandledFilter>, anyhow::Error>;

    #[allow(async_fn_in_trait)]
    async fn query_state_machine_commitment(
        &self,
        id: StateMachineHeight,
    ) -> Result<StateCommitment, anyhow::Error>;

    // Get state machine hyperbridge consensus state machine height
    #[allow(async_fn_in_trait)]
    async fn state_machine_update_notification(
        &self,
        counterparty_state_id: StateMachineId,
    ) -> Result<BoxStream<StateMachineUpdated>, anyhow::Error>;

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

    #[allow(async_fn_in_trait)]
    async fn submit(&self, msg: Message) -> Result<H256, anyhow::Error>;
}
