use crate::optimism::client::OpHost;
use anyhow::{anyhow, Error};
use codec::{Decode, Encode};
use ethers::prelude::Middleware;
use futures::stream;
use geth_primitives::CodecHeader;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	events::StateMachineUpdated,
	messaging::{ConsensusMessage, CreateConsensusState},
};
use tesseract_primitives::{BoxStream, ByzantineHandler, IsmpHost, IsmpProvider, Reconnect};

#[async_trait::async_trait]
impl ByzantineHandler for OpHost {
	async fn query_consensus_message(
		&self,
		event: StateMachineUpdated,
	) -> Result<ConsensusMessage, anyhow::Error> {
		let header: CodecHeader = self
			.op_execution_client
			.get_block(event.latest_height)
			.await?
			.ok_or_else(|| anyhow!("Header should be available"))?
			.into();
		Ok(ConsensusMessage {
			consensus_proof: header.encode(),
			consensus_state_id: self.consensus_state_id,
		})
	}

	async fn check_for_byzantine_attack<C: IsmpHost + IsmpProvider>(
		&self,
		counterparty: &C,
		consensus_message: ConsensusMessage,
	) -> Result<(), anyhow::Error> {
		let header = CodecHeader::decode(&mut &*consensus_message.consensus_proof)?;
		let height = StateMachineHeight {
			id: StateMachineId {
				state_id: self.config.state_machine(),
				consensus_state_id: self.consensus_state_id,
			},
			height: header.number.low_u64(),
		};
		let state_machine_commitment = counterparty.query_state_machine_commitment(height).await?;
		if state_machine_commitment.state_root != header.state_root {
			// Submit Freeze message
			log::info!(
				"Freezing {:?} on {:?}",
				self.config.state_machine(),
				counterparty.state_machine_id().state_id
			);
			counterparty.freeze_state_machine(height.id).await?;
		}
		Ok(())
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
	async fn reconnect(&mut self) -> Result<(), anyhow::Error> {
		let new_host = OpHost::new(&self.config).await?;
		*self = new_host;
		Ok(())
	}
}
