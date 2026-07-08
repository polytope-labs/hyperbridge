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

//! Arc consensus prover.
//!
//! Fetches commit certificates (`arc_getCertificate`), execution headers and
//! EIP-1186 proofs of the ValidatorRegistry's active validator set, and
//! assembles them into [`VerifierStateUpdate`]s for the on-chain client.
//!
//! Arc RPC nodes run reth with the default zero proof window: `eth_getProof`
//! only serves the node's current tip, and with sub-second block times the
//! tip is stale by the time a client observes it. The `fetch_latest_*`
//! methods therefore request proofs at `"latest"` and discover the anchor
//! block afterwards, by checking the account proof against candidate headers'
//! state roots. The height-parameterised methods remain for nodes configured
//! with a larger `--rpc.eth-proof-window`.

pub mod error;
pub mod rpc;

#[cfg(test)]
mod tests;

pub use error::ProverError;

use arc_primitives::{
	active_set_element_slot, active_set_length_slot, validator_slots, CommitCertificate,
	ValidatorSet, ValidatorSetProof, VerifierState, VerifierStateUpdate,
	VALIDATOR_REGISTRY_ADDRESS,
};
use arc_verifier::{extract_validator_set, registry_storage_root};
use geth_primitives::{CodecHeader, Header};
use ismp::messaging::Keccak256;
use primitive_types::{H256, U256};
use rpc::{hex_to_bytes, ArcRpcClient, RpcAccountProof};
use std::{collections::BTreeSet, sync::Arc, time::Duration};

/// Keccak256 hasher for the prover, backed by sp-core.
pub struct Keccak256Hasher;

impl Keccak256 for Keccak256Hasher {
	fn keccak256(bytes: &[u8]) -> H256
	where
		Self: Sized,
	{
		sp_core::keccak_256(bytes).into()
	}
}

/// How many attempts to make at capturing a `"latest"`-anchored proof before
/// giving up. Attempts fail when the validator set changes mid-capture or the
/// anchor block can't be identified.
const MAX_ANCHOR_ATTEMPTS: usize = 5;

/// How far below/above the bracketing tip observations to scan for the
/// anchor block. Load-balanced RPC backends can lag each other by a few
/// blocks.
const ANCHOR_SCAN_MARGIN: u64 = 8;

/// Attempts at fetching the commit certificate for a freshly anchored block.
const CERTIFICATE_ATTEMPTS: usize = 5;

/// Arc prover for constructing light client updates.
#[derive(Clone)]
pub struct ArcProver {
	/// The underlying RPC client
	pub rpc: Arc<ArcRpcClient>,
}

impl ArcProver {
	/// Create a new prover for the given RPC endpoint.
	pub fn new(endpoint: impl Into<String>) -> Result<Self, ProverError> {
		Ok(Self { rpc: Arc::new(ArcRpcClient::new(endpoint)?) })
	}

	/// Create a new prover that sources commit certificates from a separate
	/// endpoint (third-party providers often don't proxy `arc_getCertificate`).
	pub fn with_certificate_endpoint(
		endpoint: impl Into<String>,
		certificate_endpoint: impl Into<String>,
	) -> Result<Self, ProverError> {
		Ok(Self {
			rpc: Arc::new(ArcRpcClient::with_certificate_endpoint(endpoint, certificate_endpoint)?),
		})
	}

	/// Fetch the latest block number from the node.
	pub async fn latest_height(&self) -> Result<u64, ProverError> {
		self.rpc.get_block_number().await
	}

	/// Fetch the commit certificate for `block_number`.
	pub async fn fetch_certificate(
		&self,
		block_number: u64,
	) -> Result<CommitCertificate, ProverError> {
		let certificate = self.rpc.get_certificate(block_number).await?.try_into_certificate()?;
		if certificate.height != block_number {
			return Err(ProverError::CertificateMismatch(format!(
				"requested height {block_number}, certificate is for {}",
				certificate.height
			)));
		}
		Ok(certificate)
	}

	/// Fetch a complete light client update anchored at the node's tip.
	pub async fn fetch_latest_update(&self) -> Result<VerifierStateUpdate, ProverError> {
		let (header, validator_set_proof, _) = self.fetch_latest_anchored().await?;
		let certificate = self
			.fetch_certificate_with_retry(header.number.low_u64(), CERTIFICATE_ATTEMPTS)
			.await?;
		Ok(VerifierStateUpdate { header, certificate, validator_set_proof })
	}

	/// Construct a trusted [`VerifierState`] anchored at the node's tip.
	///
	/// The validator set is reconstructed from a storage proof with the same
	/// code the on-chain verifier runs, so the bootstrapped state is exactly
	/// what verification of subsequent updates expects.
	pub async fn fetch_latest_verifier_state(&self) -> Result<VerifierState, ProverError> {
		let (header, _, current_validators) = self.fetch_latest_anchored().await?;
		Ok(VerifierState {
			current_validators,
			finalized_height: header.number.low_u64(),
			finalized_hash: header_hash(&header),
		})
	}

