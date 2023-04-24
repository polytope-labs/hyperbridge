use crate::ParachainClient;
use futures::Stream;
use ismp::{
    consensus::StateMachineId,
    messaging::{ConsensusMessage, Message},
};
use std::pin::Pin;
use tesseract_primitives::{IsmpHost, StateMachineUpdated};

#[async_trait::async_trait]
impl<T> IsmpHost for ParachainClient<T>
where
    T: subxt::Config + Send + Sync + Clone,
    T::Header: Send + Sync,
{
    fn name(&self) -> &str {
        todo!()
    }

    fn state_machine_id(&self) -> StateMachineId {
        todo!()
    }

    fn block_max_gas(&self) -> u64 {
        todo!()
    }

    async fn estimate_gas(&self, _msg: Vec<Message>) -> Result<u64, anyhow::Error> {
        todo!()
    }

    async fn consensus_notification(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<ConsensusMessage, anyhow::Error>> + Send>> {
        todo!()
    }

    async fn state_machine_update_notification(
        &self,
        counterparty_id: StateMachineId,
    ) -> Pin<Box<dyn Stream<Item = Result<StateMachineUpdated, anyhow::Error>> + Send>> {
        self.state_machine_update_notification_stream(counterparty_id)
            .await
            .expect("Failed to get state machine notification stream")
    }

    async fn submit(&self, _messages: Vec<Message>) -> Result<Self::TransactionId, anyhow::Error> {
        todo!()
    }
}
