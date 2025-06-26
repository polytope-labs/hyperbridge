use std::{sync::Arc, time::Duration};

use crate::ArbHost;
use anyhow::{anyhow, Error};

use codec::{Decode, Encode};
use ethers::prelude::Middleware;
use futures::{stream, StreamExt};
use log::trace;
use ismp::{
	consensus::{StateCommitment, StateMachineId},
	messaging::{ConsensusMessage, CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_arbitrum::{
	ArbitrumConsensusProof, ArbitrumConsensusType, ArbitrumUpdate, ConsensusState,
	ARBITRUM_CONSENSUS_CLIENT_ID,
};
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tokio::sync::Mutex;

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
		let latest_height = Arc::new(Mutex::new(initial_height));
		let latest_height_for_stream = latest_height.clone();

		let counterparty_clone = counterparty.clone();
		let interval_stream = stream::unfold(interval, move |mut interval| {
			let client = self.clone();
			let counterparty = counterparty_clone.clone();
			let consensus_state = consensus_state.clone();
			let latest_height= latest_height_for_stream.clone();

			async move {
				interval.tick().await;
				let current_height =
					match counterparty.query_latest_height(l1_state_machine_id).await {
						Ok(height) => height,
						Err(_) =>
							return Some((Err(anyhow!("Not a fatal error: Error fetching l1 latest height for {:?} on {:?}",
								client.state_machine,counterparty.state_machine_id().state_id)), interval,)),
					} as u64;
				trace!(target: "arb-host", "current height found for l1 state machine is {current_height:?}");

				let previous_height = *latest_height.lock().await;
				if current_height <= previous_height {
					trace!(target: "arb-host", "current height {current_height:?} is less than or equals {previous_height:?}");
					return Some((Ok(None), interval));
				}

				trace!(target: "arb-host", "consensus state type is {:?}", consensus_state.arbitrum_consensus_type.clone());
				trace!(target: "arb-host", "orbit fetching event between {previous_height:?} and  {current_height:?}");
				return match consensus_state.arbitrum_consensus_type {
					ArbitrumConsensusType::ArbitrumOrbit => {
						match client.latest_event(previous_height + 1, current_height).await {
							Ok(Some(event)) => {
								trace!(target: "arb-host", "fetching arbitrum payload");
								match self.fetch_arbitrum_payload(current_height, event).await {
									Ok(payload) => {
										let update = ArbitrumUpdate {
											state_machine_id: StateMachineId {
												state_id: self.state_machine,
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

										trace!(target: "arb-host", "gotten updates for arbitrum");

										Some((Ok::<_, Error>(Some((Some(consensus_message), current_height))), interval))
									}
									Err(_) => Some((Err(anyhow!("Not a fatal error: Error fetching arbitrum orbit payload with height {:?} on {:?} {:?}",
								current_height, client.state_machine,counterparty.state_machine_id().state_id)), interval,)),
								}
							}
							Ok(None) => {
								trace!(target: "arb-host", "no event is being returned for arb orbit");
								Some((Ok::<_, Error>(Some((None, current_height))), interval))
							}
							Err(_) => {
								Some((
									Err(anyhow!(
                                "Not a fatal error: Failed to fetch latest arbitrum orbit event for heights {:?} and {:?}, for {:?} on {:?}",
                                latest_height,
                                current_height,
										client.state_machine, counterparty.state_machine_id().state_id
                            )),
									interval,
								))
							}
						}
					}
					ArbitrumConsensusType::ArbitrumBold => {
						trace!(target: "arb-host", "bold fetching event between {previous_height:?} is less than or equals {current_height:?}");
						match client.latest_assertion_event(previous_height + 1, current_height).await {
							Ok(Some(event)) => {
								trace!(target: "arb-host", "fetching bold payload");
								match self.fetch_arbitrum_bold_payload(current_height, event).await {
									Ok(payload) => {
										let update = ArbitrumUpdate {
											state_machine_id: StateMachineId {
												state_id: self.state_machine,
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

										trace!(target: "arb-host", "gotten bold update");

										Some((Ok::<_, Error>(Some((Some(consensus_message), current_height))), interval))
									}
									Err(_) => Some((Err(anyhow!("Not a fatal error: Error fetching arbitrum bold payload with height {:?} on {:?} {:?}",
								current_height, client.state_machine,counterparty.state_machine_id().state_id)), interval,)),
								}
							}
							Ok(None) => {
								trace!(target: "arb-host", "no events is being returned for arb bold");
								Some((Ok::<_, Error>(Some((None, current_height))), interval))
							}
							Err(_) => {
								Some((
									Err(anyhow!(
                                "Not a fatal error: Failed to fetch latest arbitrum bold event for heights {:?} and {:?}, for {:?} on {:?}",
                                latest_height,
                                current_height,
										client.state_machine, counterparty.state_machine_id().state_id
                            )),
									interval,
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
				Ok((Some(consensus_message), height)) => {
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
						trace!(target: "arb-host", "error submitting arbitrum update");
						log::error!(
							"Failed to submit transaction to {}: {err:?}",
							counterparty.name()
						)
					} else {
						trace!(target: "arb-host", "advancing current height");
						let mut current_height = latest_height.lock().await;
						*current_height = height;
					}
				},
				Ok((None, height)) => {
					trace!(target: "arb-host", "advancing current height with no consensus message found");
					let mut current_height = latest_height.lock().await;
					*current_height = height;

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
