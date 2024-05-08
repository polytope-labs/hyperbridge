use std::sync::Arc;

use crate::ArbHost;
use anyhow::anyhow;
use ethers::providers::Middleware;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	messaging::{ConsensusMessage, CreateConsensusState},
};
use tesseract_primitives::{
	BoxStream, ByzantineHandler, IsmpHost, IsmpProvider, StateMachineUpdated,
};

#[async_trait::async_trait]
impl IsmpHost for ArbHost {
	async fn consensus_notification(
		&self,
		_counterparty: Arc<dyn IsmpProvider>,
	) -> Result<BoxStream<ConsensusMessage>, anyhow::Error> {
		Ok(Box::pin(futures::stream::pending()))
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		Ok(None)
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}

#[async_trait::async_trait]
impl ByzantineHandler for ArbHost {
	async fn check_for_byzantine_attack(
		&self,
		counterparty: Arc<dyn IsmpHost>,
		event: StateMachineUpdated,
	) -> Result<(), anyhow::Error> {
		let header =
			self.arb_execution_client.get_block(event.latest_height).await?.ok_or_else(|| {
				anyhow!("Failed to fetch header in {:?} byzantine handler", self.evm.state_machine)
			})?;
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.evm.state_machine,
				consensus_state_id: self.consensus_state_id,
			},
			height: event.latest_height,
		};
		let counterparty_provider = counterparty.provider();
		let state_machine_commitment =
			counterparty_provider.query_state_machine_commitment(height).await?;
		if state_machine_commitment.state_root != header.state_root {
			log::info!(
				"Vetoing State Machine Update for {:?} on {:?}",
				self.evm.state_machine,
				counterparty_provider.name()
			);
			counterparty_provider.veto_state_commitment(height).await?;
		}
		Ok(())
	}
}
