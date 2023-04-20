use crate::ParachainClient;
use ismp::messaging::ConsensusMessage;
use tesseract_primitives::{ByzantineHandler, ChallengePeriodStarted, IsmpHost};

#[async_trait::async_trait]
impl<T> ByzantineHandler for ParachainClient<T>
where
    T: subxt::Config,
{
    async fn query_consensus_message(
        &self,
        _challenge_event: ChallengePeriodStarted,
    ) -> Result<ConsensusMessage, anyhow::Error> {
        todo!()
    }

    async fn check_for_byzantine_attack<C: IsmpHost>(
        &self,
        _counterparty: &C,
        _consensus_message: ConsensusMessage,
    ) -> Result<(), anyhow::Error> {
        todo!()
    }
}
