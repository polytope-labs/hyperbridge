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

use anyhow::{anyhow, Error};
use bsc_pos_verifier::primitives::{compute_epoch, EPOCH_LENGTH};
use codec::{Decode, Encode};
use futures::stream;
use ismp::messaging::{ConsensusMessage, CreateConsensusState};

use ethers::providers::Middleware;
use ismp_bsc_pos::ConsensusState;
use std::time::Duration;

use crate::{notification::consensus_notification, BscPosHost, KeccakHasher};
use tesseract_primitives::{BoxStream, IsmpHost, IsmpProvider};

#[async_trait::async_trait]
impl IsmpHost for BscPosHost {
	async fn consensus_notification<C>(
		&self,
		counterparty: C,
	) -> Result<BoxStream<ConsensusMessage>, anyhow::Error>
	where
		C: IsmpHost + IsmpProvider + 'static,
	{
		let client = BscPosHost::clone(&self);

		let interval = tokio::time::interval(Duration::from_secs(
			self.host.consensus_update_frequency.unwrap_or(300),
		));

		let stream = stream::unfold(interval, move |mut interval| {
			let client = client.clone();
			let counterparty = counterparty.clone();
			async move {
				let consensus_state = if let Ok(consensus_state) =
					counterparty.query_consensus_state(None, client.consensus_state_id).await
				{
					consensus_state
				} else {
					return Some((
						Err(anyhow!(
							"Not a fatal error: Error fetching consensus state for {:?} on {:?}",
							client.state_machine,
							counterparty.state_machine_id().state_id
						)),
						interval,
					))
				};
				let consensus_state = ConsensusState::decode(&mut &*consensus_state)
					.expect("Consensus state should always decode correctly");
				let current_epoch = compute_epoch(consensus_state.finalized_height);
				let attested_header = if let Ok(header) = client.prover.latest_header().await {
					header
				} else {
					return Some((
						Err(anyhow!(
							"Not a fatal error: Error fetching latest header for {:?}",
							client.state_machine
						)),
						interval,
					))
				};

				let attested_epoch = compute_epoch(attested_header.number.low_u64());
				// Try to sync the client first
				if attested_epoch > current_epoch {
					log::info!(
						"Syncing {:?} on {:?}",
						client.state_machine,
						counterparty.state_machine_id().state_id
					);
					let next_epoch = current_epoch + 1;

					let epoch_block_number = next_epoch * EPOCH_LENGTH;
					// Find a block that finalizes the epoch change block, Validators are rotated 12
					// blocks after the epoch boundary
					let mut block = epoch_block_number + 2;
					while block <= epoch_block_number + 11 {
						let header = if let Ok(header) = client.prover.fetch_header(block).await {
							header
						} else {
							return Some((
								Err(anyhow!(
									"Not a fatal error: Error fetching {:?} header for {block}",
									client.state_machine
								)),
								interval,
							))
						};
						let header = if let Some(header) = header {
							header
						} else {
							// If header does not exist yet wait before continuing
							tokio::time::sleep(Duration::from_secs(6)).await;
							continue
						};

						match client.prover.fetch_bsc_update::<KeccakHasher>(header).await {
							Ok(Some(update)) => {
								if update.source_header.number.low_u64() <=
									consensus_state.finalized_height
								{
									block += 1;
									continue
								}

								if !update.epoch_header_ancestry.is_empty() {
									return Some((
										Ok(ConsensusMessage {
											consensus_proof: update.encode(),
											consensus_state_id: client.consensus_state_id,
										}),
										interval,
									))
								}

								block += 1;
							},
							Ok(None) => {
								block += 1;
								continue
							},
							Err(_) =>
								return Some((
									Err(anyhow!(
										"Not a fatal error: Error fetching sync update for {:?}",
										client.state_machine
									)),
									interval,
								)),
						}
					}
				}

				interval.tick().await;

				let lambda = || async {
					let block_number = client.prover.client.get_block_number().await?;
					let block =
						client.prover.client.get_block(block_number).await?.ok_or_else(|| {
							anyhow!(
								"Block with number {block_number} not found for {:?}",
								client.state_machine
							)
						})?;

					let result = consensus_notification(&client, counterparty, block)
						.await?
						.map(|update| ConsensusMessage {
							consensus_proof: update.encode(),
							consensus_state_id: client.consensus_state_id,
						})
						.ok_or_else(|| anyhow!("Failed to fetch consensus proof"))?;

					Ok(result)
				};

				let result = lambda().await;

				Some((result, interval))
			}
		});

		Ok(Box::pin(stream))
	}

	async fn query_initial_consensus_state(&self) -> Result<Option<CreateConsensusState>, Error> {
		let initial_consensus_state =
			self.get_consensus_state::<KeccakHasher>(self.evm.ismp_host).await?;
		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: ismp_bsc_pos::BSC_CONSENSUS_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_period: 5 * 60,
			state_machine_commitments: vec![],
		}))
	}
}
