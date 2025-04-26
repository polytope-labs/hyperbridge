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
use bsc_verifier::{
	primitives::{compute_epoch, parse_extra, Config, VALIDATOR_BIT_SET_SIZE},
	verify_bsc_header,
};
use codec::{Decode, Encode};
use futures::{stream, StreamExt};
use ismp::messaging::{ConsensusMessage, CreateConsensusState, Message};

use bsc_prover::UpdateParams;
use ismp_bsc::ConsensusState;
use sp_core::H160;
use std::{cmp::max, sync::Arc, time::Duration};

use crate::{notification::consensus_notification, BscPosHost, KeccakHasher};
use bsc_prover::get_rotation_block;
use ssz_rs::{Bitvector, Deserialize};
use tesseract_primitives::{IsmpHost, IsmpProvider};

#[async_trait::async_trait]
impl<C: Config> IsmpHost for BscPosHost<C> {
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let client = BscPosHost::clone(&self);

		let interval = tokio::time::interval(Duration::from_secs(
			self.host.consensus_update_frequency.unwrap_or(300),
		));

		let epoch_length = self.host.epoch_length;

		let counterparty_clone = counterparty.clone();

		// We query consensus state at the finalized heights of the counterparty chain and we only
		// We only try to yield new consensus updates when the finalized height has changed.
		// This prevents the observed case where a transaction is banned because there was a reorg
		// and the task becomes stalled trying to resend the same consensus update.
		// This can happen because unsigned transactions don't have nonces, so the hyperbridge
		// txpool would not let us resubmit identical transactions.
		// If the stream encounters an error while processing we reset the consensus state
		// maintained as part of the stream's internal state so it would try to yield a new update
		// the next time its polled.
		let stream =
			stream::unfold((interval, None), move |(mut interval, prev_consensus_state)| {
				let client = client.clone();
				let counterparty = counterparty_clone.clone();
				async move {
					let counterparty_finalized = match counterparty.query_finalized_height().await {
						Ok(height) => height,
						Err(_) =>
							return Some((
								Err(anyhow!(
						"Not a fatal error: Error fetching consensus state for {:?} on {:?}",
						client.state_machine,
						counterparty.state_machine_id().state_id
					)),
								(interval, None),
							)),
					};
					let consensus_state = if let Ok(consensus_state) = counterparty
						.query_consensus_state(
							Some(counterparty_finalized),
							client.consensus_state_id,
						)
						.await
					{
						consensus_state
					} else {
						return Some((
							Err(anyhow!(
							"Not a fatal error: Error fetching consensus state for {:?} on {:?}",
							client.state_machine,
							counterparty.state_machine_id().state_id
						)),
							(interval, None),
						));
					};
					let consensus_state = ConsensusState::decode(&mut &*consensus_state)
						.expect("Consensus state should always decode correctly");
					// If the finalized consensus state has not changed since last time we polled
					// wait till the next poll
					if Some(consensus_state.clone()) == prev_consensus_state {
						return Some((Ok(None), (interval, prev_consensus_state)));
					}

					let current_epoch = max(
						compute_epoch(consensus_state.finalized_height, epoch_length),
						consensus_state.current_epoch,
					);

					let attested_header = if let Ok(header) = client.prover.latest_header().await {
						header
					} else {
						return Some((
							Err(anyhow!(
								"Not a fatal error: Error fetching latest header for {}",
								client.state_machine
							)),
							(interval, None),
						));
					};

					let attested_epoch =
						compute_epoch(attested_header.number.low_u64(), epoch_length);
					// Send a block that would enact authority set rotation
					if consensus_state.next_validators.is_some() {
						let rotation_block =
							consensus_state.next_validators.as_ref().expect("Valid").rotation_block;
						let enactment_epoch = compute_epoch(rotation_block, epoch_length);

						log::trace!(
							"Enacting Authority Set Rotation for {:?} on {}",
							client.state_machine,
							counterparty.state_machine_id().state_id
						);

						let epoch_block_number = enactment_epoch * epoch_length;
						let rotation_block = get_rotation_block(
							epoch_block_number,
							consensus_state.current_validators.len() as u64,
							epoch_length,
						);
						let mut block = rotation_block;
						// Authority set rotation is valid between rotation_block and
						// epoch_block+epoch_length
						while block <= (epoch_block_number + epoch_length - 1) {
							let header = if let Ok(header) = client.prover.fetch_header(block).await
							{
								header
							} else {
								return Some((
									Err(anyhow!(
										"Not a fatal error: Error fetching {} header for {block}",
										client.state_machine
									)),
									(interval, None),
								));
							};
							// If header does not exist we have to wait until it does
							let header = if let Some(header) = header {
								header
							} else {
								// If header does not exist yet wait before continuing
								tokio::time::sleep(Duration::from_secs(3)).await;
								continue;
							};

							match client
								.prover
								.fetch_bsc_update::<KeccakHasher>(UpdateParams {
									attested_header: header,
									validator_size: consensus_state.current_validators.len() as u64,
									epoch: enactment_epoch,
									epoch_length,
									fetch_val_set_change: false,
								})
								.await
							{
								Ok(Some(update)) => {
									// We try to validate before seubmiting to be sure enactment has
									// taken place, we know the minimum block at which rotation
									// should occur but sometimes It happens further in the future

									let next_validators =
										consensus_state.next_validators.clone().unwrap_or_default();

									let extra_data =
										parse_extra::<KeccakHasher, C>(&update.attested_header)
											.expect(
											"Infallible, was parsed before update was generated",
										);

									let validators_bit_set =
										Bitvector::<VALIDATOR_BIT_SET_SIZE>::deserialize(
											extra_data
												.vote_address_set
												.to_le_bytes()
												.to_vec()
												.as_slice(),
										)
										.expect(
											"Infallible, was parsed before update was generated",
										);

									if validators_bit_set.iter().as_bitslice().count_ones() <
										((2 * next_validators.validators.len() / 3) + 1)
									{
										log::trace!("Not enough participants in bsc update for block {block:?}");
										block += 1;
										continue;
									}
									let res = verify_bsc_header::<KeccakHasher, C>(
										&next_validators.validators,
										update.clone(),
										epoch_length,
									);
									if update.source_header.number.low_u64() <=
										consensus_state.finalized_height ||
										res.is_err()
									{
										block += 1;
										continue;
									}

									return Some((
										Ok(Some(ConsensusMessage {
											consensus_proof: update.encode(),
											consensus_state_id: client.consensus_state_id,
											signer: H160::random().0.to_vec(),
										})),
										(interval, Some(consensus_state)),
									));
								},
								Ok(None) => block += 1,
								Err(_) =>
									return Some((
										Err(anyhow!(
										"Not a fatal error: Error fetching authority enactment update for {}",
										client.state_machine
									)),
										(interval, None),
									)),
							}
						}
						log::trace!("No valid update found to enact authority set change");
						return Some((Ok(None), (interval, None)));
					}
					// Try to sync the client first
					if attested_epoch > current_epoch && consensus_state.next_validators.is_none() {
						log::info!(
							"Syncing {} on {}",
							client.state_machine,
							counterparty.state_machine_id().state_id
						);
						let next_epoch = current_epoch + 1;

						let epoch_block_number = next_epoch * epoch_length;
						// Find a block that finalizes the epoch change block, Validators are
						// rotated (validator_size / 2) blocks after the epoch boundary
						let mut block = epoch_block_number + 2;
						let rotation_block = get_rotation_block(
							epoch_block_number,
							consensus_state.current_validators.len() as u64,
							epoch_length,
						) - 1;

						while block <= rotation_block + epoch_length / 2 {
							let header = if let Ok(header) = client.prover.fetch_header(block).await
							{
								header
							} else {
								return Some((
									Err(anyhow!(
										"Not a fatal error: Error fetching {} header for {block}",
										client.state_machine
									)),
									(interval, None),
								));
							};
							let header = if let Some(header) = header {
								header
							} else {
								// If header does not exist yet wait before continuing
								tokio::time::sleep(Duration::from_secs(3)).await;
								continue;
							};

							match client
								.prover
								.fetch_bsc_update::<KeccakHasher>(UpdateParams {
									attested_header: header,
									validator_size: consensus_state.current_validators.len() as u64,
									epoch: next_epoch,
									epoch_length,
									fetch_val_set_change: true,
								})
								.await
							{
								Ok(Some(update)) => {
									// check number of participants
									let extra_data =
										parse_extra::<KeccakHasher, C>(&update.attested_header)
											.expect(
											"Infallible, was parsed before update was generated",
										);

									let validators_bit_set =
										Bitvector::<VALIDATOR_BIT_SET_SIZE>::deserialize(
											extra_data
												.vote_address_set
												.to_le_bytes()
												.to_vec()
												.as_slice(),
										)
										.expect(
											"Infallible, was parsed before update was generated",
										);

									if validators_bit_set.iter().as_bitslice().count_ones() <
										((2 * consensus_state.current_validators.len() / 3) + 1)
									{
										log::trace!("Not enough participants in bsc update for block {block:?}");
										block += 1;
										continue;
									}

									if update.source_header.number.low_u64() <=
										consensus_state.finalized_height
									{
										block += 1;
										continue;
									}

									if let Err(err) = verify_bsc_header::<KeccakHasher, C>(
										&consensus_state.current_validators,
										update.clone(),
										epoch_length,
									) {
										// If we are still looking for the next sync block and we
										// find an update not signed by the current validator set We
										// cannot rotate validator set safely
										return Some((
											Err(anyhow!(
												"Fatal error: No valid sync update found for  {}: {err:?}",
												client.state_machine
											)),
											(interval, None),
										));
									}

									// If we have an epoch ancestry or the source header that was
									// finalized is the epoch block
									if !update.epoch_header_ancestry.is_empty() ||
										update.source_header.number.low_u64() ==
											epoch_block_number
									{
										return Some((
											Ok(Some(ConsensusMessage {
												consensus_proof: update.encode(),
												consensus_state_id: client.consensus_state_id,
												signer: H160::random().0.to_vec(),
											})),
											(interval, Some(consensus_state)),
										));
									}

									block += 1;
								},
								Ok(None) => {
									block += 1;
									continue;
								},
								Err(err) =>
									return Some((
										Err(anyhow!(
										"Not a fatal error: Error fetching sync update for {} \n {err:?}",
										client.state_machine
									)),
										(interval, None),
									)),
							}
						}

						log::trace!("No valid sync update found for {next_epoch}");
						return Some((Ok(None), (interval, None)));
					}

					interval.tick().await;

					let mut consensus_state = None;

					let lambda = || async {
						let (update, cs_state) =
							consensus_notification(&client, counterparty).await?;
						consensus_state = cs_state;
						let result = update.map(|update| ConsensusMessage {
							consensus_proof: update.encode(),
							consensus_state_id: client.consensus_state_id,
							signer: H160::random().0.to_vec(),
						});

						Ok(result)
					};

					let result = lambda().await;
					// If the result is an error, we want to reset the consensus state maintained by
					// the stream so it can try to yield an update the next time its polled
					let ret = if result.is_err() { None } else { consensus_state };
					Some((result, (interval, ret)))
				}
			})
			.filter_map(|res| async move {
				match res {
					Ok(Some(update)) => Some(Ok(update)),
					Ok(None) => None,
					Err(err) => Some(Err(err)),
				}
			});

		let mut stream = Box::pin(stream);
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
		let initial_consensus_state = self.get_consensus_state::<KeccakHasher>().await?;
		Ok(Some(CreateConsensusState {
			consensus_state: initial_consensus_state.encode(),
			consensus_client_id: ismp_bsc::BSC_CONSENSUS_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_periods: vec![(self.state_machine, 5 * 60)].into_iter().collect(),
			state_machine_commitments: vec![],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		self.provider.clone()
	}
}
