use std::sync::Arc;

use crate::ArbHost;
use anyhow::anyhow;

use futures::StreamExt;
use ismp::consensus::{StateCommitment, StateMachineId};
use ismp::messaging::{CreateConsensusState, StateCommitmentHeight};
use ismp_arbitrum::{ARBITRUM_CONSENSUS_CLIENT_ID, ConsensusState};
use tesseract_primitives::{IsmpHost, IsmpProvider};
use codec::{Decode, Encode};
use ethers::prelude::Middleware;

#[async_trait::async_trait]
impl IsmpHost for ArbHost {
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
		let mut state_machine_commitments = vec![];

		let number = self.arb_execution_client.get_block_number().await?;
		let block =
			self.arb_execution_client.get_block(number).await?.ok_or_else(|| {
				anyhow!(
								"Didn't find block with number {number} on {:?}",
								self.evm.state_machine
							)
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
