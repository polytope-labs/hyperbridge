use crate::ParachainClient;
use futures::Stream;
use ismp::messaging::{ConsensusMessage, Message};
use std::pin::Pin;
use tesseract_primitives::{IsmpHost, StateMachineUpdated};

#[async_trait::async_trait]
impl<T> IsmpHost for ParachainClient<T>
where
    T: subxt::Config,
{
    fn name(&self) -> &str {
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
    ) -> Pin<Box<dyn Stream<Item = StateMachineUpdated> + Send>> {
        todo!()
    }

    async fn submit(&self, _messages: Vec<Message>) -> Result<Self::TransactionId, anyhow::Error> {
        todo!()
    }
}
