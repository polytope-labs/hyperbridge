use crate::optimism::client::OpHost;
use anyhow::anyhow;
use futures::stream;
use tesseract_primitives::{
    BoxStream, ByzantineHandler, ChallengePeriodStarted, IsmpHost, IsmpProvider,
};

#[async_trait::async_trait]
impl ByzantineHandler for OpHost {
    async fn query_consensus_message(
        &self,
        _challenge_event: ChallengePeriodStarted,
    ) -> Result<ismp::messaging::ConsensusMessage, anyhow::Error> {
        Err(anyhow!("No consensus messages"))
    }

    async fn check_for_byzantine_attack<T: IsmpHost>(
        &self,
        _counterparty: &T,
        _consensus_message: ismp::messaging::ConsensusMessage,
    ) -> Result<(), anyhow::Error> {
        Err(anyhow!("No byzantine faults"))
    }
}

#[async_trait::async_trait]
impl IsmpHost for OpHost {
    async fn consensus_notification<I>(
        &self,
        _counterparty: I,
    ) -> Result<BoxStream<ismp::messaging::ConsensusMessage>, anyhow::Error>
    where
        I: IsmpHost + IsmpProvider + Clone + 'static,
    {
        Ok(Box::pin(stream::pending()))
    }
}
