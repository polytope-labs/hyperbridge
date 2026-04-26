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

//! Pallet-based implementation of the ProofBackend trait.
//!
//! Instead of queuing proofs for a separate host process, this backend submits proofs
//! directly to the `pallet-beefy-consensus-proofs` on the hyperbridge parachain via
//! unsigned extrinsics.

use super::{ConsensusProof, ProofBackend, QueueMessage, StreamMessage};
use alloy_sol_types::SolType;
use anyhow::anyhow;
use beefy_verifier_primitives::ConsensusState;
use codec::{Decode, Encode};
use futures::Stream;
use ismp::{
	consensus::{ConsensusStateId, StateMachineId},
	host::StateMachine,
};
use pallet_beefy_consensus_proofs::types::SIGNATURE_DOMAIN;
use polkadot_sdk::*;
use sp_core::{sr25519, Pair};
use sp_runtime::{generic::Header, traits::Header as _};
use std::{pin::Pin, sync::Arc};
use subxt::{
	dynamic::Value,
	ext::subxt_rpcs::{rpc_params, RpcClient},
	utils::MultiSignature,
	OnlineClient,
};
use tesseract_substrate::extrinsic::send_unsigned_extrinsic;
use tokio::sync::RwLock;

/// Proof backend that submits proofs directly to `pallet-beefy-consensus-proofs`
/// on the hyperbridge parachain.
///
/// When `send_mandatory_proof` or `send_messages_proof` is called, this backend
/// constructs a `SubmitProofPayload`, signs it with SR25519, and submits the
/// unsigned extrinsic directly to the chain. The pallet handles verification,
/// consensus state updates, and reward distribution.
///
/// The host-side methods (`receive_*`, `queue_notifications`, `delete_message`)
/// are no-ops since the pallet processes proofs inline — there is no intermediate
/// queue for a separate host process to consume.
pub struct OnchainBackend<P: subxt::Config> {
	/// Subxt client for the hyperbridge parachain
	client: OnlineClient<P>,
	/// Raw RPC client for custom ISMP RPC calls
	rpc_client: RpcClient,
	/// SR25519 keypair used to sign proof payloads
	signer: sr25519::Pair,
	/// The coprocessor (hyperbridge) state machine ID
	state_machine_id: StateMachineId,
	/// In-memory cache of the last saved consensus state
	state: Arc<RwLock<Option<crate::prover::ProverConsensusState>>>,
}

impl<P: subxt::Config> OnchainBackend<P> {
	pub fn new(
		client: OnlineClient<P>,
		rpc_client: RpcClient,
		signer: sr25519::Pair,
		state_machine_id: StateMachineId,
	) -> Self {
		Self { client, rpc_client, signer, state_machine_id, state: Arc::new(RwLock::new(None)) }
	}
}

impl<P> OnchainBackend<P>
where
	P: subxt::Config + Send + Sync,
	P::Signature: From<MultiSignature> + Send + Sync,
{
	/// Submit a consensus proof to `pallet-beefy-consensus-proofs::submit_proof`.
	async fn submit_to_pallet(&self, proof: &ConsensusProof) -> Result<(), anyhow::Error> {
		let proof_bytes = proof.message.consensus_proof.clone();
		let submitter_bytes: [u8; 32] = self.signer.public().0;

		// Sign: keccak256((SIGNATURE_DOMAIN, submitter, keccak256(proof)).encode())
		let proof_digest = sp_core::hashing::keccak_256(&proof_bytes);
		let msg_preimage = (SIGNATURE_DOMAIN, submitter_bytes, proof_digest).encode();
		let signed_msg = sp_core::hashing::keccak_256(&msg_preimage);
		let signature = self.signer.sign(&signed_msg);

		// Construct the dynamic extrinsic
		let payload_value = Value::named_composite([
			("submitter", Value::from_bytes(submitter_bytes)),
			("proof", Value::from_bytes(proof_bytes.clone())),
		]);
		let signature_value = Value::from_bytes(signature.0);

		let tx = subxt::dynamic::tx(
			"BeefyConsensusProofs",
			"submit_proof",
			vec![payload_value, signature_value],
		);

		let result = send_unsigned_extrinsic(&self.client, tx, false).await;

		// Wait one block so that load_state() on the next iteration sees the
		// updated LastProvenHeight written by the pallet in the previous block.
		let mut blocks = self.client.blocks().subscribe_best().await?;
		let _ = blocks.next().await;

		result?;

		tracing::info!(
			target: crate::LOG_TARGET,
			"Successfully submitted proof to pallet-beefy-consensus-proofs (relay_height: {}, parachain_height: {})",
			proof.finalized_height,
			extract_parachain_height(&proof_bytes).map_or("null".to_string(), |h| h.to_string()),
		);

		Ok(())
	}

	/// Fetch the BEEFY consensus state from `pallet-ismp` via the `ismp_queryConsensusState` RPC.
	async fn fetch_onchain_consensus_state(&self) -> Result<ConsensusState, anyhow::Error> {
		let params = rpc_params![None::<u32>, ismp_beefy::BEEFY_CONSENSUS_ID];
		let encoded: Vec<u8> = self
			.rpc_client
			.request("ismp_queryConsensusState", params)
			.await
			.map_err(|e| anyhow!("ismp_queryConsensusState failed: {e}"))?;
		let state = ConsensusState::decode(&mut &encoded[..])
			.map_err(|e| anyhow!("Failed to decode on-chain BEEFY consensus state: {e}"))?;
		Ok(state)
	}

	/// Fetch the latest proven height for the coprocessor from `pallet-ismp` via RPC.
	async fn fetch_last_proven_height(&self) -> Result<u64, anyhow::Error> {
		let params = rpc_params![self.state_machine_id];
		let height: u32 =
			self.rpc_client.request("ismp_queryStateMachineLatestHeight", params).await?;
		Ok(height as u64)
	}
}

