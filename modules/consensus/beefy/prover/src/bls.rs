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
use sp_consensus_beefy::{SignedCommitment, VersionedFinalityProof};
use sp_io::hashing::keccak_256;
use subxt::{backend::legacy::LegacyRpcMethods, Config};
use subxt_core::config::HashFor;

use beefy_verifier_primitives::{
	BlsConsensusMessage, BlsMmrProof, BlsSigner, BLS_G1_SIGNATURE_LEN, BLS_G2_PUBLIC_KEY_LEN,
};

use crate::{
	build_parachain_proof,
	relay::{fetch_mmr_proof, paras_parachains},
	Prover, BEEFY_AUTHORITIES,
};

/// Wire size of a paired (ECDSA, BLS12-381) BEEFY key or signature.
pub const PAIRED_LEN: usize = 177;

/// Offset of the BLS G1 signature within a paired signature: the ECDSA half is 65 bytes, then the
/// `DoubleSignature` begins with its 48-byte G1 point.
const PAIRED_SIGNATURE_G1_OFFSET: usize = 65;

/// Offset of the BLS G2 public key within a paired public key: the ECDSA half is 33 bytes, then
/// the `DoublePublicKey` is `G1 (48) || G2 (96)`, so G2 starts 48 bytes further in.
const PAIRED_PUBLIC_G2_OFFSET: usize = 33 + 48;

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

/// The validators' BLS12-381 G2 public keys, in authority-set order.
pub async fn beefy_g2_authorities<T: Config>(
	rpc: &LegacyRpcMethods<T>,
	at: Option<HashFor<T>>,
) -> Result<Vec<[u8; BLS_G2_PUBLIC_KEY_LEN]>, anyhow::Error> {
	let data = rpc
		.state_get_storage(BEEFY_AUTHORITIES.as_slice(), at)
		.await?
		.ok_or_else(|| anyhow!("No beefy authorities found!"))?;

	let paired = Vec::<[u8; PAIRED_LEN]>::decode(&mut data.as_ref())?;

	Ok(paired
		.into_iter()
		.map(|key| {
			let mut g2 = [0u8; BLS_G2_PUBLIC_KEY_LEN];
			g2.copy_from_slice(
				&key[PAIRED_PUBLIC_G2_OFFSET..PAIRED_PUBLIC_G2_OFFSET + BLS_G2_PUBLIC_KEY_LEN],
			);
			g2
		})
		.collect())
}

/// Sum compressed G1 signatures into the single compressed G1 point the verifier checks.
pub fn aggregate_signatures(
	signatures: &[[u8; BLS_G1_SIGNATURE_LEN]],
) -> Result<[u8; BLS_G1_SIGNATURE_LEN], anyhow::Error> {
	use w3f_bls::{EngineBLS, SerializableToBytes, Signature, TinyBLS381};

	let mut aggregate: Option<<TinyBLS381 as EngineBLS>::SignatureGroup> = None;
	for signature in signatures {
		let signature = Signature::<TinyBLS381>::from_bytes(signature)
			.map_err(|_| anyhow!("Invalid G1 signature encoding"))?;
		aggregate = Some(aggregate.map_or(signature.0, |sum| sum + signature.0));
	}

	let aggregate = aggregate.ok_or_else(|| anyhow!("No signatures to aggregate"))?;
	Signature::<TinyBLS381>(aggregate)
		.to_bytes()
		.try_into()
		.map_err(|_| anyhow!("Aggregated signature was not {BLS_G1_SIGNATURE_LEN} bytes"))
}