	/// Capture a `"latest"`-anchored validator set proof, returning the anchor
	/// header, the proof, and the extracted validator set.
	///
	/// Individual attempts can fail transiently: with sub-second blocks, RPC
	/// backends (and load balancers that resolve `"latest"` themselves) race
	/// their own tips, and the validator set can change mid-capture.
	async fn fetch_latest_anchored(
		&self,
	) -> Result<(CodecHeader, ValidatorSetProof, ValidatorSet), ProverError> {
		let mut last_error = None;
		for attempt in 0..MAX_ANCHOR_ATTEMPTS {
			if attempt > 0 {
				tokio::time::sleep(Duration::from_millis(500)).await;
			}
			match self.try_fetch_latest_anchored().await {
				Ok(anchored) => return Ok(anchored),
				Err(e) => {
					log::debug!(
						target: "arc-prover",
						"anchored capture attempt {attempt} failed: {e}"
					);
					last_error = Some(e);
				},
			}
		}

		Err(last_error.expect("MAX_ANCHOR_ATTEMPTS > 0; qed"))
	}

	async fn try_fetch_latest_anchored(
		&self,
	) -> Result<(CodecHeader, ValidatorSetProof, ValidatorSet), ProverError> {
		// Discover the current registration ids so the record slots can
		// be enumerated; re-proven below in a single anchored request.
		let length_word = self
			.rpc
			.get_storage_at_latest(VALIDATOR_REGISTRY_ADDRESS, active_set_length_slot())
			.await?;
		let set_slots = set_slots(word_to_u64(length_word)?);
		let discovery = self.rpc.get_proof_latest(VALIDATOR_REGISTRY_ADDRESS, &set_slots).await?;
		let registration_ids = parse_registration_ids(&discovery)?;

		let mut all_slots = set_slots;
		all_slots.extend(record_slots(&registration_ids));

		// Bracket the proof request with tip observations to bound the
		// anchor block, then identify it by verifying the account proof
		// against candidate state roots.
		let lower = self.rpc.get_block_number().await?;
		let proof = self.rpc.get_proof_latest(VALIDATOR_REGISTRY_ADDRESS, &all_slots).await?;
		let upper = self.rpc.get_block_number().await?.max(lower);

		let validator_set_proof = ValidatorSetProof {
			account_proof: decode_nodes(&proof.account_proof)?,
			storage_proof: collect_storage_nodes(&proof)?.into_iter().collect(),
		};

		let header = self
			.resolve_anchor(
				lower.saturating_sub(ANCHOR_SCAN_MARGIN),
				upper + ANCHOR_SCAN_MARGIN,
				&validator_set_proof.account_proof,
			)
			.await?
			.ok_or(ProverError::AnchorNotFound { lower, upper })?;

		// If the active set changed between discovery and capture, the
		// proof may not cover every slot; the caller retries from discovery.
		let validator_set =
			extract_validator_set::<Keccak256Hasher>(header.state_root, &validator_set_proof)?;

		Ok((header, validator_set_proof, validator_set))
	}

	/// Scan `[lower, upper]` (newest first) for the block whose state root the
	/// account proof verifies against.
	async fn resolve_anchor(
		&self,
		lower: u64,
		upper: u64,
		account_proof: &[Vec<u8>],
	) -> Result<Option<CodecHeader>, ProverError> {
		for height in (lower..=upper).rev() {
			let header = match self.rpc.get_block_by_number(height).await {
				Ok(header) => header,
				Err(ProverError::BlockNotFound(_)) => continue,
				Err(e) => return Err(e),
			};
			if registry_storage_root::<Keccak256Hasher>(header.state_root, account_proof.to_vec())
				.is_ok()
			{
				return Ok(Some(header));
			}
		}
		Ok(None)
	}

	async fn fetch_certificate_with_retry(
		&self,
		block_number: u64,
		attempts: usize,
	) -> Result<CommitCertificate, ProverError> {
		let mut last_error = ProverError::BlockNotFound(block_number);
		for attempt in 0..attempts {
			if attempt > 0 {
				tokio::time::sleep(Duration::from_millis(300)).await;
			}
			match self.fetch_certificate(block_number).await {
				Ok(certificate) => return Ok(certificate),
				Err(e) => last_error = e,
			}
		}
		Err(last_error)
	}

