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
//! Modeled on the fee accumulation task. For every
//! [`PendingConsensusDeliveryClaim`] pushed by the outbound delivery task:
//!
//! 1. Wait for Hyperbridge's consensus client for the destination to verify a height >=
//!    `delivery_height` (same `wait_for_state_machine_update` helper fee accumulation uses).
//! 2. Build an EIP-1186 storage proof of `HandlerV2._epochs[set_id]` on the destination at the
//!    height HB just verified.
//! 3. Sign `outbound_consensus_delivery_message(set_id, destination, payee)` with the destination's
//!    EVM key.
//! 4. Submit `pallet_ismp_relayer::claim_outbound_consensus_delivery_reward` (unsigned) on
//!    Hyperbridge.
//! 5. Delete the persisted claim row.

use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Context as _};
use ismp::{consensus::StateMachineHeight, host::StateMachine, messaging::Proof};
use pallet_ismp_relayer::{
	outbound_consensus_delivery_message, OutboundConsensusDeliveryClaim, HANDLER_V2_EPOCHS_SLOT,
};
use polkadot_sdk::sp_runtime::AccountId32;
use primitive_types::{H160, U256};
use sp_core::keccak_256;
use tesseract_primitives::{
	wait_for_state_machine_update, IsmpProvider, PendingConsensusDeliveryClaim, StateProofQueryType,
};
use tesseract_substrate::{config::KeccakSubstrateChain, SubstrateClient};
use tokio::sync::mpsc::Receiver;
use tracing::Instrument;
use transaction_fees::TransactionPayment;

const LOG_TARGET: &str = "tesseract-messaging-outbound-claim";

/// Drive a single relayer's outbound consensus delivery claims. Mirrors
/// [`fee_accumulation`](crate::fee_accumulation) shape: drain the trigger
/// channel, wait for HB to verify the destination block, prove
/// `_epochs[set_id]`, submit the claim extrinsic, delete the persisted row.
///
/// `payee` is the sr25519 Hyperbridge account the reward will be credited
/// to — the relayer's own HB account is the obvious default; the cli
/// passes `hyperbridge.signer.public().into()`.
pub async fn run(
	hyperbridge: SubstrateClient<KeccakSubstrateChain>,
	destinations: HashMap<StateMachine, Arc<dyn IsmpProvider>>,
	mut receiver: Receiver<PendingConsensusDeliveryClaim>,
	tx_payment: Option<Arc<TransactionPayment>>,
	payee: AccountId32,
) -> Result<(), anyhow::Error> {
	let hb_provider: Arc<dyn IsmpProvider> = Arc::new(hyperbridge.clone());
	let payee_bytes: [u8; 32] = *payee.as_ref();

	while let Some(pending) = receiver.recv().await {
		let span = tracing::info_span!(
			"outbound_claim",
			destination = %pending.destination,
			delivery_height = pending.delivery_height,
			set_id = pending.set_id,
		);
		let dest = destinations.get(&pending.destination).cloned();
		let hb = hyperbridge.clone();
		let hb_view = hb_provider.clone();
		let tx_payment = tx_payment.clone();
		async move {
			let Some(dest) = dest else {
				tracing::warn!(target: LOG_TARGET, "no provider for destination; dropping claim");
				return;
			};
			match process_claim(&hb, hb_view, dest, &pending, payee_bytes).await {
				Ok(()) => {
					tracing::info!(target: LOG_TARGET, "claim submitted");
					if let Some(tx_payment) = &tx_payment {
						let _ = tx_payment
							.delete_rotation_claim(&pending.destination.to_string(), pending.set_id)
							.await;
					}
				},
				Err(err) => {
					tracing::error!(
						target: LOG_TARGET,
						?err,
						"claim submission failed; row left in DB for next-startup retry",
					);
				},
			}
		}
		.instrument(span)
		.await;
	}

	Err(anyhow!("outbound-claim channel closed"))
}

async fn process_claim(
	hyperbridge: &SubstrateClient<KeccakSubstrateChain>,
	hb_provider: Arc<dyn IsmpProvider>,
	dest: Arc<dyn IsmpProvider>,
	pending: &PendingConsensusDeliveryClaim,
	payee: [u8; 32],
) -> anyhow::Result<()> {
	// Same shape fee_accumulation uses: wait for HB's view of the
	// destination to cross the delivery block.
	let dest_height = wait_for_state_machine_update(
		dest.state_machine_id(),
		hb_provider.clone(),
		dest.clone(),
		pending.delivery_height,
	)
	.await
	.context("wait_for_state_machine_update")?;

	// Build the 52-byte EIP-1186 key the EVM verifier expects:
	// `handler_v2 (20) || keccak256(set_id || HANDLER_V2_EPOCHS_SLOT) (32)`.
	// The EVM provider reads HandlerV2 from `EvmHost.hostParams().handler`
	// so it stays in sync with whatever governance has set on chain.
	// Substrate destinations report `None` and are skipped — the claim
	// flow is EVM-only.
	let handler = dest.handler_v2_address().await.ok_or_else(|| {
		anyhow!(
			"destination has no HandlerV2 address (non-EVM, or hostParams() RPC failed); \
			 cannot derive _epochs[set_id] key",
		)
	})?;
	let key = epochs_slot_key(handler, pending.set_id);

	let proof_bytes = dest
		.query_state_proof(dest_height, StateProofQueryType::Arbitrary(vec![key]))
		.await
		.context("query_state_proof on destination")?;

	let msg = outbound_consensus_delivery_message(pending.set_id, pending.destination, payee);
	let signature = dest.sign(&msg);

	let claim = OutboundConsensusDeliveryClaim {
		state_proof: Proof {
			height: StateMachineHeight { id: dest.state_machine_id(), height: dest_height },
			proof: proof_bytes,
		},
		set_id: pending.set_id,
		payee,
		signature,
	};

	hyperbridge
		.submit_outbound_consensus_delivery_claim(claim)
		.await
		.context("submit_outbound_consensus_delivery_claim")?;
	Ok(())
}

/// `keccak256(set_id || HANDLER_V2_EPOCHS_SLOT)` prefixed with the
/// HandlerV2 contract address. Matches the pallet-side derivation in
/// `process_outbound_consensus_delivery_claim`.
fn epochs_slot_key(handler: H160, set_id: u64) -> Vec<u8> {
	let mut input = [0u8; 64];
	input[..32].copy_from_slice(&U256::from(set_id).to_big_endian());
	input[32..].copy_from_slice(&U256::from(HANDLER_V2_EPOCHS_SLOT).to_big_endian());
	let slot_hash = keccak_256(&input);

	let mut key = Vec::with_capacity(52);
	key.extend_from_slice(&handler.0);
	key.extend_from_slice(&slot_hash);
	key
}
