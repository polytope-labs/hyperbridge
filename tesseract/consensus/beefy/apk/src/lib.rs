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

//! Assembles the BEEFY consensus proof that `BlsApkBeefy` verifies.
//!
//! The relay chain half comes from `beefy_prover`, and everything specific to this path is built
//! here: the aggregate of the signers' keys in both groups, their aggregate signature, and the
//! SNARK that ties the aggregate to the authority set.
//!
//! Generating that SNARK needs a Go toolchain through cgo and a large structured reference string,
//! so it sits behind [`ApkProver`] rather than being called directly. Only the binary that
//! actually proves has to take those on.

use alloy_primitives::{Bytes, FixedBytes, U256};
use anyhow::anyhow;
use ark_bls12_381::{Fq, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{BigInteger, PrimeField};
use ark_serialize::CanonicalDeserialize;
use std::sync::Arc;
use subxt::config::HashFor;

use beefy_prover::bls::{
	aggregate_signatures, beefy_g1_authorities, beefy_g2_authorities, fetch_paired_justification,
	PairedSignature,
};
use beefy_verifier_primitives::{ConsensusState, BLS_G1_SIGNATURE_LEN};
use ismp_abi::bls_apk_beefy::BlsApkBeefy;

#[cfg(feature = "local")]
mod local;
#[cfg(feature = "local")]
pub use local::LocalProver;

/// Payload id of the mmr root in a BEEFY commitment.
const MMR_ROOT_ID: &[u8; 2] = b"mh";

/// Number of words in the circuit's participation bitlist.
const BITLIST_WORDS: usize = 5;

/// What the circuit is asked to prove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApkProofRequest {
	/// Compressed G1 keys of the whole authority set, in authority order.
	pub keys: Vec<[u8; BLS_G1_SIGNATURE_LEN]>,
	/// Indices of the authorities that signed.
	pub participation: Vec<u64>,
}

/// What it produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApkProof {
	/// The PLONK proof bytes the contract passes to its verifier.
	pub proof: Vec<u8>,
	/// The participation set as the contract reads it, one bit per authority slot.
	pub bitlist: [U256; BITLIST_WORDS],
	/// Poseidon2 over the padded key set, a public input of the proof.
	pub apk_commitment: [u8; 32],
}

/// Generates APK proofs.
///
/// An implementation may prove in process or hand the work to something else. Either way it is
/// slow, minutes rather than seconds, which is why this is only ever called by the prover service
/// and never on the delivery path.
#[async_trait::async_trait]
pub trait ApkProver: Send + Sync {
	/// Prove that the aggregate of the participating keys belongs to this key set.
	async fn prove(&self, request: ApkProofRequest) -> Result<ApkProof, anyhow::Error>;
}

/// Consensus prover for BEEFY verified through an aggregate public key proof.
///
/// The SNARK prover is a trait object rather than a type parameter, since nothing here needs to
/// know which one it is and a second generic would spread through every caller.
pub struct Prover<R: subxt::Config, P: subxt::Config> {
	/// The relay chain half, shared with every other BEEFY proof variant.
	pub inner: beefy_prover::Prover<R, P>,
	/// Produces the SNARK.
	pub apk: Arc<dyn ApkProver>,
}

impl<R, P> Clone for Prover<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
	beefy_prover::Prover<R, P>: Clone,
{
	fn clone(&self) -> Self {
		Self { inner: self.inner.clone(), apk: self.apk.clone() }
	}
}

