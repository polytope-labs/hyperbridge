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

use anyhow::{anyhow, Error};
use codec::Encode;
use futures::{stream, StreamExt};
use polkadot_sdk::sp_runtime::traits::{One, Zero};
use subxt::{
	config::{ExtrinsicParams, HashFor, Header},
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature, H256},
};

use grandpa_verifier_primitives::ConsensusState;
use ismp::{
	host::StateMachine,
	messaging::{ConsensusMessage, CreateConsensusState, Message},
};
use ismp_grandpa::{
	consensus::GRANDPA_CONSENSUS_ID,
	messages::{RelayChainMessage, StandaloneChainMessage},
};
use tesseract_primitives::{IsmpHost, IsmpProvider};

use crate::GrandpaHost;

#[async_trait::async_trait]
impl<H, C> IsmpHost for GrandpaHost<H, C>
where
	H: subxt::Config + Send + Sync + Clone,
	C: subxt::Config + Send + Sync + Clone,
	<H::Header as Header>::Number: Ord + Zero + finality_grandpa::BlockNumberOps + One + From<u32>,
	u32: From<<H::Header as Header>::Number>,
	H256: From<HashFor<H>>,
	H::Header: codec::Decode,
	<H::Hasher as subxt::config::Hasher>::Output: From<HashFor<H>>,
	HashFor<H>: From<<H::Hasher as subxt::config::Hasher>::Output>,
	HashFor<H>: From<H256>,
	<H::ExtrinsicParams as ExtrinsicParams<H>>::Params: Send + Sync + DefaultParams,
	H::Signature: From<MultiSignature> + Send + Sync,
	H::AccountId: From<AccountId32> + Into<H::Address> + Clone + 'static + Send + Sync,

	<C::ExtrinsicParams as ExtrinsicParams<C>>::Params: Send + Sync + DefaultParams,
	C::Signature: From<MultiSignature> + Send + Sync,
	C::AccountId: From<AccountId32> + Into<C::Address> + Clone + 'static + Send + Sync,
	H256: From<HashFor<C>>,
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

					let consensus_state: ConsensusState =
						codec::Decode::decode(&mut &consensus_state_bytes[..])?;
					log::trace!(
						"Consensus state: {:#?}",
						ConsensusState { current_authorities: vec![], ..consensus_state }
					);
					client.should_sync(consensus_state.current_set_id).await
				};

				match sync().await {
					Ok(val) => {
						// If the consensus client on counterparty needs to be synced, then we
						// should not observe the interval
						if !val {
							interval.tick().await;
						}
					},
					Err(e) =>
						return Some((
							Err(anyhow!("Error while checking sync status of client {e:?}")),
							interval,
						)),
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

								log::trace!(
									"Consensus state for {}: {:#?}",
									client.state_machine,
									ConsensusState {
										current_authorities: vec![],
										..consensus_state
									}
								);

								let finalized_hash =
									client.prover.rpc.chain_get_finalized_head().await?;
								let latest_finalized_head: u64 = client
									.prover
									.rpc
									.chain_get_header(Some(finalized_hash))
									.await?
									.ok_or_else(|| anyhow!("Failed to fetch finalized head"))?
									.number()
									.into();

								if latest_finalized_head <= consensus_state.latest_height.into() {
									return Ok(None);
								}

								// Query finality proof will give us the highest finality proof in
								// the epoch of the block number we supplied
								let finality_proof = client
									.prover
									.query_finality_proof(consensus_state.clone())
									.await?;

								let parachain_headers_with_proof = client
									.prover
									.query_finalized_parachain_headers_with_proof(
										finality_proof.block.into(),
									)
									.await?;

								let relay_chain_message = RelayChainMessage {
									finality_proof: codec::Decode::decode(
										&mut &finality_proof.encode()[..],
									)?,
									parachain_headers: parachain_headers_with_proof,
								};
								let message = ConsensusMessage {
									consensus_proof:
										ismp_grandpa::messages::ConsensusMessage::Polkadot(
											relay_chain_message,
										)
										.encode(),
									consensus_state_id: client.consensus_state_id.clone(),
									signer: H256::random().0.to_vec(),
								};

								Ok::<_, Error>(Some(message))
							},

							StateMachine::Relay { .. } => {
								let consensus_state_bytes = counterparty
									.query_consensus_state(None, client.consensus_state_id.clone())
									.await?;

								let consensus_state: ConsensusState =
									codec::Decode::decode(&mut &consensus_state_bytes[..])?;

								log::trace!(
									"Consensus state for {}: {:#?}",
									client.state_machine,
									ConsensusState {
										current_authorities: vec![],
										..consensus_state
									}
								);

								let finalized_hash =
									client.prover.rpc.chain_get_finalized_head().await?;
								let latest_finalized_head: u64 = client
									.prover
									.rpc
									.chain_get_header(Some(finalized_hash))
									.await?
									.ok_or_else(|| anyhow!("Failed to fetch finalized head"))?
									.number()
									.into();

								if latest_finalized_head <= consensus_state.latest_height.into() {
									return Ok(None);
								}

								// Query finality proof will give us the highest finality proof in
								// the epoch of the block number we supplied
								let finality_proof = client
									.prover
									.query_finality_proof(consensus_state.clone())
									.await?;

								let parachain_headers_with_proof = client
									.prover
									.query_finalized_parachain_headers_with_proof(
										finality_proof.block.into(),
									)
									.await?;

								let relay_chain_message = RelayChainMessage {
									finality_proof: codec::Decode::decode(
										&mut &finality_proof.encode()[..],
									)?,
									parachain_headers: parachain_headers_with_proof,
								};
								let message = ConsensusMessage {
									consensus_proof:
										ismp_grandpa::messages::ConsensusMessage::Relaychain(
											relay_chain_message,
										)
										.encode(),
									consensus_state_id: client.consensus_state_id.clone(),
									signer: H256::random().0.to_vec(),
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

								log::trace!(
									"Consensus state for {}: {:#?}",
									client.state_machine,
									ConsensusState {
										current_authorities: vec![],
										..consensus_state
									}
								);

								let finalized_hash =
									client.prover.rpc.chain_get_finalized_head().await?;
								let latest_finalized_head: u64 = client
									.prover
									.rpc
									.chain_get_header(Some(finalized_hash))
									.await?
									.ok_or_else(|| anyhow!("Failed to fetch finalized head"))?
									.number()
									.into();

								// We ensure there's a new finalized block before trying to query a
								// finality proof
								if latest_finalized_head <= consensus_state.latest_height.into() {
									return Ok(None);
								}

								let finality_proof =
									client.prover.query_finality_proof(consensus_state).await?;
								let standalone_message = StandaloneChainMessage {
									finality_proof: codec::Decode::decode(
										&mut &finality_proof.encode()[..],
									)?,
								};
								let message = ConsensusMessage {
									consensus_proof:
										ismp_grandpa::messages::ConsensusMessage::StandaloneChain(
											standalone_message,
										)
										.encode(),
									consensus_state_id: client.consensus_state_id,
									signer: H256::random().0.to_vec(),
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
		let finalized_hash = self.prover.rpc.chain_get_finalized_head().await?;
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
