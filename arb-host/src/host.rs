use std::sync::Arc;

use ismp::messaging::{ConsensusMessage, CreateConsensusState};
use tesseract_primitives::{
	BoxStream, ByzantineHandler, IsmpHost, IsmpProvider, StateMachineUpdated,
};

use crate::ArbHost;

#[async_trait::async_trait]
impl IsmpHost for ArbHost {
	async fn consensus_notification(
		&self,
		_counterparty: Arc<dyn IsmpProvider>,
	) -> Result<BoxStream<ConsensusMessage>, anyhow::Error> {
		unimplemented!()
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		unimplemented!()
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		unimplemented!()
	}
}

#[async_trait::async_trait]
impl ByzantineHandler for ArbHost {
	async fn query_consensus_message(
		&self,
		_challenge_event: StateMachineUpdated,
	) -> Result<ConsensusMessage, anyhow::Error> {
		unimplemented!()
	}

	async fn check_for_byzantine_attack(
		&self,
		_counterparty: Arc<dyn IsmpHost>,
		_consensus_message: ConsensusMessage,
	) -> Result<(), anyhow::Error> {
		unimplemented!()
	}
}