impl<R, P> Prover<R, P>
where
	R: subxt::Config,
	P: subxt::Config,
{
	/// Build a prover over an existing relay chain prover.
	pub fn new(prover: beefy_prover::Prover<R, P>, apk: Arc<dyn ApkProver>) -> Self {
		Self { inner: prover, apk }
	}

	/// Build the whole update: the signed commitment, the aggregate proof, the mmr leaf and the
	/// parachain headers, in the shape `BlsApkBeefy.verify` decodes.
	///
	/// Takes the ECDSA shaped commitment every variant is handed, then reads the justification
	/// again itself, since that decode throws away the BLS half this path signs with.
	pub async fn consensus_proof(
		&self,
		signed_commitment: sp_consensus_beefy::SignedCommitment<
			u32,
			sp_consensus_beefy::ecdsa_crypto::Signature,
		>,
		consensus_state: ConsensusState,
	) -> Result<BlsApkBeefy::BlsApkBeefyConsensusProof, anyhow::Error> {
		let set_id = signed_commitment.commitment.validator_set_id;
		if set_id != consensus_state.current_authorities.id &&
			set_id != consensus_state.next_authorities.id
		{
			Err(anyhow!("Unknown validator set {set_id}"))?
		}

		let height = signed_commitment.commitment.block_number;
		let at = self
			.inner
			.relay_rpc
			.chain_get_block_hash(Some(height.into()))
			.await?
			.ok_or_else(|| anyhow!("No block hash for beefy block {height}"))?;

		let paired = fetch_paired_justification(&self.inner.relay_rpc, at).await?;
		let message = self.inner.bls_consensus_proof(paired.clone()).await?;
		let aggregate = self.aggregate(&paired, at).await?;
		let proof = self.apk.prove(aggregate.request).await?;

		// The circuit and the runtime must describe the same set, since the client checks the
		// proof against whichever of the two it was given. Disagreement here means the relay was
		// read at a height where the set had already rotated.
		let expected = apk_commitment_of(&aggregate.keys)?;
		if proof.apk_commitment != expected {
			Err(anyhow!(
				"apk commitment mismatch, circuit says {} and the key set hashes to {}",
				hex::encode(proof.apk_commitment),
				hex::encode(expected)
			))?
		}

		let mmr_root = message
			.mmr
			.commitment
			.payload
			.get_raw(MMR_ROOT_ID)
			.ok_or_else(|| anyhow!("Commitment carries no mmr root payload"))?
			.clone();

		let leaf = &message.mmr.latest_mmr_leaf;
		let relay = BlsApkBeefy::BlsApkRelayChainProof {
			commitment: BlsApkBeefy::Commitment {
				payload: vec![BlsApkBeefy::Payload {
					id: FixedBytes(*MMR_ROOT_ID),
					data: Bytes::from(mmr_root),
				}],
				blockNumber: message.mmr.commitment.block_number,
				validatorSetId: set_id,
			},
			bitlist: proof.bitlist,
			apk: aggregate.apk,
			apk2: aggregate.apk2,
			apkProof: Bytes::from(proof.proof),
			signature: aggregate.signature,
			latestMmrLeaf: BlsApkBeefy::BeefyMmrLeaf {
				// One byte carrying the major version in the top three bits and the minor in the
				// rest, which is how the reverse conversion in `ismp-abi` reads it back.
				version: {
					let (major, minor) = leaf.version.split();
					(major << 5) | minor
				},
				parentNumber: leaf.parent_number_and_hash.0,
				parentHash: FixedBytes(leaf.parent_number_and_hash.1 .0),
				nextAuthoritySet: BlsApkBeefy::AuthoritySetCommitment {
					id: leaf.beefy_next_authority_set.id,
					len: leaf.beefy_next_authority_set.len,
					root: FixedBytes(leaf.beefy_next_authority_set.keyset_commitment.0),
				},
				extra: FixedBytes(leaf.leaf_extra.0),
				leafIndex: U256::from(
					message.mmr.mmr_proof.leaf_indices.first().copied().unwrap_or_default(),
				),
			},
			mmrProof: message.mmr.mmr_proof.items.iter().map(|item| FixedBytes(item.0)).collect(),
		};

		let parachain = BlsApkBeefy::ParachainProof {
			parachains: message
				.parachain
				.parachains
				.iter()
				.map(|para| BlsApkBeefy::Parachain {
					index: U256::from(para.index),
					id: U256::from(para.para_id),
					header: Bytes::from(para.header.clone()),
				})
				.collect(),
			proof: message.parachain.proof.iter().map(|node| FixedBytes(*node)).collect(),
			leafCount: U256::from(message.parachain.total_leaves),
		};

		Ok(BlsApkBeefy::BlsApkBeefyConsensusProof { relay, parachain })
	}

	/// Sum the signers' keys in both groups and their signatures, and note who they were.
	///
	/// The G1 halves are what the circuit binds to and the G2 halves are what BEEFY's signature
	/// verifies against, so both are needed even though they describe one aggregate secret.
	async fn aggregate(
		&self,
		signed_commitment: &sp_consensus_beefy::SignedCommitment<u32, PairedSignature>,
		at: HashFor<R>,
	) -> Result<Aggregate, anyhow::Error> {
		let keys = beefy_g1_authorities(&self.inner.relay_rpc, Some(at)).await?;
		let g2_keys = beefy_g2_authorities(&self.inner.relay_rpc, Some(at)).await?;

		let mut apk_g1 = G1Projective::default();
		let mut apk_g2 = G2Projective::default();
		let mut signatures = Vec::new();
		let mut participation = Vec::new();
		for (index, signature) in signed_commitment.signatures.iter().enumerate() {
			let Some(signature) = signature else { continue };
			let g1 =
				keys.get(index).ok_or_else(|| anyhow!("Signer {index} is not an authority"))?;
			let g2 = g2_keys
				.get(index)
				.ok_or_else(|| anyhow!("Signer {index} is not an authority"))?;

			apk_g1 += decompress_g1(g1)?;
			apk_g2 += G2Affine::deserialize_compressed(&g2[..])
				.map_err(|_| anyhow!("Authority {index} has a malformed G2 key"))?;
			signatures.push(signature.g1_signature());
			participation.push(index as u64);
		}

		if participation.is_empty() {
			Err(anyhow!("Commitment carries no signatures"))?
		}

		let signature = decompress_g1(&aggregate_signatures(&signatures)?)?;

		Ok(Aggregate {
			apk: pack_g1(&apk_g1.into_affine())?,
			apk2: pack_g2(&apk_g2.into_affine())?,
			signature: pack_g1(&signature)?,
			request: ApkProofRequest { keys: keys.clone(), participation },
			keys,
		})
	}
}

