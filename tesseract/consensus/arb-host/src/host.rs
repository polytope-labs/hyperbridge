use std::sync::Arc;

use crate::ArbHost;
use anyhow::{anyhow, Error};

use codec::Encode;
use ethers::prelude::Middleware;
use futures::StreamExt;
use ismp::{
	consensus::{StateCommitment, StateMachineId},
	messaging::{ConsensusMessage, CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_arbitrum::{
	ArbitrumConsensusProof, ArbitrumConsensusType, ArbitrumUpdate, ConsensusState,
	ARBITRUM_CONSENSUS_CLIENT_ID,
};
use primitive_types::H160;
use tesseract_primitives::{IsmpHost, IsmpProvider};

#[async_trait::async_trait]
impl IsmpHost for ArbHost {
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let consensus_state =
			counterparty.query_consensus_state(None, self.consensus_state_id).await?;
		let consensus_state = ConsensusState::decode(&mut &*consensus_state)?;

		let l1_state_machine_id = StateMachineId {
			state_id: self.l1_state_machine,
			consensus_state_id: self.l1_consensus_state_id,
		};
		let mut stream = counterparty
			.state_machine_update_notification(l1_state_machine_id.clone())
			.await?;
		let mut latest_height = counterparty.query_latest_height(l1_state_machine_id).await? as u64;
		while let Some(res) = stream.next().await {
			match res {
				Ok(event) => match consensus_state.arbitrum_consensus_type {
					ArbitrumConsensusType::ArbitrumOrbit => {
						let event_height = event.latest_height;
						let latest_event = self.latest_event(latest_height, event_height).await?;

						if let Some(event) = latest_event {
							let payload = self.fetch_arbitrum_payload(event_height, event).await?;
							let update = ArbitrumUpdate {
								state_machine_id: StateMachineId {
									state_id: self.state_machine,
									consensus_state_id: self.consensus_state_id,
								},
								l1_height: event_height,
								proof: ArbitrumConsensusProof::ArbitrumOrbit(payload),
							};

							let consensus_message = ConsensusMessage {
								consensus_proof: update.encode(),
								consensus_state_id: self.consensus_state_id,
								signer: counterparty.address(),
							};

							let _ = counterparty
								.submit(
									vec![Message::Consensus(consensus_message)],
									counterparty.state_machine_id().state_id,
								)
								.await;

							latest_height = event_height;
						}
					},
					ArbitrumConsensusType::ArbitrumBold => {
						let event_height = event.latest_height;
						let latest_event =
							self.latest_assertion_event(latest_height, event_height).await?;

						if let Some(event) = latest_event {
							let payload =
								self.fetch_arbitrum_bold_payload(event_height, event).await?;
							let update = ArbitrumUpdate {
								state_machine_id: StateMachineId {
									state_id: self.state_machine,
									consensus_state_id: self.consensus_state_id,
								},
								l1_height: event_height,
								proof: ArbitrumConsensusProof::ArbitrumBold(payload),
							};

							let consensus_message = ConsensusMessage {
								consensus_proof: update.encode(),
								consensus_state_id: self.consensus_state_id,
								signer: counterparty.address(),
							};

							let _ = counterparty
								.submit(
									vec![Message::Consensus(consensus_message)],
									counterparty.state_machine_id().state_id,
								)
								.await;

							latest_height = event_height;
						}
					},
				},
				Err(err) => {
					log::error!("State machine update stream returned an error {err:?}")
				},
			}
		}

		Err(anyhow!(
			"{}-{} consensus task has failed, Please restart relayer",
			self.provider().name(),
			counterparty.name()
		))
	}

	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		let mut state_machine_commitments = vec![];

		let number = self.arb_execution_client.get_block_number().await?;
		let block = self.arb_execution_client.get_block(number).await?.ok_or_else(|| {
			anyhow!("Didn't find block with number {number} on {:?}", self.evm.state_machine)
		})?;

		let state_machine_id = StateMachineId {
			state_id: self.state_machine,
			consensus_state_id: self.consensus_state_id.clone(),
		};

		let initial_consensus_state = ConsensusState {
			finalized_height: number.as_u64(),
			state_machine_id,
			l1_state_machine_id: StateMachineId {
				state_id: self.l1_state_machine,
				consensus_state_id: self.l1_consensus_state_id,
			},
			state_root: block.state_root.0.into(),
			arbitrum_consensus_type: ArbitrumConsensusType::ArbitrumOrbit,
		};

		state_machine_commitments.push((
			state_machine_id,
			StateCommitmentHeight {
				commitment: StateCommitment {
					timestamp: block.timestamp.as_u64(),
					overlay_root: None,
					state_root: block.state_root.0.into(),
				},
				height: number.as_u64(),
			},
		));

		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: ARBITRUM_CONSENSUS_CLIENT_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_periods: state_machine_commitments
				.iter()
				.map(|(state_machine, ..)| (state_machine.state_id, 5 * 60))
				.collect(),
			state_machine_commitments,
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}
