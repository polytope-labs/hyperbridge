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

//! Proving BEEFY commitments signed with aggregate BLS12-381.
//!
//! A relay whose BEEFY authorities hold paired `ecdsa_bls_crypto` keys puts both signatures on the
//! wire. `crate::relay::decode_beefy_justification` keeps the ECDSA half so the existing verifier
//! works unchanged; this module keeps the BLS half instead, and builds a proof the aggregate
//! verifier can check in a single pairing.
//!
//! This only applies to a relay whose keyset commitment is over BLS public keys. A chain
//! committing Ethereum addresses cannot be proven this way, and vice versa.

use anyhow::anyhow;
use codec::{Decode, Encode};
use polkadot_sdk::*;
use sp_consensus_beefy::{SignedCommitment, VersionedFinalityProof, BEEFY_ENGINE_ID};
use subxt::{backend::legacy::LegacyRpcMethods, Config};
use subxt_core::config::HashFor;

use beefy_verifier_primitives::{
	BlsConsensusMessage, BlsMmrProof, PairedAuthority, BLS_G1_SIGNATURE_LEN, BLS_G2_PUBLIC_KEY_LEN,
};

use crate::{
	build_parachain_proof,
	relay::{fetch_mmr_proof, paras_parachains},
	Prover, BEEFY_AUTHORITIES,
};

/// Wire size of a paired (ECDSA, BLS12-381) BEEFY signature.
pub const PAIRED_LEN: usize = 177;

/// Offset of the BLS G1 signature within a paired signature: the ECDSA half is 65 bytes, then the
/// `DoubleSignature` begins with its 48-byte G1 point.
const PAIRED_SIGNATURE_G1_OFFSET: usize = 65;

/// A paired (ECDSA, BLS12-381) signature exactly as SCALE-encoded on the wire.
#[derive(Clone)]
pub struct PairedSignature(pub [u8; PAIRED_LEN]);

impl Decode for PairedSignature {
	fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
		let mut bytes = [0u8; PAIRED_LEN];
		input.read(&mut bytes)?;
		Ok(PairedSignature(bytes))
	}
}

impl PairedSignature {
	/// The BLS12-381 G1 signature half.
	pub fn g1_signature(&self) -> [u8; BLS_G1_SIGNATURE_LEN] {
		let mut g1 = [0u8; BLS_G1_SIGNATURE_LEN];
		g1.copy_from_slice(
			&self.0[PAIRED_SIGNATURE_G1_OFFSET..PAIRED_SIGNATURE_G1_OFFSET + BLS_G1_SIGNATURE_LEN],
		);
		g1
	}
}

/// Decode a BEEFY justification keeping the whole paired signature.
pub fn decode_paired_justification(
	bytes: &[u8],
) -> Result<SignedCommitment<u32, PairedSignature>, anyhow::Error> {
	let VersionedFinalityProof::V1(signed_commitment) =
		VersionedFinalityProof::<u32, PairedSignature>::decode(&mut &*bytes)?;
	Ok(signed_commitment)
}

/// The justification at `at`, with both halves of each paired signature kept.
///
/// `crate::relay::fetch_latest_beefy_justification` reads the same bytes but discards the BLS
/// half, which is the half this path needs.
pub async fn fetch_paired_justification<T: Config>(
	rpc: &LegacyRpcMethods<T>,
	at: HashFor<T>,
) -> Result<SignedCommitment<u32, PairedSignature>, anyhow::Error> {
	let block = rpc
		.chain_get_block(Some(at))
		.await?
		.ok_or_else(|| anyhow!("No block at {at:?}"))?;

	let justification = block
		.justifications
		.and_then(|justifications| {
			justifications
				.into_iter()
				.find_map(|(id, encoded)| (id == BEEFY_ENGINE_ID).then_some(encoded))
		})
		.ok_or_else(|| anyhow!("Block {at:?} carries no beefy justification"))?;

	decode_paired_justification(&justification)
}

