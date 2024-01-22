use crate::optimism::client::OpHost;
use anyhow::{anyhow, Error};
use futures::stream;
use ismp::{events::StateMachineUpdated, messaging::CreateConsensusState};
use tesseract_primitives::{BoxStream, ByzantineHandler, IsmpHost, IsmpProvider, Reconnect};

#[async_trait::async_trait]
impl ByzantineHandler for OpHost {
	async fn query_consensus_message(
		&self,
		_challenge_event: StateMachineUpdated,
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

	async fn get_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
		Ok(None)
	}
}

#[async_trait::async_trait]
impl Reconnect for OpHost {
	async fn reconnect<C: IsmpProvider>(&mut self, _counterparty: &C) -> Result<(), anyhow::Error> {
		let new_host = OpHost::new(&self.config).await?;
		*self = new_host;
		Ok(())
	}
}
