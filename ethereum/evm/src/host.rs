use crate::EvmClient;
use std::sync::Arc;
use tesseract_primitives::{
    BoxStream, ByzantineHandler, ChallengePeriodStarted, IsmpHost, IsmpProvider,
};

#[async_trait::async_trait]
impl<I> ByzantineHandler for EvmClient<I>
where
    I: IsmpHost,
{
    async fn query_consensus_message(
        &self,
        challenge_event: ChallengePeriodStarted,
    ) -> Result<ismp::messaging::ConsensusMessage, anyhow::Error> {
        self.host.query_consensus_message(challenge_event).await
    }

    async fn check_for_byzantine_attack<T: IsmpHost>(
        &self,
        counterparty: &T,
        consensus_message: ismp::messaging::ConsensusMessage,
    ) -> Result<(), anyhow::Error> {
        self.host.check_for_byzantine_attack(counterparty, consensus_message).await
    }
}

#[async_trait::async_trait]
impl<T> IsmpHost for EvmClient<T>
where
    T: IsmpHost + Clone,
{
    async fn consensus_notification<I>(
        &self,
        counterparty: I,
    ) -> Result<BoxStream<ismp::messaging::ConsensusMessage>, anyhow::Error>
    where
        I: IsmpHost + IsmpProvider + Clone + 'static,
    {
        self.host.consensus_notification(counterparty).await
    }
}

impl<T: IsmpHost + Clone> Clone for EvmClient<T> {
    fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            client: self.client.clone(),
            signer: self.signer.clone(),
            events: self.events.clone(),
            consensus_state_id: self.consensus_state_id,
            state_machine: self.state_machine,
            latest_state_machine_height: Arc::clone(&self.latest_state_machine_height),
            ismp_host_address: self.ismp_host_address,
            handler_address: self.handler_address,
            gas_limit: self.gas_limit,
        }
    }
}