/// The validators' BLS12-381 G2 public keys, in authority-set order.
pub async fn beefy_g2_authorities<T: Config>(
	rpc: &LegacyRpcMethods<T>,
	at: Option<HashFor<T>>,
) -> Result<Vec<[u8; BLS_G2_PUBLIC_KEY_LEN]>, anyhow::Error> {
	let data = rpc
		.state_get_storage(BEEFY_AUTHORITIES.as_slice(), at)
		.await?
		.ok_or_else(|| anyhow!("No beefy authorities found!"))?;

	Ok(Vec::<PairedAuthority>::decode(&mut data.as_ref())?
		.iter()
		.map(PairedAuthority::g2)
		.collect())
}

/// The validators' BLS12-381 G1 public keys, in authority-set order.
///
/// `DoublePublicKey` publishes the same secret in both groups, so these are the G1 counterparts of
/// [`beefy_g2_authorities`]. An APK proof consumes the G1 halves while BEEFY's own signature
/// verifies against the G2 halves.
pub async fn beefy_g1_authorities<T: Config>(
	rpc: &LegacyRpcMethods<T>,
	at: Option<HashFor<T>>,
) -> Result<Vec<[u8; BLS_G1_SIGNATURE_LEN]>, anyhow::Error> {
	let data = rpc
		.state_get_storage(BEEFY_AUTHORITIES.as_slice(), at)
		.await?
		.ok_or_else(|| anyhow!("No beefy authorities found!"))?;

	Ok(Vec::<PairedAuthority>::decode(&mut data.as_ref())?
		.iter()
		.map(PairedAuthority::g1)
		.collect())
}

/// Sum compressed G1 signatures into the single compressed G1 point the verifier checks.
pub fn aggregate_signatures(
	signatures: &[[u8; BLS_G1_SIGNATURE_LEN]],
) -> Result<[u8; BLS_G1_SIGNATURE_LEN], anyhow::Error> {
	use ark_ff::Zero;
	use w3f_bls::{EngineBLS, SerializableToBytes, Signature, TinyBLS381};

	// Aggregating nothing would give the identity, which is a well formed point and a meaningless
	// signature, so an empty set is refused rather than summed.
	if signatures.is_empty() {
		Err(anyhow!("No signatures to aggregate"))?
	}

	let aggregate = signatures.iter().try_fold(
		<TinyBLS381 as EngineBLS>::SignatureGroup::zero(),
		|sum, signature| {
			let signature = Signature::<TinyBLS381>::from_bytes(signature)
				.map_err(|_| anyhow!("Invalid G1 signature encoding"))?;
			Ok::<_, anyhow::Error>(sum + signature.0)
		},
	)?;
	Signature::<TinyBLS381>(aggregate)
		.to_bytes()
		.try_into()
		.map_err(|_| anyhow!("Aggregated signature was not {BLS_G1_SIGNATURE_LEN} bytes"))
}

impl<R: Config, P: Config> Prover<R, P> {
	/// Collect the relay chain half of a BLS BEEFY update: the signed commitment, the aggregate
	/// signature, the MMR leaf and its proof, and the parachain headers.
	///
	/// Which validators signed is left to the caller, which reads it from the justification's
	/// bitfield and proves it with an APK proof. Nothing here grows with the number of signers.
	pub async fn bls_consensus_proof(
		&self,
		signed_commitment: SignedCommitment<u32, PairedSignature>,
	) -> Result<BlsConsensusMessage, anyhow::Error> {
		let block_number: u32 = signed_commitment.commitment.block_number;
		let (mmr_proof, latest_leaf) =
			fetch_mmr_proof(&self.relay_rpc, block_number, self.query_batch_size).await?;

		// Only the signatures are needed here. The keys they belong to are the APK proof's
		// business, and reading them is the caller's.
		let g1_signatures = signed_commitment
			.signatures
			.iter()
			.flatten()
			.map(|signature| signature.g1_signature())
			.collect::<Vec<_>>();
		let aggregate_signature = aggregate_signatures(&g1_signatures)?;

		let mmr = BlsMmrProof {
			commitment: signed_commitment.commitment.clone(),
			aggregate_signature,
			latest_mmr_leaf: latest_leaf.clone(),
			mmr_proof,
		};

		let heads = paras_parachains(
			&self.relay_rpc,
			Some(HashFor::<R>::decode(&mut &*latest_leaf.parent_number_and_hash.1.encode())?),
		)
		.await?;

		let parachain = build_parachain_proof(&self.para_ids, &heads);

		Ok(BlsConsensusMessage { mmr, parachain })
	}
}
