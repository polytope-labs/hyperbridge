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
use anyhow::anyhow;
use beefy_verifier_primitives::ConsensusState;
use codec::{Decode, Encode};
use futures::Stream;
use ismp::{consensus::ConsensusStateId, host::StateMachine};
use pallet_beefy_consensus_proofs::types::SIGNATURE_DOMAIN;
use sp_core::{sr25519, Pair};
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
	/// Consensus state id under which the BEEFY state is stored in `pallet-ismp`
	consensus_state_id: ConsensusStateId,
	/// In-memory cache of the last saved consensus state
	state: Arc<RwLock<Option<crate::prover::ProverConsensusState>>>,
}

impl<P: subxt::Config> OnchainBackend<P> {
	pub fn new(
		client: OnlineClient<P>,
		rpc_client: RpcClient,
		signer: sr25519::Pair,
		consensus_state_id: ConsensusStateId,
	) -> Self {
		Self { client, rpc_client, signer, consensus_state_id, state: Arc::new(RwLock::new(None)) }
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
			("proof", Value::from_bytes(proof_bytes)),
		]);
		let signature_value = Value::from_bytes(signature.0);

		let tx = subxt::dynamic::tx(
			"BeefyConsensusProofs",
			"submit_proof",
			vec![payload_value, signature_value],
		);

		send_unsigned_extrinsic(&self.client, tx, true).await?;

		tracing::info!(
			target: crate::LOG_TARGET, "Successfully submitted proof to pallet-beefy-consensus-proofs (height: {})",
			proof.finalized_height
		);

		Ok(())
	}

	/// Fetch the BEEFY consensus state from `pallet-ismp` via the `ismp_queryConsensusState` RPC.
	async fn fetch_onchain_consensus_state(&self) -> Result<ConsensusState, anyhow::Error> {
		let params = rpc_params![None::<u32>, self.consensus_state_id];
		let encoded: Vec<u8> = self
			.rpc_client
			.request("ismp_queryConsensusState", params)
			.await
			.map_err(|e| anyhow!("ismp_queryConsensusState failed: {e}"))?;
		let state = ConsensusState::decode(&mut &encoded[..])
			.map_err(|e| anyhow!("Failed to decode on-chain BEEFY consensus state: {e}"))?;
		Ok(state)
	}

	/// Fetch `pallet-beefy-consensus-proofs::LastProvenHeight` via subxt storage query.
	async fn fetch_last_proven_height(&self) -> Result<u64, anyhow::Error> {
		let query = subxt::dynamic::storage(
			"BeefyConsensusProofs",
			"LastProvenHeight",
			Vec::<Value>::new(),
		);
		let storage = self.client.storage().at_latest().await?;
		let Some(raw) = storage.fetch(&query).await? else {
			return Ok(0);
		};
		let bytes = raw.encoded();
		let height = u64::decode(&mut &bytes[..])
			.map_err(|e| anyhow!("Failed to decode LastProvenHeight: {e}"))?;
		Ok(height)
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
		state_machine: &StateMachine,
		proof: ConsensusProof,
	) -> Result<(), anyhow::Error> {
		tracing::info!(target: crate::LOG_TARGET, "Submitting mandatory proof to pallet for {state_machine}");
		self.submit_to_pallet(&proof).await
	}

	async fn send_messages_proof(
		&self,
		state_machine: &StateMachine,
		proof: ConsensusProof,
	) -> Result<(), anyhow::Error> {
		tracing::info!(target: crate::LOG_TARGET, "Submitting messages proof to pallet for {state_machine}");
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
		let finalized_parachain_height = self.fetch_last_proven_height().await?;

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
