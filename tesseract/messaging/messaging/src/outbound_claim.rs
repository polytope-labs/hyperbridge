// Copyright (C) Polytope Labs Ltd.
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

//! Outbound consensus delivery reward claim task.
//!
//! Consumes [`PendingConsensusDeliveryClaim`] messages emitted by the
//! outbound pipeline after every successful delivery of a mandatory (authority
//! set rotation) consensus proof to a destination chain. For each pending
//! claim the task:
//!
//! 1. Waits for Hyperbridge's consensus client for the destination to advance past the destination
//!    block that processed the rotation delivery.
//! 2. Queries the destination RPC for a state proof of the destination's
//!    `pallet-ismp::BoundedStateCommitments` entry for `(Hyperbridge, rotation_height)`.
//! 3. Signs a domain separated payload with the Hyperbridge sr25519 signer.
//! 4. Submits `pallet_ismp_relayer::claim_outbound_consensus_delivery_reward` on Hyperbridge, which
//!    pays the configured per chain reward from the treasury to the claiming account.

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Context;
use codec::Encode;
use ismp::{
	consensus::{StateMachineHeight, StateMachineId},
	host::StateMachine,
	messaging::Proof,
};
use pallet_ismp_relayer::OutboundConsensusDeliveryClaim;
use primitive_types::U256;
use sp_core::{Pair, H256};
use tesseract_primitives::{
	IsmpProvider, PendingConsensusDeliveryClaim, StateProofQueryType, BEEFY_CONSENSUS_STATE_ID,
};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use tokio::sync::mpsc::Receiver;
use tracing::Instrument;
use transaction_fees::TransactionPayment;

/// Log target for the outbound claim task.
const LOG_TARGET: &str = concat!("tesseract-messaging", "-outbound-claim");

/// How long to wait between polls when checking whether Hyperbridge's view of
/// the destination has advanced past the rotation landing. A rotation lands
/// on the destination, then the destination finalizes, then HB's consensus
/// client for the destination verifies, which takes at least one rotation
/// cycle on HB. Thirty seconds is granular enough to catch this without
/// hammering the node.
const POLL_INTERVAL_SECS: u64 = 30;

/// Cap on polling iterations before giving up on a claim. Two hours is
/// generous enough for even the slowest substrate destinations (relay
/// epochs) but bounded so a stuck claim doesn't hold a task slot forever.
const MAX_WAIT_ITERATIONS: u32 = 240;

/// Domain separator for the relayer's sr25519 signature. Binds the claim to
/// this specific extrinsic so signatures cannot be replayed against any
/// other payload. Must match the value the pallet uses in
/// `process_outbound_consensus_delivery_claim`.
const SIGNATURE_DOMAIN: &[u8] = b"outbound-consensus-delivery-reward";

/// Drain `receiver` forever, processing one claim per message. Each claim is
/// handled in a spawned task so slow claims (waiting on HB to catch up to
/// the destination) do not block newer ones.
pub async fn run(
	hyperbridge: SubstrateClient<KeccakSubstrateChain>,
	destinations: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	mut receiver: Receiver<PendingConsensusDeliveryClaim>,
	tx_payment: Option<Arc<TransactionPayment>>,
) -> Result<(), anyhow::Error> {
	let hb_state_machine = hyperbridge.state_machine_id().state_id;
	let hyperbridge = Arc::new(hyperbridge);

	while let Some(pending) = receiver.recv().await {
		let Some(dest_provider) = destinations.get(&pending.destination).cloned() else {
			tracing::warn!(
				target: LOG_TARGET,
				destination = %pending.destination,
				"no provider for destination; dropping claim",
			);
			continue;
		};

		let hb = hyperbridge.clone();
		let tx_payment = tx_payment.clone();
		let span = tracing::info_span!(
			"outbound_claim",
			destination = %pending.destination,
			rotation_height = pending.rotation_height,
			set_id = pending.new_set_id,
		);
		tokio::spawn(
			async move {
				let dest_name = pending.destination.to_string();
				let set_id = pending.new_set_id;
				match process_claim(&hb, dest_provider, pending, hb_state_machine).await {
					Ok(()) =>
						if let Some(tx_payment) = &tx_payment {
							if let Err(err) =
								tx_payment.mark_rotation_claimed(&dest_name, set_id).await
							{
								tracing::warn!(
									target: LOG_TARGET,
									?err,
									"failed to mark rotation claim as claimed; startup replay \
									 may retry a claim we already submitted (idempotent on HB \
									 via the (dest, set_id) tag)",
								);
							}
						},
					Err(err) => {
						let msg = format!("{err:?}");
						tracing::error!(target: LOG_TARGET, err = %msg, "claim submission failed");
						// Permanent: another relayer won the race. Abandon so
						// the next startup doesn't replay forever.
						if msg.contains("OutboundRotationAlreadyClaimed") {
							if let Some(tx_payment) = &tx_payment {
								let _ = tx_payment
									.mark_rotation_abandoned(
										&dest_name,
										set_id,
										"OutboundRotationAlreadyClaimed",
									)
									.await;
							}
						}
					},
				}
			}
			.instrument(span),
		);
	}

	Err(anyhow::anyhow!("outbound-claim channel closed"))
}

