use std::{sync::Arc, time::Duration};

use crate::ArbHost;
use anyhow::{anyhow, Error};

use codec::{Decode, Encode};
use ethers::prelude::Middleware;
use futures::{stream, StreamExt};
use ismp::{
	consensus::{StateCommitment, StateMachineId},
	messaging::{ConsensusMessage, CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_arbitrum::{
	ArbitrumConsensusProof, ArbitrumConsensusType, ArbitrumUpdate, ConsensusState,
	ARBITRUM_CONSENSUS_CLIENT_ID,
};
use log::trace;
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

		let interval = tokio::time::interval(Duration::from_secs(
			self.host.consensus_update_frequency.unwrap_or(300),
		));

		let initial_height = counterparty.query_latest_height(l1_state_machine_id).await? as u64;
		trace!(target: "arb-host", "Latest height found for l1 state machine is {initial_height:?}");
		let latest_height = initial_height;

		let counterparty_clone = counterparty.clone();
		let interval_stream = stream::unfold((interval, latest_height), move |(mut interval, latest_height)| {
			let client = self.clone();
			let counterparty = counterparty_clone.clone();
			let consensus_state = consensus_state.clone();
			let state_machine = client.evm.state_machine();

			async move {
				interval.tick().await;
				let current_height =
					match counterparty.query_latest_height(l1_state_machine_id).await {
						Ok(height) => height,
						Err(_) =>
							return Some((Err(anyhow!("Not a fatal error: Error fetching l1 latest height")), (interval, latest_height),)),
					} as u64;
				trace!(target: "arb-host", "{state_machine:?} -> current height found for l1 state machine is {current_height:?}");

				let previous_height = latest_height;
				if current_height <= previous_height {
					trace!(target: "arb-host", "{state_machine:?} -> current height {current_height:?} <={previous_height:?}");
					return Some((Ok(None), (interval, previous_height)));
				}

				trace!(target: "arb-host", "{state_machine:?} ->  fetching events between {previous_height:?} and  {current_height:?}");
				return match consensus_state.arbitrum_consensus_type {
					ArbitrumConsensusType::ArbitrumOrbit => {
						match client.latest_event(previous_height + 1, current_height).await {
							Ok(Some(event)) => {
								trace!(target: "arb-host", "fetching arbitrum payload");
								match self.fetch_arbitrum_payload(current_height, event).await {
									Ok(payload) => {
										let update = ArbitrumUpdate {
											state_machine_id: StateMachineId {
												state_id: self.evm.state_machine,
												consensus_state_id: self.consensus_state_id,
											},
											l1_height: current_height,
											proof: ArbitrumConsensusProof::ArbitrumOrbit(payload),
										};

										let consensus_message = ConsensusMessage {
											consensus_proof: update.encode(),
											consensus_state_id: self.consensus_state_id,
											signer: counterparty.address(),
										};

										trace!(target: "arb-host", "Gotten update for {state_machine:?}");

										Some((Ok::<_, Error>(Some(consensus_message)), (interval, current_height)))
									}
									Err(e) => Some((Err(anyhow!("Not a fatal error: Error fetching arbitrum orbit payload with height {current_height:?}\n{e:?}")), (interval, latest_height),)),
								}
							}
							Ok(None) => {
								trace!(target: "arb-host", "{state_machine:?} ->  no event found between {previous_height} and {current_height}");
								Some((Ok::<_, Error>(None), (interval, current_height)))
							}
							Err(e) => {
								Some((
									Err(anyhow!(
                                "Not a fatal error: Failed to fetch latest arbitrum orbit event\n{e:?}",
                                )),
									(interval, latest_height),
								))
							}
						}
					}
					ArbitrumConsensusType::ArbitrumBold => {
						trace!(target: "arb-host", "{state_machine:?} ->  fetching bold event between {previous_height:?} is less than or equals {current_height:?}");
						match client.latest_assertion_event(previous_height + 1, current_height).await {
							Ok(Some(event)) => {
								trace!(target: "arb-host", "{state_machine:?}: fetching bold payload");
								match self.fetch_arbitrum_bold_payload(current_height, event).await {
									Ok(payload) => {
										let update = ArbitrumUpdate {
											state_machine_id: StateMachineId {
												state_id: self.evm.state_machine,
												consensus_state_id: self.consensus_state_id,
											},
											l1_height: current_height,
											proof: ArbitrumConsensusProof::ArbitrumBold(payload),
										};

										let consensus_message = ConsensusMessage {
											consensus_proof: update.encode(),
											consensus_state_id: self.consensus_state_id,
											signer: counterparty.address(),
										};

										trace!(target: "arb-host", "{state_machine:?} gotten bold update");

										Some((Ok::<_, Error>(Some(consensus_message)), (interval, current_height)))
									}
									Err(e) => Some((Err(anyhow!("Not a fatal error: Error fetching arbitrum bold payload\n{e:?}")), (interval, latest_height))),
								}
							}
							Ok(None) => {
								trace!(target: "arb-host", "{state_machine:?}: no events found between {previous_height}..{current_height}");
								Some((Ok::<_, Error>(None), (interval, current_height)))
							}
							Err(e) => {
								Some((
									Err(anyhow!(
                                "Not a fatal error: Failed to fetch latest arbitrum bold event for heights\n{e:?}"
                            )),
									(interval, latest_height),
								))
							}
						}
					}
				}
			}
		})
		.filter_map(|res| async move {
			match res {
				Ok(Some(update)) => Some(Ok(update)),
				Ok(None) => None,
				Err(err) => Some(Err(err)),
			}
		});

		let provider = self.provider();
		let mut stream = Box::pin(interval_stream);
		while let Some(item) = stream.next().await {
			match item {
				Ok(consensus_message) => {
					log::info!(
						target: "tesseract",
						"🛰️ Transmitting consensus message from {} to {}",
						provider.name(), counterparty.name()
					);
					let res = counterparty
						.submit(
							vec![Message::Consensus(consensus_message)],
							counterparty.state_machine_id().state_id,
						)
						.await;
					if let Err(err) = res {
						log::error!(
							"Failed to submit transaction to {}: {err:?}",
							counterparty.name()
						)
					}
				},
				Err(e) => {
					log::error!(target: "tesseract","Consensus task {}->{} encountered an error: {e:?}", provider.name(), counterparty.name())
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
			state_id: self.evm.state_machine,
			consensus_state_id: self.consensus_state_id.clone(),
		};

		let initial_consensus_state = ConsensusState {
			finalized_height: number.as_u64(),
			state_machine_id,
			l1_state_machine_id: StateMachineId {
				state_id: self.l1_state_machine,
				consensus_state_id: self.l1_consensus_state_id,
			},
			arbitrum_consensus_type: ArbitrumConsensusType::ArbitrumBold,
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
			// since there is no staking involved
			unbonding_period: u64::MAX,
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