	/// Fetch a complete light client update for `block_number`.
	///
	/// Requires an RPC node whose `eth_getProof` window covers the block; use
	/// [`Self::fetch_latest_update`] against default-configured nodes.
	pub async fn fetch_update(
		&self,
		block_number: u64,
	) -> Result<VerifierStateUpdate, ProverError> {
		let header = self.rpc.get_block_by_number(block_number).await?;
		let certificate = self.fetch_certificate(block_number).await?;
		let validator_set_proof = self.fetch_validator_set_proof(block_number).await?;

		Ok(VerifierStateUpdate { header, certificate, validator_set_proof })
	}

	/// Fetch an EIP-1186 proof of the ValidatorRegistry's active validator set
	/// at `block_number`.
	///
	/// Two `eth_getProof` rounds: the first proves the active set length and
	/// registration ids, whose values determine the per-validator record slots
	/// proven by the second.
	pub async fn fetch_validator_set_proof(
		&self,
		block_number: u64,
	) -> Result<ValidatorSetProof, ProverError> {
		let length_slot = active_set_length_slot();
		let length = self
			.rpc
			.get_storage_at(VALIDATOR_REGISTRY_ADDRESS, length_slot, block_number)
			.await?;

		let set_slots = set_slots(word_to_u64(length)?);
		let set_proof =
			self.rpc.get_proof(VALIDATOR_REGISTRY_ADDRESS, &set_slots, block_number).await?;
		let registration_ids = parse_registration_ids(&set_proof)?;

		let record_proof = self
			.rpc
			.get_proof(VALIDATOR_REGISTRY_ADDRESS, &record_slots(&registration_ids), block_number)
			.await?;

		let mut storage_nodes = collect_storage_nodes(&set_proof)?;
		storage_nodes.extend(collect_storage_nodes(&record_proof)?);

		Ok(ValidatorSetProof {
			account_proof: decode_nodes(&record_proof.account_proof)?,
			storage_proof: storage_nodes.into_iter().collect(),
		})
	}

	/// Construct a trusted [`VerifierState`] anchored at `block_number`.
	///
	/// Requires an RPC node whose `eth_getProof` window covers the block; use
	/// [`Self::fetch_latest_verifier_state`] against default-configured nodes.
	pub async fn fetch_verifier_state(
		&self,
		block_number: u64,
	) -> Result<VerifierState, ProverError> {
		let header = self.rpc.get_block_by_number(block_number).await?;
		let proof = self.fetch_validator_set_proof(block_number).await?;
		let current_validators =
			extract_validator_set::<Keccak256Hasher>(header.state_root, &proof)?;

		Ok(VerifierState {
			current_validators,
			finalized_height: block_number,
			finalized_hash: header_hash(&header),
		})
	}
}

/// keccak256 of the RLP-encoded header.
pub fn header_hash(header: &CodecHeader) -> H256 {
	Header::from(header).hash::<Keccak256Hasher>()
}

/// The active-set length slot followed by the element slots for a set of
/// `length` registrations.
fn set_slots(length: u64) -> Vec<H256> {
	let mut slots = vec![active_set_length_slot()];
	slots.extend((0..length).map(active_set_element_slot::<Keccak256Hasher>));
	slots
}

/// The four record slots (status, public key header, public key data, voting
/// power) for each registration id.
fn record_slots(registration_ids: &[H256]) -> Vec<H256> {
	registration_ids
		.iter()
		.flat_map(|id| {
			let slots = validator_slots::<Keccak256Hasher>(*id);
			[slots.status, slots.public_key_header, slots.public_key_data, slots.voting_power]
		})
		.collect()
}

/// Read the registration ids from the values of a proof over [`set_slots`].
fn parse_registration_ids(proof: &RpcAccountProof) -> Result<Vec<H256>, ProverError> {
	proof
		.storage_proof
		.iter()
		.skip(1)
		.map(|entry| {
			let bytes = hex_to_bytes(&entry.value)?;
			if bytes.len() > 32 {
				return Err(ProverError::InvalidLength {
					field: "registration id",
					expected: 32,
					got: bytes.len(),
				});
			}
			let mut word = [0u8; 32];
			word[32 - bytes.len()..].copy_from_slice(&bytes);
			Ok(H256(word))
		})
		.collect()
}

fn collect_storage_nodes(proof: &RpcAccountProof) -> Result<BTreeSet<Vec<u8>>, ProverError> {
	let mut nodes = BTreeSet::new();
	for entry in &proof.storage_proof {
		for node in &entry.proof {
			nodes.insert(hex_to_bytes(node)?);
		}
	}
	Ok(nodes)
}

fn decode_nodes(nodes: &[String]) -> Result<Vec<Vec<u8>>, ProverError> {
	nodes.iter().map(|node| hex_to_bytes(node)).collect()
}

fn word_to_u64(word: H256) -> Result<u64, ProverError> {
	let value = U256::from_big_endian(word.as_bytes());
	if value > U256::from(u64::MAX) {
		return Err(ProverError::InvalidNumber);
	}
	Ok(value.low_u64())
}
