use std::sync::Arc;

use ethers::providers::Middleware;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::StateMachineUpdated,
	host::StateMachine,
};
use tesseract_primitives::{ByzantineHandler, IsmpProvider};

use crate::EvmClient;

#[async_trait::async_trait]
impl ByzantineHandler for EvmClient {
	async fn check_for_byzantine_attack(
		&self,
		_coprocessor: StateMachine,
		counterparty: Arc<dyn IsmpProvider>,
		event: StateMachineUpdated,
	) -> Result<(), anyhow::Error> {
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.state_machine,
				consensus_state_id: self.consensus_state_id,
			},
			height: event.latest_height,
		};
		let Some(header) = self.client.get_block(event.latest_height).await? else {
			// If block header is not found veto the state commitment
			log::info!(
				"Vetoing State Machine Update for {} on {}",
				self.state_machine,
				counterparty.state_machine_id().state_id
			);
			counterparty.veto_state_commitment(height).await?;
			return Ok(())
		};

		let state_machine_commitment = counterparty.query_state_machine_commitment(height).await?;
		if header.state_root != state_machine_commitment.state_root {
			log::info!(
				"Vetoing State Machine Update for {} on {}",
				self.state_machine,
				counterparty.state_machine_id().state_id
			);
			counterparty.veto_state_commitment(height).await?;
		}

		Ok(())
	}
}
