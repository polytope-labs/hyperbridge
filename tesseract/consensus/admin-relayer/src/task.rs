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

/// Run the mandatory-consensus task for a single EVM chain.
///
/// Blocks forever; returns only if the underlying notification stream errors
/// unrecoverably.
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
						tokio::time::sleep(Duration::from_secs(5)).await;
						continue;
					},
				};
				continue;
			},
		};

		if message != StreamMessage::EpochChanged {
			continue;
		}

		// Drain all available mandatory proofs.
		loop {
			let pulled = match backend.receive_mandatory_proof(&chain).await {
				Ok(Some(m)) => m,
				Ok(None) => break,
				Err(err) => {
					log::error!("[{chain}] error pulling mandatory proof: {err:?}");
					break;
				},
			};

			let QueueMessage { id, proof } = pulled;
			let set_id = proof.set_id;
			log::info!(
				"[{chain}] received authority-set-handover proof (set_id={set_id}, finalized_height={})",
				proof.finalized_height
			);

			// Sanity-check against the counterparty's current view of consensus.
			let encoded_state = match provider
				.query_consensus_state(None, consensus_state_id)
				.await
			{
				Ok(s) => s,
				Err(err) => {
					log::error!("[{chain}] could not fetch consensus state: {err:?}");
					break;
				},
			};
			let state = match ConsensusState::decode(&mut &encoded_state[..]) {
				Ok(s) => s,
				Err(err) => {
					log::error!("[{chain}] failed to decode consensus state: {err:?}");
					break;
				},
			};

			if set_id < state.next_authorities.id {
				log::warn!(
					"[{chain}] stale handover set_id={set_id} < next={}; discarding",
					state.next_authorities.id
				);
				if let Err(err) = backend
					.delete_message(&chain, &id, StreamMessage::EpochChanged)
					.await
				{
					log::error!("[{chain}] failed to delete stale handover: {err:?}");
				}
				continue;
			}

			if set_id != state.next_authorities.id {
				// Future handover — we cannot apply it yet. Leave the message
				// visible for the next cycle rather than hot-looping on it.
				log::warn!(
					"[{chain}] future handover set_id={set_id} != expected next={}; breaking",
					state.next_authorities.id
				);
				break;
			}

			match submit_mandatory(&client, proof.message.clone(), mode).await {
				Ok(tx_hash) => {
					log::info!(
						"[{chain}] mandatory update for set_id={set_id} landed: {tx_hash:?}"
					);
					if let Err(err) = backend
						.delete_message(&chain, &id, StreamMessage::EpochChanged)
						.await
					{
						log::error!(
							"[{chain}] failed to delete processed handover: {err:?}"
						);
						return Err(err);
					}
				},
				Err(err) => {
					log::error!(
						"[{chain}] mandatory batch submission failed for set_id={set_id}: {err:?}"
					);
					// leave the message on the queue; it will be retried after
					// the visibility timeout expires.
					break;
				},
			}
		}
	}

	log::warn!("[{chain}] notification stream ended");
	Ok(())
}