async fn process_claim(
	hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
	dest: Arc<dyn IsmpProvider>,
	pending: PendingConsensusDeliveryClaim,
	hb_state_machine: StateMachine,
) -> anyhow::Result<()> {
	// Wait for HB's view of the destination to advance.
	let hb_provider: Arc<dyn IsmpProvider> = Arc::new(hyperbridge.clone());
	let dest_height = wait_for_hb_view(&hb_provider, dest.state_machine_id()).await?;

	// 2. Build the destination side storage key for the rotation entry and
	// fetch a state proof of it.
	let key = match pending.destination {
		StateMachine::Evm(_) => {
			let contract = dest.ismp_host_contract().ok_or_else(|| {
				anyhow::anyhow!(
					"destination {:?} has no `ismp_host_contract()`; cannot build EVM claim key",
					pending.destination,
				)
			})?;
			evm_state_commitment_key(hb_state_machine, contract, pending.rotation_height)?
		},
		_ => destination_hb_state_commitments_key(hb_state_machine, pending.rotation_height),
	};
	let proof_bytes = dest
		.query_state_proof(dest_height, StateProofQueryType::Arbitrary(vec![key]))
		.await
		.context("query_state_proof on destination")?;

	// 3. Sign the domain separated payload with the Hyperbridge signer.
	// Payload and hash match what the pallet reconstructs on its side.
	let claimer: [u8; 32] = hyperbridge.signer.public().0;
	let payload = (SIGNATURE_DOMAIN, pending.destination, pending.new_set_id, claimer).encode();
	let msg_hash = sp_core::keccak_256(&payload);
	let signature = hyperbridge.signer.sign(&msg_hash).0.to_vec();

	// 4. Assemble and submit.
	let claim = OutboundConsensusDeliveryClaim {
		state_proof: Proof {
			height: StateMachineHeight { id: dest.state_machine_id(), height: dest_height },
			proof: proof_bytes,
		},
		rotation_height: pending.rotation_height,
		new_set_id: pending.new_set_id,
		hb_consensus_state_id: BEEFY_CONSENSUS_STATE_ID,
		claimer,
	};

	tracing::info!(
		target: LOG_TARGET,
		dest_height,
		"submitting outbound consensus delivery claim",
	);
	hyperbridge
		.submit_outbound_consensus_delivery_claim(claim, signature)
		.await
		.context("submit_outbound_consensus_delivery_claim")?;
	tracing::info!(target: LOG_TARGET, "claim submitted");
	Ok(())
}

/// Poll `hb_provider.query_latest_height(dest_id)` until it returns a
/// non zero height, then return that height as the proof anchor. Returning
/// the latest height (rather than some specific target) is the simplest
/// usable anchor: HB is guaranteed to know the destination's state at any
/// height it reports, and a proof rooted at the latest verified block
/// necessarily includes any rotation landing at or before it.
async fn wait_for_hb_view(
	hb_provider: &Arc<dyn IsmpProvider>,
	dest_state_machine_id: StateMachineId,
) -> anyhow::Result<u64> {
	for _ in 0..MAX_WAIT_ITERATIONS {
		let hb_view = hb_provider.query_latest_height(dest_state_machine_id).await? as u64;
		if hb_view > 0 {
			return Ok(hb_view);
		}
		tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
	}
	Err(anyhow::anyhow!(
		"HB's view of destination {:?} never advanced within {} iterations",
		dest_state_machine_id.state_id,
		MAX_WAIT_ITERATIONS,
	))
}

