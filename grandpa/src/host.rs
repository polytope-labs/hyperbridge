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

use std::{sync::Arc, time::Duration};

use crate::GrandpaHost;
use anyhow::{anyhow, Error};
use codec::{Decode, Encode};
use futures::{stream, StreamExt};
use grandpa_verifier_primitives::ConsensusState;
use ismp::{
	host::StateMachine,
	messaging::{ConsensusMessage, CreateConsensusState, Message},
};
use ismp_grandpa::{
	consensus::GRANDPA_CONSENSUS_ID,
	messages::{RelayChainMessage, StandaloneChainMessage},
};

use grandpa_verifier_primitives::justification::GrandpaJustification;
use sp_core::{crypto, H256};
use subxt::config::{
	extrinsic_params::BaseExtrinsicParamsBuilder, polkadot::PlainTip, ExtrinsicParams, Header,
};

use subxt::{
	config::substrate::{BlakeTwo256, SubstrateHeader},
	ext::sp_runtime::{
		traits::{One, Zero},
		MultiSignature,
	},
};
use tesseract_primitives::{IsmpHost, IsmpProvider};

pub type Justification = GrandpaJustification<polkadot_core_primitives::Header>;

#[async_trait::async_trait]
impl<H, C> IsmpHost for GrandpaHost<H, C>
where
	H: subxt::Config + Send + Sync + Clone,
	C: subxt::Config + Send + Sync + Clone,
	<H::Header as Header>::Number: Ord + Zero + finality_grandpa::BlockNumberOps + One,
	u32: From<<H::Header as Header>::Number>,
	sp_core::H256: From<H::Hash>,
	H::Header: codec::Decode,
	<H::Hasher as subxt::config::Hasher>::Output: From<H::Hash>,
	H::Hash: From<<H::Hasher as subxt::config::Hasher>::Output>,
	<H as subxt::Config>::Hash: From<sp_core::H256>,
	<H::ExtrinsicParams as ExtrinsicParams<H::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<H, PlainTip>>,
	H::Signature: From<MultiSignature> + Send + Sync,
	H::AccountId: From<crypto::AccountId32> + Into<H::Address> + Clone + 'static + Send + Sync,

	<C::ExtrinsicParams as ExtrinsicParams<C::Hash>>::OtherParams:
		Default + Send + Sync + From<BaseExtrinsicParamsBuilder<C, PlainTip>>,
	C::Signature: From<MultiSignature> + Send + Sync,
	C::AccountId: From<crypto::AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
	H256: From<<C as subxt::Config>::Hash>,
{
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let client = GrandpaHost::clone(&self);

		let interval = tokio::time::interval(Duration::from_secs(
			self.config.grandpa.consensus_update_frequency.unwrap_or(300),
		));

		let counterparty_clone = counterparty.clone();

		let interval_stream = stream::unfold(interval, move |mut interval| {
			let client = client.clone();
			let counterparty = counterparty_clone.clone();
			async move {
                let sync = || async {
                    let consensus_state_bytes = counterparty
						.query_consensus_state(None, client.consensus_state_id.clone())
						.await?;

					let consensus_state: ConsensusState =codec::Decode::decode(&mut &consensus_state_bytes[..])?;
                    client.should_sync(consensus_state.current_set_id).await
                };

                match sync().await {
                    Ok(val) => {
                        // If the consensus client on counterparty needs to be synced, then we should not observe the interval
                        if !val {
                            interval.tick().await;
                        }
                    }
                    Err(e) => {
                        return Some((Err(anyhow!("Error while checking sync status of client {e:?}")), interval))
                    }
                }

				let lambda = || {
					async {
						match client.state_machine {
							StateMachine::Polkadot(_) | StateMachine::Kusama(_) => {
								let consensus_state_bytes = counterparty
									.query_consensus_state(None, client.consensus_state_id.clone())
									.await?;

								let consensus_state: ConsensusState =
									codec::Decode::decode(&mut &consensus_state_bytes[..])?;

                                let finalized_hash = client.prover.client.rpc().finalized_head().await?;
                                let latest_finalized_head: u64 = client.prover.client.rpc().header(Some(finalized_hash)).await?.ok_or_else(|| anyhow!("Failed to fetch finalized head"))?.number().into();

                                if latest_finalized_head <= consensus_state.latest_height.into() {
                                    return Ok(None)
                                }

                                // Query finality proof will give us the highest finality proof in the epoch of the block number we supplied
								let next_relay_height = consensus_state.latest_height + 1;

								let finality_proof = client
									.prover
									.query_finality_proof::<SubstrateHeader<u32, BlakeTwo256>>(
										consensus_state.latest_height,
										next_relay_height,
									)
									.await?;

								let justification =
									Justification::decode(&mut &finality_proof.justification[..])?;

								let parachain_headers_with_proof = client
									.prover
									.query_finalized_parachain_headers_with_proof::<SubstrateHeader<u32, BlakeTwo256>>(
										justification.commit.target_number,
										finality_proof.clone(),
									)
									.await?;

								let relay_chain_message = RelayChainMessage {
									finality_proof: codec::Decode::decode(
										&mut &parachain_headers_with_proof.finality_proof.encode()
											[..],
									)?,
									parachain_headers: parachain_headers_with_proof
										.parachain_headers,
								};
								let message = ConsensusMessage {
									consensus_proof:
										ismp_grandpa::messages::ConsensusMessage::RelayChainMessage(
											relay_chain_message,
										)
										.encode(),
									consensus_state_id: client.consensus_state_id.clone(),
									signer: counterparty.address(),
								};

								Ok::<_, Error>(Some(message))
							},
							StateMachine::Substrate(_) => {
								// Query finality proof
								let consensus_state_bytes = counterparty
									.query_consensus_state(None, client.consensus_state_id)
									.await?;

								let consensus_state: ConsensusState =
									codec::Decode::decode(&mut &consensus_state_bytes[..])?;

                                let finalized_hash = client.prover.client.rpc().finalized_head().await?;
                                let latest_finalized_head: u64 = client.prover.client.rpc().header(Some(finalized_hash)).await?.ok_or_else(|| anyhow!("Failed to fetch finalized head"))?.number().into();

                                // We ensure there's a new finalized block before trying to query a finality proof
                                if latest_finalized_head <= consensus_state.latest_height.into() {
                                    return Ok(None)
                                }

                                // Query finality proof will give us the highest finality proof in the epoch of the block number we supplied
								let next_relay_height = consensus_state.latest_height + 1;

								let finality_proof = client
									.prover
									.query_finality_proof::<SubstrateHeader<u32, BlakeTwo256>>(
										consensus_state.latest_height,
										next_relay_height,
									)
									.await?;
								let standalone_message = StandaloneChainMessage {
									finality_proof: codec::Decode::decode(
										&mut &finality_proof.encode()[..],
									)?,
								};
								let message = ConsensusMessage {
                                    consensus_proof:
                                        ismp_grandpa::messages::ConsensusMessage::StandaloneChainMessage(
                                            standalone_message,
                                        )
                                        .encode(),
                                    consensus_state_id: client.consensus_state_id,
                                    signer: counterparty.address()
                                };

								Ok(Some(message))
							},
							_ => Err(anyhow!("Unsupported state machine")),
						}
					}
				};

				match lambda().await {
					Ok(message) => Some((Ok(message), interval)),
					Err(err) => Some((Err(err), interval)),
				}
			}
		}).filter_map(|res| async move {
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
						"🛰️ Transmitting consensus proof of size {} from {} to {}",
						human_bytes::human_bytes(consensus_message.consensus_proof.len() as u32),
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

	/// Queries the consensus state at the latest height
	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		let finalized_hash = self.prover.client.rpc().finalized_head().await?;
		let consensus_state: ConsensusState = self
			.prover
			.initialize_consensus_state(self.config.grandpa.slot_duration, finalized_hash)
			.await?;

		Ok(Some(CreateConsensusState {
			consensus_state: consensus_state.encode(),
			consensus_client_id: GRANDPA_CONSENSUS_ID,
			consensus_state_id: self.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_periods: vec![(self.state_machine, 5 * 60)].into_iter().collect(),
			state_machine_commitments: vec![],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		Arc::new(self.substrate_client.clone())
	}
}
