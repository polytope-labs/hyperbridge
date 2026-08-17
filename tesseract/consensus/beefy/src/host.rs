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
	host::StateMachine,
	messaging::{CreateConsensusState, Message},
};
use ismp_abi::ecdsa_beefy::BeefyConsensusState;
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
}

/// What the host needs out of a destination's consensus state, independent of its shape.
struct StateSummary {
	latest_beefy_height: u32,
	current_set_id: u64,
	next_set_id: u64,
}

/// The beefy host is responsible for receiving BEEFY proofs from the queue and submitting
/// them to the counterparty.
pub struct BeefyHost<R, P, B, Q: ?Sized>
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
	Q: ProofBackend + ?Sized,
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

	/// The parts of the destination's consensus state this host reasons about.
	///
	/// Two things decide how the bytes are read. A solidity client stores its state abi-encoded
	/// where a substrate one stores it SCALE-encoded, and an apk client identifies an authority
	/// set by a commitment to its keys where the others carry a merkle root. Only these three
	/// values are wanted here, and every shape has them.
	fn state_summary(
		&self,
		encoded: &[u8],
		destination: StateMachine,
	) -> Result<StateSummary, anyhow::Error> {
		use alloy_sol_types::SolType;

		let summarise = |state: ConsensusState| StateSummary {
			latest_beefy_height: state.latest_beefy_height,
			current_set_id: state.current_authorities.id,
			next_set_id: state.next_authorities.id,
		};
		let summarise_apk = |state: beefy_verifier_primitives::ApkConsensusState| StateSummary {
			latest_beefy_height: state.latest_beefy_height,
			current_set_id: state.current_authorities.id,
			next_set_id: state.next_authorities.id,
		};

		let apk = matches!(self.prover, Prover::Apk(_));
		match (matches!(destination, StateMachine::Evm(_)), apk) {
			(true, true) => {
				let state =
					<ismp_abi::bls_beefy::BlsBeefy::BeefyConsensusState as SolType>::abi_decode(
						encoded,
					)
					.context("Could not abi-decode apk consensus state")?;
				let state: beefy_verifier_primitives::ApkConsensusState =
					state.try_into().map_err(|e| anyhow!("{e}"))?;
				Ok(summarise_apk(state))
			},
			(true, false) => {
				let state = <BeefyConsensusState as SolType>::abi_decode(encoded)
					.context("Could not abi-decode consensus state")?;
				Ok(summarise(state.into()))
			},
			(false, true) => {
				let state = beefy_verifier_primitives::ApkConsensusState::decode(&mut &encoded[..])
					.context("Could not decode apk consensus state")?;
				Ok(summarise_apk(state))
			},
			(false, false) => {
				let state = ConsensusState::decode(&mut &encoded[..])
					.context("Could not decode consensus state")?;
				Ok(summarise(state))
			},
		}
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
	Q: ProofBackend + ?Sized,
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
				tracing::error!(target: crate::LOG_TARGET, "Error in queue notification stream: {:?}", error);

				// Check if it's a Redis connection error and attempt reconnection
				if let Some(redis_error) = error.downcast_ref::<redis_async::error::Error>() {
					if matches!(
						redis_error,
						redis_async::error::Error::Connection(_) | redis_async::error::Error::IO(_)
					) {
						tracing::info!(target: crate::LOG_TARGET, "Redis connection error detected, attempting to reconnect");
						if let Err(e) = self.backend.reconnect_notifier().await {
							tracing::error!(
								target: crate::LOG_TARGET, "Failed to recreate notification subscription: {:?}",
								e
							);
						} else {
							tracing::info!(target: crate::LOG_TARGET, "Successfully recreated notification subscription");
						}
					}
				}

				// Try to recreate the notification stream
				notifications = match self
					.backend
					.queue_notifications(counterparty_state_machine)
					.await
				{
					Ok(n) => n,
					Err(e) => {
						tracing::error!(target: crate::LOG_TARGET, "Failed to recreate notification stream: {:?}", e);
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
									target: crate::LOG_TARGET, "{counterparty_state_machine} error pulling from mandatory queue: {err:?}"
								);
								// non-fatal error, keep trying
								continue;
							},
						};

					tracing::info!(target: crate::LOG_TARGET, "{counterparty_state_machine} got authority set handover proof for {set_id}");
					let encoded = counterparty
						.query_consensus_state(None, self.config.consensus_state_id)
						.await
						.context("Could not fetch consenus state")?; // somewhat fatal
					let StateSummary { next_set_id, .. } =
						self.state_summary(&encoded, counterparty_state_machine)?;

					// just some sanity checks
					if set_id < next_set_id {
						tracing::error!(
							target: crate::LOG_TARGET, "{counterparty_state_machine} got proof with set_id: {set_id} < next_set_id:{next_set_id}",
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
					if set_id != next_set_id {
						tracing::error!(
							target: crate::LOG_TARGET, "{counterparty_state_machine} consensus proof with set_id: {set_id} does not match next_set_id: {next_set_id}",
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
							target: crate::LOG_TARGET, "Error submitting consensus message to {counterparty_state_machine}: {err:?}",
						);

						// non-fatal error, keep trying. This will pull it from the queue once more
						continue;
					};

					tracing::info!(
						target: crate::LOG_TARGET, "Submitted mandatory proof to {counterparty_state_machine} for {set_id}"
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
					proof: ConsensusProof { message, finalized_height, set_id, .. },
				} = match item {
					Ok(Some(message)) => message,
					Ok(None) => break, // no new items in the queue
					Err(err) => {
						tracing::error!(target: crate::LOG_TARGET, "{counterparty_state_machine} error pulling from messages queue: {err:?}");
						// non-fatal error, keep trying
						continue;
					},
				};

				tracing::info!(
					target: crate::LOG_TARGET, "{counterparty_state_machine} got messages proof for {finalized_height}"
				);
				let encoded = counterparty
					.query_consensus_state(None, self.config.consensus_state_id)
					.await?; // somewhat fatal
				let StateSummary { latest_beefy_height, current_set_id, next_set_id } =
					self.state_summary(&encoded, counterparty_state_machine)?;

				// check if the update is relevant to us.
				if latest_beefy_height >= finalized_height {
					tracing::info!(
						target: crate::LOG_TARGET, "{counterparty_state_machine} saw proof for stale height {finalized_height}, current: {latest_beefy_height}",
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

				if set_id != current_set_id && set_id != next_set_id {
					tracing::info!(
						target: crate::LOG_TARGET, "{counterparty_state_machine} saw proof for unknown set_id {set_id}, current: {current_set_id}, next: {next_set_id}",
					);

					if set_id > next_set_id {
						tracing::info!(
							target: crate::LOG_TARGET, "{counterparty_state_machine} proof was for future set: {set_id}, next: {next_set_id}",
						);
						// break so that we can process a mandatory update
						break;
					} else if set_id < current_set_id {
						tracing::info!(
							target: crate::LOG_TARGET, "{counterparty_state_machine} proof was for older set: {set_id}, current: {current_set_id}",
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
						target: crate::LOG_TARGET, "Error submitting consensus message to {counterparty_state_machine}: {err:?}",
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
		let prover_state = self.prover.query_initial_consensus_state(None).await?;

		// An apk client is seeded with a commitment to the signing set's keys instead of a merkle
		// root, and the set that signs the first update has to be given one by hand, since every
		// later commitment arrives in a header digest. The next set's is left empty for that
		// reason.
		let consensus_state = match self.prover {
			Prover::Apk(ref apk) => {
				let at = apk
					.inner
					.relay_rpc
					.chain_get_block_hash(Some(prover_state.inner.latest_beefy_height.into()))
					.await?
					.ok_or_else(|| anyhow!("No block hash for the initial beefy height"))?;
				let commitment = apk.current_apk_commitment(at).await?;

				let inner = prover_state.inner.clone();
				let authority_set = |set: sp_consensus_beefy::mmr::BeefyAuthoritySet<H256>,
				                     apk_commitment: H256| {
					beefy_verifier_primitives::ApkAuthoritySet {
						id: set.id,
						len: set.len,
						apk_commitment,
					}
				};
				let state = beefy_verifier_primitives::ApkConsensusState {
					latest_beefy_height: inner.latest_beefy_height,
					beefy_activation_block: inner.beefy_activation_block,
					mmr_root_hash: inner.mmr_root_hash,
					current_authorities: authority_set(inner.current_authorities, H256(commitment)),
					next_authorities: authority_set(inner.next_authorities, H256::zero()),
				};
				ismp_abi::bls_beefy::BlsBeefy::BeefyConsensusState::from(state).abi_encode()
			},
			_ => {
				let state: BeefyConsensusState = prover_state.inner.into();
				state.abi_encode()
			},
		};

		Ok(Some(CreateConsensusState {
			consensus_state,
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