/// Build the EVM-side storage key for the `EvmHost._stateCommitments`
/// timestamp slot at `(hb_state_machine_id, rotation_height)`. The EVM
/// `StateProofQueryType::Arbitrary` path expects a 52 byte key: contract
/// address (20 bytes) ++ slot hash (32 bytes).
///
/// Mirrors the pallet's `destination_hb_evm_state_commitments_key` so both
/// sides compute the same bytes.
fn evm_state_commitment_key(
	hb_state_machine: StateMachine,
	evm_host: sp_core::H160,
	rotation_height: u64,
) -> anyhow::Result<Vec<u8>> {
	use sp_core::keccak_256;

	// Hyperbridge's state machine id as the uint256 the EVM
	// `_stateCommitments[stateMachineId][height]` mapping expects.
	let hb_id_u256 = match hb_state_machine {
		StateMachine::Polkadot(id) | StateMachine::Kusama(id) => U256::from(id),
		other => {
			return Err(anyhow::anyhow!(
				"HB state machine must be Polkadot or Kusama for EVM key derivation, got {other:?}",
			));
		},
	};

	// Solidity mapping layout: `_stateCommitments` lives at slot 5 of the
	// EvmHost contract.
	const STATE_COMMITMENT_SLOT: u64 = 5;
	let slot = U256::from(STATE_COMMITMENT_SLOT);

	// outerSlot = keccak256(stateMachineId || STATE_COMMITMENT_SLOT)
	let mut outer_input = [0u8; 64];
	outer_input[..32].copy_from_slice(&hb_id_u256.to_big_endian());
	outer_input[32..].copy_from_slice(&slot.to_big_endian());
	let outer_slot = keccak_256(&outer_input);

	// innerSlot = keccak256(rotation_height || outerSlot)
	// This is the timestamp slot (offset 0 in StateCommitment).
	let mut inner_input = [0u8; 64];
	inner_input[..32].copy_from_slice(&U256::from(rotation_height).to_big_endian());
	inner_input[32..].copy_from_slice(&outer_slot);
	let timestamp_slot = H256::from(keccak_256(&inner_input));

	let mut key = Vec::with_capacity(52);
	key.extend_from_slice(&evm_host.0);
	key.extend_from_slice(&timestamp_slot.0);
	Ok(key)
}

/// Derive the destination chain's `pallet-ismp::BoundedStateCommitments`
/// storage key for the entry that records "Hyperbridge has been updated to
/// `rotation_height`". Layout matches the pallet's derivation exactly so
/// both sides compute the same bytes.
///
/// `BoundedStateCommitments` is a `StorageDoubleMap<StateMachineId, u64>`
/// with `Blake2_128Concat` on both keys:
///
/// `twox_128("Ismp") ++ twox_128("BoundedStateCommitments") ++
///  blake2_128_concat(scale_encode(StateMachineId)) ++
///  blake2_128_concat(scale_encode(u64 height))`
fn destination_hb_state_commitments_key(
	hb_state_machine: StateMachine,
	rotation_height: u64,
) -> Vec<u8> {
	use sp_core::hashing::{blake2_128, twox_128};
	let id =
		StateMachineId { state_id: hb_state_machine, consensus_state_id: BEEFY_CONSENSUS_STATE_ID };
	let id_encoded = id.encode();
	let height_encoded = rotation_height.encode();
	let mut key = Vec::with_capacity(64 + id_encoded.len() + height_encoded.len());
	key.extend_from_slice(&twox_128(b"Ismp"));
	key.extend_from_slice(&twox_128(b"BoundedStateCommitments"));
	key.extend_from_slice(&blake2_128(&id_encoded));
	key.extend_from_slice(&id_encoded);
	key.extend_from_slice(&blake2_128(&height_encoded));
	key.extend_from_slice(&height_encoded);
	key
}
