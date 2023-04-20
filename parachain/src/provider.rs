use crate::ParachainClient;
use ismp::{
    consensus_client::{ConsensusClientId, StateMachineId},
    router::{Request, Response},
};
use std::time::Duration;
use tesseract_primitives::{IsmpProvider, Query, StateMachineUpdated};

#[async_trait::async_trait]
impl<T> IsmpProvider for ParachainClient<T>
where
    T: subxt::Config,
{
    type TransactionId = ();

    async fn query_consensus_state(
        &self,
        _at: u64,
        _id: ConsensusClientId,
    ) -> Result<Vec<u8>, anyhow::Error> {
        todo!()
    }

    async fn query_latest_state_machine_height(
        &self,
        _id: StateMachineId,
    ) -> Result<u32, anyhow::Error> {
        todo!()
    }

    async fn query_consensus_update_time(
        &self,
        _id: ConsensusClientId,
    ) -> Result<Duration, anyhow::Error> {
        todo!()
    }

    async fn query_requests_proof(
        &self,
        _at: u64,
        _keys: Vec<Query>,
    ) -> Result<Vec<u8>, anyhow::Error> {
        todo!()
    }

    async fn query_responses_proof(
        &self,
        _at: u64,
        _keys: Vec<Query>,
    ) -> Result<Vec<u8>, anyhow::Error> {
        todo!()
    }

    async fn query_timeout_proof(
        &self,
        _at: u64,
        _keys: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, anyhow::Error> {
        todo!()
    }

    async fn query_ismp_events(
        &self,
        _event: StateMachineUpdated,
    ) -> Result<Vec<pallet_ismp::events::Event>, anyhow::Error> {
        todo!()
    }

    async fn query_requests(
        &self,
        _at: u64,
        _keys: Vec<Query>,
    ) -> Result<Vec<Request>, anyhow::Error> {
        todo!()
    }

    async fn query_responses(
        &self,
        _at: u64,
        _keys: Vec<Query>,
    ) -> Result<Vec<Response>, anyhow::Error> {
        todo!()
    }
}
