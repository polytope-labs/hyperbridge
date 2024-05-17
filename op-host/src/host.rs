use std::sync::Arc;

use anyhow::anyhow;
use ethers::{providers::Middleware, types::SyncingStatus};
use futures::StreamExt;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	messaging::CreateConsensusState,
};
use tesseract_primitives::{ByzantineHandler, IsmpHost, IsmpProvider, StateMachineUpdated};

use crate::OpHost;

#[async_trait::async_trait]
impl IsmpHost for OpHost {
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let mut stream = Box::pin(futures::stream::pending::<()>());
		while let Some(_) = stream.next().await {}
		Err(anyhow!(
			"{}-{} consensus task has failed, Please restart relayer",
			self.provider().name(),
			counterparty.name()
		))
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
impl ByzantineHandler for OpHost {
	async fn check_for_byzantine_attack(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
		event: StateMachineUpdated,
	) -> Result<(), anyhow::Error> {
		let sync_status = match self.op_execution_client.syncing().await? {
			SyncingStatus::IsFalse => false,
			_ => true,
		};
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.evm.state_machine,
				consensus_state_id: self.consensus_state_id,
			},
			height: event.latest_height,
		};
		let Some(header) = self.op_execution_client.get_block(event.latest_height).await? else {
			// If block header is not found and node is fully synced, veto the state commitment
			if !sync_status {
				log::info!(
					"Vetoing State Machine Update for {:?} on {:?}",
					self.evm.state_machine,
					counterparty.name()
				);
				counterparty.veto_state_commitment(height).await?;
				return Ok(())
			} else {
				Err(anyhow!("Node is still syncing, cannot fetch finalized block"))?
			}
		};

		let state_machine_commitment = counterparty.query_state_machine_commitment(height).await?;
		if state_machine_commitment.state_root != header.state_root {
			log::info!(
				"Vetoing State Machine Update for {:?} on {:?}",
				self.evm.state_machine,
				counterparty.name()
			);
			counterparty.veto_state_commitment(height).await?;
		}
		Ok(())
	}
}