/// The aggregated halves of one signed commitment.
struct Aggregate {
	apk: [FixedBytes<32>; 3],
	apk2: [FixedBytes<32>; 6],
	signature: [FixedBytes<32>; 3],
	keys: Vec<[u8; BLS_G1_SIGNATURE_LEN]>,
	request: ApkProofRequest,
}

/// Poseidon2 over the key set, padded to the circuit's width. The same value the runtime pallet
/// publishes in a header digest.
fn apk_commitment_of(keys: &[[u8; BLS_G1_SIGNATURE_LEN]]) -> Result<[u8; 32], anyhow::Error> {
	let points = keys.iter().map(decompress_g1).collect::<Result<Vec<_>, _>>()?;
	let padded = apk_commitment::padded_to_circuit_width(&points);
	Ok(apk_commitment::public_keys_commitment_bytes(&padded))
}

fn decompress_g1(key: &[u8; BLS_G1_SIGNATURE_LEN]) -> Result<G1Affine, anyhow::Error> {
	G1Affine::deserialize_compressed(&key[..]).map_err(|_| anyhow!("Malformed G1 point"))
}

/// Curve points reach the SNARK verifier as raw 48 byte coordinates packed into 32 byte words,
/// which is not the zero padded layout the EIP-2537 precompiles take.
fn pack_g1(point: &G1Affine) -> Result<[FixedBytes<32>; 3], anyhow::Error> {
	let (x, y) = point.xy().ok_or_else(|| anyhow!("Aggregate is the identity"))?;
	words(&[x, y])?.try_into().map_err(|_| anyhow!("G1 point is not three words"))
}

fn pack_g2(point: &G2Affine) -> Result<[FixedBytes<32>; 6], anyhow::Error> {
	let (x, y) = point.xy().ok_or_else(|| anyhow!("Aggregate is the identity"))?;
	words(&[&x.c0, &x.c1, &y.c0, &y.c1])?
		.try_into()
		.map_err(|_| anyhow!("G2 point is not six words"))
}

fn words(coordinates: &[&Fq]) -> Result<Vec<FixedBytes<32>>, anyhow::Error> {
	let mut bytes = Vec::with_capacity(coordinates.len() * 48);
	for coordinate in coordinates {
		bytes.extend_from_slice(&coordinate.into_bigint().to_bytes_be());
	}
	if bytes.len() % 32 != 0 {
		Err(anyhow!("Packed point is not a whole number of words"))?
	}
	Ok(bytes.chunks(32).map(FixedBytes::<32>::from_slice).collect())
}
