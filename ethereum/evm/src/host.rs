use crate::EvmClient;
use anyhow::{anyhow, Error};
use ismp::{events::StateMachineUpdated, messaging::CreateConsensusState};
use tesseract_primitives::{BoxStream, ByzantineHandler, IsmpHost, IsmpProvider};

#[async_trait::async_trait]
impl<I> ByzantineHandler for EvmClient<I>
where
	I: IsmpHost,
{
	async fn query_consensus_message(
		&self,
		challenge_event: StateMachineUpdated,
	) -> Result<ismp::messaging::ConsensusMessage, anyhow::Error> {
		self.host
			.as_ref()
			.ok_or_else(|| anyhow!("Host not initialized"))?
			.query_consensus_message(challenge_event)
			.await
	}

	async fn check_for_byzantine_attack<T: IsmpHost + IsmpProvider>(
		&self,
		counterparty: &T,
		consensus_message: ismp::messaging::ConsensusMessage,
	) -> Result<(), anyhow::Error> {
		self.host
			.as_ref()
			.ok_or_else(|| anyhow!("Host not initialized"))?
			.check_for_byzantine_attack(counterparty, consensus_message)
			.await
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
		self.host
			.as_ref()
			.ok_or_else(|| anyhow!("Host not initialized"))?
			.consensus_notification(counterparty)
			.await
	}

	async fn get_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
		self.host
			.as_ref()
			.ok_or_else(|| anyhow!("Host not initialized"))?
			.get_initial_consensus_state()
			.await
	}
}

impl<T: IsmpHost + Clone> Clone for EvmClient<T> {
	fn clone(&self) -> Self {
		Self {
			host: self.host.clone(),
			client: self.client.clone(),
			signer: self.signer.clone(),
			address: self.address.clone(),
			consensus_state_id: self.consensus_state_id,
			state_machine: self.state_machine,
			initial_height: self.initial_height,
			ismp_host: self.ismp_host,
			handler: self.handler,
			nonce_provider: self.nonce_provider.clone(),
			gas_limit: self.gas_limit,
			config: self.config.clone(),
			rpc_client: self.rpc_client.clone(),
		}
	}
}
