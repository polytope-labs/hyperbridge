// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use crate::{GetConsensusStateParams, L2Host, SyncCommitteeHost};
use codec::{Decode, Encode};
use ethers::prelude::Middleware;

use anyhow::{anyhow, Error};
use futures::{StreamExt, TryFutureExt};
use ismp::{
	consensus::{StateCommitment, StateMachineId},
	messaging::{ConsensusMessage, CreateConsensusState, Message, StateCommitmentHeight},
};
use ismp_sync_committee::types::{BeaconClientUpdate, ConsensusState};
use primitive_types::H160;
use std::{collections::BTreeMap, sync::Arc};
use sync_committee_primitives::{constants::Config, util::compute_sync_committee_period};

use crate::notification::consensus_notification;
use op_verifier::{CANNON, _PERMISSIONED};
use tesseract_primitives::{IsmpHost, IsmpProvider};

#[async_trait::async_trait]
impl<T: Config + Send + Sync + 'static, const ETH1_DATA_VOTES_BOUND: usize, const PROPOSER_LOOK_AHEAD_LIMIT: usize> IsmpHost
	for SyncCommitteeHost<T, ETH1_DATA_VOTES_BOUND, PROPOSER_LOOK_AHEAD_LIMIT>
{
	async fn start_consensus(&self, counterparty: Arc<dyn IsmpProvider>) -> Result<(), Error> {
		let client = SyncCommitteeHost::clone(&self);

		let interval = tokio::time::interval(self.consensus_update_frequency);
		let counterparty_clone = counterparty.clone();
		let interval_stream = futures::stream::unfold(interval, move |mut interval| {
			let client = client.clone();
			let counterparty = counterparty_clone.clone();

			async move {
				let sync = || async {
					let checkpoint =
						client.prover.fetch_finalized_checkpoint(Some("head")).await?.finalized;
					let consensus_state =
						counterparty.query_consensus_state(None, client.consensus_state_id).await?;
					let consensus_state = ConsensusState::decode(&mut &*consensus_state)?;
					let light_client_state = consensus_state.light_client_state;

					if checkpoint.epoch <= light_client_state.latest_finalized_epoch {
						return Ok(None);
					}
					// Signature period for this finalized epoch will be two epochs ahead
					let signature_period = compute_sync_committee_period::<T>(checkpoint.epoch + 2);
					// Do a sync check before returning any updates
					let state_period = light_client_state.state_period;
					if !(state_period..=(state_period + 1)).contains(&signature_period) {
						let next_period = state_period + 1;
						log::trace!(
							"Fetching sync update for sync committee period: {next_period}"
						);
						let update = client.prover.latest_update_for_period(next_period).await?;
						let message = BeaconClientUpdate { consensus_update: update };
						return Ok::<_, Error>(Some(message));
					}
					Ok(None)
				};

				match client
					.retry
					.retry(|| {
						sync().map_err(|err| {
							log::error!(
								"Error trying to fetch sync message for {:?}: {err:?}",
								client.state_machine
							);
							err
						})
					})
					.await
				{
					Ok(Some(beacon_message)) => {
						let update = ConsensusMessage {
							consensus_proof: beacon_message.encode(),
							consensus_state_id: client.consensus_state_id,
							signer: H160::random().0.to_vec(),
						};
						return Some((Ok::<_, Error>(Some(update)), interval));
					},
					Ok(None) => {},
					Err(err) =>
						return Some((
							Err::<_, Error>(err.context(format!(
								"Error trying to fetch sync message for {:?}",
								client.state_machine
							))),
							interval,
						)),
				};

				// tick the interval
				interval.tick().await;

				let checkpoint = match client
					.retry
					.retry(|| {
						client.prover.fetch_finalized_checkpoint(Some("head")).map_err(|err| {
							log::error!(
								"Failed to fetch latest finalized header for {:?}: {err:?}",
								client.state_machine
							);
							err
						})
					})
					.await
				{
					Ok(head) => head.finalized,
					Err(err) => {
						log::error!(
							"Failed to fetch latest finalized header for {:?}: {err:?}",
							client.state_machine
						);
						return Some((Ok::<_, Error>(None), interval));
					},
				};

				match client
					.retry
					.retry(|| {
						consensus_notification(&client, counterparty.clone(), checkpoint.clone())
							.map_err(|err| {
								log::error!(
									"Failed to fetch consensus proof for {:?}: {err:?}",
									client.state_machine
								);
								err
							})
					})
					.await
				{
					Ok(Some(beacon_message)) => {
						let update = ConsensusMessage {
							consensus_proof: beacon_message.encode(),
							consensus_state_id: client.consensus_state_id,
							signer: H160::random().0.to_vec(),
						};
						return Some((Ok::<_, Error>(Some(update)), interval));
					},
					Ok(None) => return Some((Ok::<_, Error>(None), interval)),
					Err(err) =>
						return Some((
							Err::<_, Error>(err.context(format!(
								"Failed to fetch consensus proof for {:?}",
								client.state_machine
							))),
							interval,
						)),
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

		let mut stream = Box::pin(interval_stream);

		let provider = self.provider();
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
			provider.name(),
			counterparty.name()
		))
	}

	async fn query_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
		let mut l2_oracle = BTreeMap::new();
		let mut dispute_factory_address = BTreeMap::new();
		let mut rollup_core_address = BTreeMap::new();
		let mut state_machine_commitments = vec![];

		for (state_machine, l2_host) in self.l2_clients.clone() {
			match l2_host {
				L2Host::ArbitrumOrbit(host) => {
					rollup_core_address.insert(host.evm.state_machine, host.host.rollup_core);
					let number = host.arb_execution_client.get_block_number().await?;
					let block =
						host.arb_execution_client.get_block(number).await?.ok_or_else(|| {
							anyhow!(
								"Didn't find block with number {number} on {:?}",
								host.evm.state_machine
							)
						})?;
					state_machine_commitments.push((
						StateMachineId {
							state_id: state_machine,
							consensus_state_id: self.consensus_state_id.clone(),
						},
						StateCommitmentHeight {
							commitment: StateCommitment {
								timestamp: block.timestamp.as_u64(),
								overlay_root: None,
								state_root: block.state_root.0.into(),
							},
							height: number.as_u64(),
						},
					));
				},
				L2Host::OpStack(host) => {
					if let Some((dispute_factory, respected_game_types)) = host
						.host
						.dispute_game_factory
						.map(|addr| (addr, vec![CANNON, _PERMISSIONED]))
					{
						dispute_factory_address.insert(
							host.evm.state_machine,
							(dispute_factory, respected_game_types),
						);
					}

					if let Some(l2_oracle_address) = host.host.l2_oracle {
						l2_oracle.insert(host.evm.state_machine, l2_oracle_address);
					}

					let number = host.op_execution_client.get_block_number().await?;
					let block =
						host.op_execution_client.get_block(number).await?.ok_or_else(|| {
							anyhow!(
								"Didn't find block with number {number} on {:?}",
								host.evm.state_machine
							)
						})?;
					state_machine_commitments.push((
						StateMachineId {
							state_id: state_machine,
							consensus_state_id: self.consensus_state_id.clone(),
						},
						StateCommitmentHeight {
							commitment: StateCommitment {
								timestamp: block.timestamp.as_u64(),
								overlay_root: None,
								state_root: block.state_root.0.into(),
							},
							height: number.as_u64(),
						},
					));
				},
			}
		}

		let params = GetConsensusStateParams {
			l2_oracle_address: l2_oracle,
			rollup_core_address,
			dispute_factory_address,
		};

		let initial_consensus_state = self.get_consensus_state(params, None).await?;

		let number = self.el.get_block_number().await?;
		let block = self.el.get_block(number).await?.ok_or_else(|| {
			anyhow!("Didn't find block with number {number} on {:?}", self.evm.state_machine)
		})?;
		state_machine_commitments.push((
			StateMachineId {
				state_id: self.state_machine,
				consensus_state_id: self.consensus_state_id.clone(),
			},
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
			consensus_client_id: T::ID,
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