#[async_trait::async_trait]
impl<P> ProofBackend for OnchainBackend<P>
where
	P: subxt::Config + Send + Sync,
	P::Signature: From<MultiSignature> + Send + Sync,
{
	async fn init_queues(&self, _state_machines: &[StateMachine]) -> Result<(), anyhow::Error> {
		// No queues needed — proofs are submitted directly to the pallet.
		Ok(())
	}

	async fn send_mandatory_proof(
		&self,
		_state_machines: &[StateMachine],
		proof: ConsensusProof,
	) -> Result<(), anyhow::Error> {
		self.submit_to_pallet(&proof).await
	}

	async fn send_messages_proof(
		&self,
		state_machines: &[StateMachine],
		proof: ConsensusProof,
	) -> Result<(), anyhow::Error> {
		tracing::info!("Submitting messages proof to pallet for {state_machines:?}");
		self.submit_to_pallet(&proof).await
	}

	async fn save_state(
		&self,
		state: &crate::prover::ProverConsensusState,
	) -> Result<(), anyhow::Error> {
		*self.state.write().await = Some(state.clone());
		Ok(())
	}

	/// Fetches the prover consensus state from on-chain. The `inner` BEEFY consensus state is
	/// read from `pallet-ismp`'s `ConsensusStates` storage (the pallet is the source of truth
	/// after every accepted proof). The `finalized_parachain_height` is read from
	/// `pallet-beefy-consensus-proofs::LastProvenHeight` — the highest parachain height the
	/// pallet has ever accepted a proof for.
	async fn load_state(&self) -> Result<crate::prover::ProverConsensusState, anyhow::Error> {
		let inner = self.fetch_onchain_consensus_state().await?;
		let onchain_height = self.fetch_last_proven_height().await?;
		let local_height =
			self.state.read().await.as_ref().map_or(0, |s| s.finalized_parachain_height);
		let finalized_parachain_height = core::cmp::max(onchain_height, local_height);

		let state = crate::prover::ProverConsensusState { inner, finalized_parachain_height };
		*self.state.write().await = Some(state.clone());
		Ok(state)
	}

	async fn queue_notifications(
		&self,
		_state_machine: StateMachine,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<StreamMessage, anyhow::Error>> + Send>>,
		anyhow::Error,
	> {
		// The pallet backend submits proofs directly — no host needs to consume from a queue.
		// Return a stream that never yields so the host's start_consensus idles.
		Ok(Box::pin(futures::stream::pending()))
	}

	async fn receive_mandatory_proof(
		&self,
		_state_machine: &StateMachine,
	) -> Result<Option<QueueMessage>, anyhow::Error> {
		// No queue to receive from — proofs go directly to the pallet.
		Ok(None)
	}

	async fn receive_messages_proof(
		&self,
		_state_machine: &StateMachine,
	) -> Result<Option<QueueMessage>, anyhow::Error> {
		// No queue to receive from — proofs go directly to the pallet.
		Ok(None)
	}

	async fn delete_message(
		&self,
		_state_machine: &StateMachine,
		_message_id: &str,
		_message_type: StreamMessage,
	) -> Result<(), anyhow::Error> {
		// No queue messages to delete.
		Ok(())
	}
}

/// Extracts the parachain block number from the consensus proof bytes.
/// The proof format is `proof_type_byte || ABI-encoded proof`. The parachain
/// header is SCALE-encoded `sp_runtime::generic::Header<u32, H256>`.
fn extract_parachain_height(proof_bytes: &[u8]) -> Option<u32> {
	use pallet_beefy_consensus_proofs::types::{PROOF_TYPE_NAIVE, PROOF_TYPE_SP1};

	let proof_type = *proof_bytes.first()?;
	let abi_payload = &proof_bytes[1..];

	let header_bytes: Vec<u8> = match proof_type {
		PROOF_TYPE_SP1 => {
			let proof =
				<ismp_solidity_abi::sp1_beefy::SP1Beefy::SP1BeefyProof as SolType>::abi_decode_params(
					abi_payload,
				)
				.ok()?;
			proof.headers.first()?.header.to_vec()
		},
		PROOF_TYPE_NAIVE => {
			let proof =
				<ismp_solidity_abi::beefy::BeefyConsensusProof as SolType>::abi_decode_params(
					abi_payload,
				)
				.ok()?;
			proof.parachain.parachains.first()?.header.to_vec()
		},
		_ => return None,
	};

	let header =
		Header::<u32, sp_runtime::traits::Keccak256>::decode(&mut &header_bytes[..]).ok()?;
	Some(*header.number())
}
