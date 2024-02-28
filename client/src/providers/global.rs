use crate::types::{BoxStream, PostRequestHandledFilter, ResponseReceipt};
use ethers::{
    prelude::H256,
    providers::{SubscriptionStream, Ws},
    types::{Block, H160},
};
use ismp::{consensus::StateMachineId, events::Event, messaging::Proof};
use ismp::events::StateMachineUpdated;

// #[async_trait::async_trait]
pub trait Client: Clone + Send + Sync + 'static {
    /// Query the latest block height of a Chain (State Machine)
    #[allow(async_fn_in_trait)]
    async fn query_latest_block_height(&self) -> Result<u64, anyhow::Error>;

    /// Returns the State Machine ID
    #[allow(async_fn_in_trait)]
    fn state_machine_id(&self) -> Result<StateMachineId, anyhow::Error>;

    /// Returns the timestamp from the ISMP host of a State machine
    #[allow(async_fn_in_trait)]
    async fn host_timestamp(&self) -> Result<u64, anyhow::Error>;

    /// Query request receipt from a ISMP host given the hash of the request
    #[allow(async_fn_in_trait)]
    async fn query_request_receipts(&self, request_hash: &H256) -> Result<H160, anyhow::Error>;

    // Queries request proof from an ISMP host (contract: (evm), pallet: Substrate)
    #[allow(async_fn_in_trait)]
    async fn query_request_proof(
        &self,
        request_query_commitment: &H256,
        at: u64,
    ) -> Result<Proof, anyhow::Error>;

    // Queries Response proof from ISMP host of (contract on evm, pallet on substrate)
    #[allow(async_fn_in_trait)]
    async fn query_response_proof(
        &self,
        request_query_commitment: &H256,
        at: u64,
    ) -> Result<Proof, anyhow::Error>;

    // Query the response receipt from the ISMP host on the destination chain
    #[allow(async_fn_in_trait)]
    async fn query_response_receipts(
        &self,
        response_hash: &H256,
    ) -> Result<ResponseReceipt, anyhow::Error>;

    // Returns the event stream of this chain on HYPERBRIDGE
    #[allow(async_fn_in_trait)]
    async fn event_stream(&self) -> Result<BoxStream<Event>, anyhow::Error>;

    // Returns a stream of the PostRequestHandled on the ISMP host of this chain
    #[allow(async_fn_in_trait)]
    async fn post_request_handled_stream(
        &self,
    ) -> Result<BoxStream<PostRequestHandledFilter>, anyhow::Error>;

    // Returns the latest state machine height from the ISMP host
    #[allow(async_fn_in_trait)]
    async fn query_state_machine_height(&self) -> Result<u64, anyhow::Error>;

    // Get state machine hyperbridge consensus state machine height
    #[allow(async_fn_in_trait)]
    async fn state_machine_update_notification(&self) -> Result<BoxStream<StateMachineUpdated>, anyhow::Error>;

    // Get the address of a ISMP handler for a chain
    #[allow(async_fn_in_trait)]
    fn ismp_handler(&self) -> H160;
}