impl<R: Config, P: Config> Prover<R, P> {
	/// Build a consensus proof whose commitment is proven by one aggregate BLS signature.
	///
	/// The signers' G2 public keys travel with the proof, since the verifier needs them both to
	/// form the aggregate key and to prove membership of the authority set. That makes the proof
	/// grow with the number of signers, which is the cost of not needing a SNARK.
	pub async fn bls_consensus_proof(
		&self,
		signed_commitment: SignedCommitment<u32, PairedSignature>,
	) -> Result<BlsConsensusMessage, anyhow::Error> {
		let block_number: u32 = signed_commitment.commitment.block_number;
		let block_hash = self
			.relay_rpc
			.chain_get_block_hash(Some(block_number.into()))
			.await?
			.ok_or_else(|| anyhow!("Failed to query blockhash for blocknumber"))?;

		let (mmr_proof, latest_leaf) =
			fetch_mmr_proof(&self.relay_rpc, block_number, self.query_batch_size).await?;

		let authorities = beefy_g2_authorities(&self.relay_rpc, Some(block_hash)).await?;

		// Signers in authority-set order, which is what the merkle multi-proof expects and what
		// the verifier enforces.
		let mut signers = Vec::new();
		let mut g1_signatures = Vec::new();
		for (index, maybe_signature) in signed_commitment.signatures.iter().enumerate() {
			let Some(signature) = maybe_signature else { continue };
			let public_key = *authorities
				.get(index)
				.ok_or_else(|| anyhow!("Signature index {index} outside the authority set"))?;

			signers.push(BlsSigner { public_key, index: index as u32 });
			g1_signatures.push(signature.g1_signature());
		}

		let aggregate_signature = aggregate_signatures(&g1_signatures)?;

		// The keyset commitment is a merkle root over the hashed G2 keys, so the tree is built
		// over every authority and opened at the signers' positions.
		let leaves = authorities.iter().map(|key| keccak_256(key)).collect::<Vec<_>>();
		let indices = signers.iter().map(|signer| signer.index as usize).collect::<Vec<_>>();
		let tree = rs_merkle::MerkleTree::<crate::util::MerkleHasher>::from_leaves(&leaves);
		let authority_proof = tree.proof(&indices).proof_hashes().to_vec();

		let mmr = BlsMmrProof {
			commitment: signed_commitment.commitment.clone(),
			signers,
			aggregate_signature,
			latest_mmr_leaf: latest_leaf.clone(),
			mmr_proof,
			authority_proof,
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

/// Building an EVM-bound proof.
///
/// The SCALE proof carries compressed points, which is what the Rust verifier and the keyset
/// commitment work on. EIP-2537 accepts only uncompressed ones, so an EVM submission has to
/// decompress. That needs curve arithmetic, which is why it lives here rather than in `ismp-abi`,
/// a crate that compiles into the runtime. The contract compresses again to rebuild the merkle
/// leaves, which is cheap in that direction.
pub mod abi {
	use anyhow::anyhow;
	use ark_bls12_381::{G1Affine, G2Affine};
	use ark_ec::AffineRepr;
	use ark_ff::{BigInteger, PrimeField};
	use ark_serialize::CanonicalDeserialize;
	use beefy_verifier_primitives::{
		BlsConsensusMessage, BLS_G1_SIGNATURE_LEN, BLS_G1_UNCOMPRESSED_LEN, BLS_G2_PUBLIC_KEY_LEN,
		BLS_G2_UNCOMPRESSED_LEN,
	};
	use ismp_abi::bls_beefy::BlsBeefy;

	/// Expand a compressed G2 public key into the EIP-2537 encoding: four 64 byte field elements,
	/// each a 48 byte big-endian value with 16 bytes of leading zeroes.
	pub fn decompress_g2(
		compressed: &[u8; BLS_G2_PUBLIC_KEY_LEN],
	) -> Result<[u8; BLS_G2_UNCOMPRESSED_LEN], anyhow::Error> {
		let point = G2Affine::deserialize_compressed(&compressed[..])
			.map_err(|e| anyhow!("invalid compressed G2 point: {e:?}"))?;
		let (x, y) = point.xy().ok_or_else(|| anyhow!("G2 point is the identity"))?;

		let mut out = [0u8; BLS_G2_UNCOMPRESSED_LEN];
		for (slot, coord) in [&x.c0, &x.c1, &y.c0, &y.c1].iter().enumerate() {
			let bytes = coord.into_bigint().to_bytes_be();
			out[slot * 64 + 16..slot * 64 + 64].copy_from_slice(&bytes);
		}

		Ok(out)
	}

	/// Expand a compressed G1 signature into the EIP-2537 encoding.
	pub fn decompress_g1(
		compressed: &[u8; BLS_G1_SIGNATURE_LEN],
	) -> Result<[u8; BLS_G1_UNCOMPRESSED_LEN], anyhow::Error> {
		let point = G1Affine::deserialize_compressed(&compressed[..])
			.map_err(|e| anyhow!("invalid compressed G1 point: {e:?}"))?;
		let (x, y) = point.xy().ok_or_else(|| anyhow!("G1 point is the identity"))?;

		let mut out = [0u8; BLS_G1_UNCOMPRESSED_LEN];
		for (slot, coord) in [x, y].iter().enumerate() {
			let bytes = coord.into_bigint().to_bytes_be();
			out[slot * 64 + 16..slot * 64 + 64].copy_from_slice(&bytes);
		}

		Ok(out)
	}

	/// Convert a consensus proof into the ABI shape the Solidity client consumes.
	pub fn to_abi_proof(
		message: BlsConsensusMessage,
	) -> Result<BlsBeefy::BlsBeefyConsensusProof, anyhow::Error> {
		use alloy_primitives::{Bytes, FixedBytes, U256};

		let mmr = message.mmr;
		let leaf_index = mmr.mmr_proof.leaf_indices.first().copied().unwrap_or_default();

		let signers = mmr
			.signers
			.iter()
			.map(|signer| {
				Ok(BlsBeefy::BlsSigner {
					publicKey: Bytes::from(decompress_g2(&signer.public_key)?.to_vec()),
					authorityIndex: U256::from(signer.index),
				})
			})
			.collect::<Result<Vec<_>, anyhow::Error>>()?;

		let relay = BlsBeefy::BlsRelayChainProof {
			commitment: BlsBeefy::Commitment {
				payload: vec![BlsBeefy::Payload {
					id: FixedBytes(*b"mh"),
					data: Bytes::from(
						mmr.commitment
							.payload
							.get_raw(b"mh")
							.ok_or_else(|| anyhow!("mmr payload not present"))?
							.clone(),
					),
				}],
				blockNumber: mmr.commitment.block_number,
				validatorSetId: mmr.commitment.validator_set_id,
			},
			signers,
			aggregateSignature: Bytes::from(decompress_g1(&mmr.aggregate_signature)?.to_vec()),
			latestMmrLeaf: BlsBeefy::BeefyMmrLeaf {
				version: 0,
				parentNumber: mmr.latest_mmr_leaf.parent_number_and_hash.0,
				parentHash: FixedBytes(mmr.latest_mmr_leaf.parent_number_and_hash.1 .0),
				nextAuthoritySet: BlsBeefy::AuthoritySetCommitment {
					id: mmr.latest_mmr_leaf.beefy_next_authority_set.id,
					len: mmr.latest_mmr_leaf.beefy_next_authority_set.len,
					root: FixedBytes(
						mmr.latest_mmr_leaf.beefy_next_authority_set.keyset_commitment.0,
					),
				},
				extra: FixedBytes(mmr.latest_mmr_leaf.leaf_extra.0),
				leafIndex: U256::from(leaf_index),
			},
			mmrProof: mmr.mmr_proof.items.iter().map(|h| FixedBytes(h.0)).collect(),
			proof: mmr.authority_proof.iter().map(|h| FixedBytes(*h)).collect(),
		};

		let parachain = BlsBeefy::ParachainProof {
			parachains: message
				.parachain
				.parachains
				.iter()
				.map(|para| BlsBeefy::Parachain {
					index: U256::from(para.index),
					id: U256::from(para.para_id),
					header: Bytes::from(para.header.clone()),
				})
				.collect(),
			proof: message.parachain.proof.iter().map(|h| FixedBytes(*h)).collect(),
			leafCount: U256::from(message.parachain.total_leaves),
		};

		Ok(BlsBeefy::BlsBeefyConsensusProof { relay, parachain })
	}
}
