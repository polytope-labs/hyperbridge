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

use anyhow::{anyhow, Context};
use codec::Decode;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use subxt::{
	config::{ExtrinsicParams, HashFor},
	tx::DefaultParams,
	utils::{AccountId32, MultiSignature, H256},
};

use beefy_verifier_primitives::ConsensusState;
use ismp::{
	consensus::ConsensusStateId,
	messaging::{CreateConsensusState, Message},
};
use ismp_solidity_abi::beefy::BeefyConsensusState;
use tesseract_primitives::{IsmpHost, IsmpProvider};
use tesseract_substrate::SubstrateClient;
use zk_beefy::BeefyProver as Sp1BeefyProverTrait;

use crate::{
	backend::{ConsensusProof, ProofBackend, QueueMessage, StreamMessage},
	prover::{query_parachain_header, Prover, ProverConsensusState},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyHostConfig {
	/// Consensus state id for the host on the counterparty
	pub consensus_state_id: ConsensusStateId,
	/// Optional Redis configuration for proof backend (if None, uses InMemoryProofBackend)
	pub redis: Option<crate::backend::RedisConfig>,
}

/// The beefy host is responsible for receiving BEEFY proofs from the queue and submitting
/// them to the counterparty.
pub struct BeefyHost<R, P, B, Q>
where
	R: subxt::Config,
	P: subxt::Config,
	B: Sp1BeefyProverTrait,
{
	/// Unified backend for receiving consensus proofs
	pub backend: Arc<Q>,
	/// Host configuration options
	config: BeefyHostConfig,
	/// Consensus prover
	prover: Prover<R, P, B>,
	/// The underlying substrate client
	pub client: SubstrateClient<P>,
}

impl<R, P, B, Q> BeefyHost<R, P, B, Q>
where
	R: subxt::Config,
	P: subxt::Config,
	B: Sp1BeefyProverTrait,
	Q: ProofBackend,
	P: subxt::Config + Send + Sync + Clone,
	<P::ExtrinsicParams as ExtrinsicParams<P>>::Params: Send + Sync + DefaultParams,
	P::Signature: From<MultiSignature> + Send + Sync,
	P::AccountId: From<AccountId32> + Into<P::Address> + Clone + 'static + Send + Sync,
	H256: From<HashFor<P>>,
{
	/// Construct an implementation of the [`BeefyHost`]
	pub async fn new(
		config: BeefyHostConfig,
		prover: Prover<R, P, B>,
		client: SubstrateClient<P>,
		backend: Arc<Q>,
	) -> Result<Self, anyhow::Error> {
		Ok(BeefyHost { backend, prover, client, config })
	}

	/// Initialize the consensus state for the prover (used by the state storage backend), then
	/// returns it.
	pub async fn hydrate_initial_consensus_state(
		&self,
		state: ConsensusState,
	) -> Result<(), anyhow::Error> {
		let prover_consensus_state = {
			let inner = self.prover.inner();
			let hash = inner
				.relay_rpc
				.chain_get_block_hash(Some(state.latest_beefy_height.into()))
				.await?
				.ok_or_else(|| {
					anyhow!("Failed to find block hash for num: {}", state.latest_beefy_height)
				})?;
			let para_header =
				query_parachain_header(&inner.relay_rpc, hash, inner.para_ids[0]).await?;

			ProverConsensusState {
				inner: state,
				finalized_parachain_height: para_header.number.into(),
			}
		};

		// Save the consensus state using the backend
		self.backend.save_state(&prover_consensus_state).await?;

		Ok(())
	}
}

#[async_trait::async_trait]
impl<R, P, B, Q> IsmpHost for BeefyHost<R, P, B, Q>
where
	R: subxt::Config + Send + Sync + Clone,
	P: subxt::Config + Send + Sync + Clone,
	B: Sp1BeefyProverTrait,
	Q: ProofBackend,
	<P::ExtrinsicParams as ExtrinsicParams<P>>::Params: Send + Sync + DefaultParams,
	P::Signature: From<MultiSignature> + Send + Sync,
	P::AccountId: From<AccountId32> + Into<P::Address> + Clone + 'static + Send + Sync,
	H256: From<HashFor<P>>,
{
	async fn start_consensus(
		&self,
		counterparty: Arc<dyn IsmpProvider>,
	) -> Result<(), anyhow::Error> {
		let counterparty_state_machine = counterparty.state_machine_id().state_id;
		let mut notifications =
			self.backend.queue_notifications(counterparty_state_machine).await?;

		// this will yield whenever the prover writes to either the mandatory or messages queue
		while let Some(item) = notifications.next().await {
			let Ok(ref message) = item else {
				let error = item.unwrap_err();
				tracing::error!("Error in queue notification stream: {:?}", error);

				// Check if it's a Redis connection error and attempt reconnection
				if let Some(redis_error) = error.downcast_ref::<redis_async::error::Error>() {
					if matches!(
						redis_error,
						redis_async::error::Error::Connection(_) | redis_async::error::Error::IO(_)
					) {
						tracing::info!("Redis connection error detected, attempting to reconnect");
						if let Err(e) = self.backend.reconnect_notifier().await {
							tracing::error!(
								"Failed to recreate notification subscription: {:?}",
								e
							);
						} else {
							tracing::info!("Successfully recreated notification subscription");
						}
					}
				}

				// Try to recreate the notification stream
				notifications =
					match self.backend.queue_notifications(counterparty_state_machine).await {
						Ok(n) => n,
						Err(e) => {
							tracing::error!("Failed to recreate notification stream: {:?}", e);
							tokio::time::sleep(Duration::from_secs(5)).await;
							continue;
						},
					};
				continue;
			};

			if *message == StreamMessage::EpochChanged {
				// try to consume all mandatory updates
				loop {
					let item =
						self.backend.receive_mandatory_proof(&counterparty_state_machine).await;

					let QueueMessage { id, proof: ConsensusProof { message, set_id, .. } } =
						match item {
							Ok(Some(message)) => message,
							// no new items in the queue, continue to process messages queue
							Ok(None) => break,
							Err(err) => {
								tracing::error!(
									"{counterparty_state_machine} error pulling from mandatory queue: {err:?}"
								);
								// non-fatal error, keep trying
								continue;
							},
						};

					tracing::info!("{counterparty_state_machine} got authority set handover proof for {set_id}");
					let encoded = counterparty
						.query_consensus_state(None, self.config.consensus_state_id)
						.await
						.context("Could not fetch consenus state")?; // somewhat fatal
					let consensus_state = ConsensusState::decode(&mut &encoded[..])
						.expect("Infallible, consensus state was encoded correctly");

					// just some sanity checks
					if set_id < consensus_state.next_authorities.id {
						tracing::error!(
							"{counterparty_state_machine} got proof with set_id: {set_id} < next_set_id:{}",
							consensus_state.next_authorities.id
						);
						self.backend
							.delete_message(
								&counterparty_state_machine,
								&id,
								StreamMessage::EpochChanged,
							)
							.await?; // this would be a fatal error
						continue;
					}

					// just some sanity checks
					if set_id != consensus_state.next_authorities.id {
						tracing::error!(
							"{counterparty_state_machine} consensus proof with set_id: {set_id} does not match next_set_id: {}",
							consensus_state.next_authorities.id
						);
						// try to pull something else
						continue;
					}

					if let Err(err) = counterparty
						.submit(
							vec![Message::Consensus(message)],
							self.client.state_machine_id().state_id,
						)
						.await
					{
						tracing::error!(
							"Error submitting consensus message to {counterparty_state_machine}: {err:?}",
						);

						// non-fatal error, keep trying. This will pull it from the queue once more
						continue;
					};

					tracing::info!(
						"Submitted mandatory proof to {counterparty_state_machine} for {set_id}"
					);
					// this would be a fatal error
					self.backend
						.delete_message(
							&counterparty_state_machine,
							&id,
							StreamMessage::EpochChanged,
						)
						.await?;
				}
			}

			// must be for a new message, try to consume all updates.
			loop {
				let item = self.backend.receive_messages_proof(&counterparty_state_machine).await;

				let QueueMessage {
					id,
					proof: ConsensusProof { message, finalized_height, set_id },
				} = match item {
					Ok(Some(message)) => message,
					Ok(None) => break, // no new items in the queue
					Err(err) => {
						tracing::error!("{counterparty_state_machine} error pulling from messages queue: {err:?}");
						// non-fatal error, keep trying
						continue;
					},
				};

				tracing::info!(
					"{counterparty_state_machine} got messages proof for {finalized_height}"
				);
				let encoded = counterparty
					.query_consensus_state(None, self.config.consensus_state_id)
					.await?; // somewhat fatal
				let consensus_state = ConsensusState::decode(&mut &encoded[..])
					.expect("Infallible, consensus state was encoded correctly");

				// check if the update is relevant to us.
				if consensus_state.latest_beefy_height >= finalized_height {
					tracing::info!(
						"{counterparty_state_machine} saw proof for stale height {finalized_height}, current: {}",
						consensus_state.latest_beefy_height
					);
					// delete the message and pull another one
					self.backend
						.delete_message(
							&counterparty_state_machine,
							&id,
							StreamMessage::NewMessages,
						)
						.await?; // this would be a fatal error
					continue;
				}

				if set_id != consensus_state.current_authorities.id &&
					set_id != consensus_state.next_authorities.id
				{
					tracing::info!(
						"{counterparty_state_machine} saw proof for unknown set_id {set_id}, current: {}, next: {}",
						consensus_state.current_authorities.id,
						consensus_state.next_authorities.id,
					);

					if set_id > consensus_state.next_authorities.id {
						tracing::info!(
							"{counterparty_state_machine} proof was for future set: {set_id}, next: {}",
							consensus_state.next_authorities.id,
						);
						// break so that we can process a mandatory update
						break;
					} else if set_id < consensus_state.current_authorities.id {
						tracing::info!(
							"{counterparty_state_machine} proof was for older set: {set_id}, current: {}",
							consensus_state.current_authorities.id,
						);
						self.backend
							.delete_message(
								&counterparty_state_machine,
								&id,
								StreamMessage::NewMessages,
							)
							.await?; // this would be a fatal error
						continue; // move on to the next proof
					}
				}

				if let Err(err) = counterparty
					.submit(
						vec![Message::Consensus(message)],
						self.client.state_machine_id().state_id,
					)
					.await
				{
					tracing::error!(
						"Error submitting consensus message to {counterparty_state_machine}: {err:?}",
					);
					// non-fatal error, keep trying. This will pull it from the queue once more
					continue;
				};

				self.backend
					.delete_message(&counterparty_state_machine, &id, StreamMessage::NewMessages)
					.await?;
				// this would be a fatal error
			}
		}

		Ok(())
	}

	/// Queries the consensus state at the latest height
	async fn query_initial_consensus_state(
		&self,
	) -> Result<Option<CreateConsensusState>, anyhow::Error> {
		use alloy_sol_types::SolValue;
		let consensus_state: BeefyConsensusState =
			self.prover.query_initial_consensus_state(None).await?.inner.into();

		Ok(Some(CreateConsensusState {
			consensus_state: consensus_state.abi_encode(),
			consensus_client_id: *b"BEEF",
			consensus_state_id: self.config.consensus_state_id,
			unbonding_period: 60 * 60 * 60 * 27,
			challenge_periods: vec![(self.client.state_machine_id().state_id, 5 * 60)]
				.into_iter()
				.collect(),
			state_machine_commitments: vec![],
		}))
	}

	fn provider(&self) -> Arc<dyn IsmpProvider> {
		Arc::new(self.client.clone())
	}
}
