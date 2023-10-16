use crate::EvmClient;
use std::sync::Arc;
use tesseract_primitives::{
	BoxStream, ByzantineHandler, ChallengePeriodStarted, IsmpHost, IsmpProvider, Reconnect,
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

#[async_trait::async_trait]
impl<T> Reconnect for EvmClient<T>
where
	T: IsmpHost + Clone,
{
	async fn reconnect<C: IsmpProvider>(&mut self, counterparty: &C) -> Result<(), anyhow::Error> {
		let nonce_provider = self.nonce_provider.clone();
		self.host.reconnect(counterparty).await?;
		let host = self.host.clone();
		let latest_height = *self.latest_height.lock();
		let mut new_client = EvmClient::new(host, self.config.clone(), counterparty).await?;
		if let Some(nonce_provider) = nonce_provider {
			new_client.set_nonce_provider(nonce_provider);
		}
		new_client.set_latest_height(latest_height);
		*self = new_client;
		Ok(())
	}
}

impl<T: IsmpHost + Clone> Clone for EvmClient<T> {
	fn clone(&self) -> Self {
		Self {
			host: self.host.clone(),
			client: self.client.clone(),
			signer: self.signer.clone(),
			consensus_state_id: self.consensus_state_id,
			state_machine: self.state_machine,
			latest_height: Arc::clone(&self.latest_height),
			ismp_host: self.ismp_host,
			handler: self.handler,
			nonce_provider: self.nonce_provider.clone(),
			gas_limit: self.gas_limit,
			config: self.config.clone(),
			rpc_client: self.rpc_client.clone(),
		}
	}
}
