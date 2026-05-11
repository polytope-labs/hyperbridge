// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

//! Per-chain mandatory consensus task.
//!
//! Subscribes to the BEEFY backend's queue notification stream, drains the
//! mandatory queue on every `EpochChanged` notification, and submits each
//! authority-set-handover proof according to the chain's
//! [`SubmissionMode`] — either as an ERC-7821 batch from a delegated EOA
//! (`Batched`), or as three sequential plain-EOA txs (`Sequential`, for
//! chains that don't yet accept EIP-7702 transactions). `NewMessages`
//! notifications are ignored — this relayer only forwards mandatory updates.
//!
//! The drain loop only exits when `receive_mandatory_proof` returns `None`.
//! Every other transient condition — a future-set-id proof, an RPC error,
//! a submit failure — sleeps and retries, so messages from the queue are
//! submitted strictly in order with no ack gaps. A failure to delete an
//! acknowledged message is treated as fatal and returns from the task, since
//! it would otherwise cause the same proof to be submitted twice.

use std::{sync::Arc, time::Duration};

use anyhow::Context;
use codec::Decode;
use futures::StreamExt;

use beefy_verifier_primitives::ConsensusState;
use ismp::consensus::ConsensusStateId;
use tesseract_beefy::backend::{ProofBackend, QueueMessage, StreamMessage};
use tesseract_evm::EvmClient;
use tesseract_primitives::IsmpProvider;

use crate::{batch::submit_mandatory, config::SubmissionMode};

/// Single backoff used for every retry path in the drain loop — transient
/// errors (RPC read failure, submit failure, stream reconnection) and
/// future-set-id proofs alike. Only a `None` return from
/// `receive_mandatory_proof` exits the loop; every other condition waits this
/// long and retries.
const RETRY_SLEEP: Duration = Duration::from_secs(10);

/// Run the mandatory-consensus task for a single EVM chain.
///
/// Blocks forever; returns only if the notification stream ends.
pub async fn run_mandatory_task(
	backend: Arc<dyn ProofBackend>,
	consensus_state_id: ConsensusStateId,
	client: EvmClient,
	mode: SubmissionMode,
) -> anyhow::Result<()> {
	let chain = client.state_machine;
	let provider: Arc<dyn IsmpProvider> = Arc::new(client.clone());

	log::info!("[{chain}] starting mandatory consensus task (mode={mode:?})");

	let mut notifications = backend
		.queue_notifications(chain)
		.await
		.context("failed to subscribe to beefy queue notifications")?;

	while let Some(item) = notifications.next().await {
		let message = match item {
			Ok(m) => m,
			Err(err) => {
				log::error!("[{chain}] notification stream error: {err:?}");
				if let Err(e) = backend.reconnect_notifier().await {
					log::error!("[{chain}] failed to reconnect notifier: {e:?}");
				}
				notifications = match backend.queue_notifications(chain).await {
					Ok(n) => n,
					Err(e) => {
						log::error!("[{chain}] failed to re-subscribe: {e:?}");
						tokio::time::sleep(RETRY_SLEEP).await;
						continue;
					},
				};
				continue;
			},
		};

		if message != StreamMessage::EpochChanged {
			continue;
		}

		// Drain all available mandatory proofs. The *only* exit from this inner
		// loop is `Ok(None)` from `receive_mandatory_proof` — every other
		// condition (error, future handover, submit failure) sleeps and retries.
		loop {
			let pulled = match backend.receive_mandatory_proof(&chain).await {
				Ok(Some(m)) => m,
				Ok(None) => break,
				Err(err) => {
					log::error!(
						"[{chain}] error pulling mandatory proof: {err:?}; retrying in {}s",
						RETRY_SLEEP.as_secs()
					);
					tokio::time::sleep(RETRY_SLEEP).await;
					continue;
				},
			};

			let QueueMessage { id, proof } = pulled;
			let set_id = proof.set_id;
			log::info!(
				"[{chain}] received authority-set-handover proof (set_id={set_id}, finalized_height={})",
				proof.finalized_height
			);

			// Sanity-check against the counterparty's current view of consensus.
			let encoded_state = match provider.query_consensus_state(None, consensus_state_id).await
			{
				Ok(s) => s,
				Err(err) => {
					log::error!(
						"[{chain}] could not fetch consensus state: {err:?}; retrying in {}s",
						RETRY_SLEEP.as_secs()
					);
					tokio::time::sleep(RETRY_SLEEP).await;
					continue;
				},
			};
			let state = match ConsensusState::decode(&mut &encoded_state[..]) {
				Ok(s) => s,
				Err(err) => {
					log::error!(
						"[{chain}] failed to decode consensus state: {err:?}; retrying in {}s",
						RETRY_SLEEP.as_secs()
					);
					tokio::time::sleep(RETRY_SLEEP).await;
					continue;
				},
			};

			if set_id < state.next_authorities.id {
				log::warn!(
					"[{chain}] stale handover set_id={set_id} < next={}; discarding",
					state.next_authorities.id
				);
				backend
					.delete_message(&chain, &id, StreamMessage::EpochChanged)
					.await
					.context("failed to delete stale handover from queue")?;
				continue;
			}

			if set_id != state.next_authorities.id {
				// Future handover — can't apply it yet. Sleep and retry so
				// we pick up the matching proof once the chain catches up.
				log::warn!(
					"[{chain}] future handover set_id={set_id} != expected next={}; sleeping {}s before retry",
					state.next_authorities.id,
					RETRY_SLEEP.as_secs()
				);
				tokio::time::sleep(RETRY_SLEEP).await;
				continue;
			}

			match submit_mandatory(&client, proof.message.clone(), mode).await {
				Ok(tx_hash) => {
					log::info!(
						"[{chain}] mandatory update for set_id={set_id} landed: {tx_hash:?}"
					);
					backend
						.delete_message(&chain, &id, StreamMessage::EpochChanged)
						.await
						.context("failed to delete processed handover from queue")?;
				},
				Err(err) => {
					log::error!(
						"[{chain}] submission failed for set_id={set_id}: {err:?}; retrying in {}s",
						RETRY_SLEEP.as_secs()
					);
					tokio::time::sleep(RETRY_SLEEP).await;
					// Fall through to the next receive_mandatory_proof. With the
					// backend's visibility-timeout semantics, the failed message
					// will resurface and we'll retry it.
					continue;
				},
			}
		}
	}

	log::warn!("[{chain}] notification stream ended");
	Ok(())
}
