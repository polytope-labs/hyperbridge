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

//! Verifying BEEFY finality through an aggregate public key proof.
//!
//! This is the runtime counterpart of `BlsApkBeefy.sol` and the two have to agree exactly, since
//! they check the same proofs. The SNARK establishes that an aggregate key is the sum of precisely
//! the validators named in a bitlist, drawn from the set a commitment describes, and a single
//! pairing then checks the aggregate signature against that key while binding its G2 counterpart.
//!
//! Nothing here grows with the number of signers, which is the whole point of the scheme.

use alloc::vec::Vec;

use ark_bls12_381::{Bls12_381, Fq, Fq2, Fr, G1Affine, G1Projective, G2Affine};
use ark_ec::{AffineRepr, CurveGroup, PrimeGroup, pairing::Pairing};
use ark_ff::{BigInteger, One, PrimeField, Zero};
use ark_serialize::CanonicalDeserialize;
use beefy_verifier_primitives::{
	APK_BITLIST_WORDS, APK_G1_LEN, APK_G2_LEN, ApkAuthoritySet, ApkCommitmentDigest,
	ApkConsensusMessage, ApkConsensusState, ApkMmrProof, ParachainHeader,
};
use codec::{Decode, Encode};
use polkadot_sdk::*;
use primitive_types::H256;
use sp_runtime::traits::BlakeTwo256;

use crate::Keccak256;
use sha2::{Digest, Sha256};

use crate::error::Error;

/// Half of a G1 point, and the width of every coordinate the circuit's verifier deals in.
const COORDINATE: usize = 48;

/// Verify a whole update and return the new trusted state with the verified parachain headers.
///
/// The order matters and mirrors the Solidity client. The commitment is only believed once the
/// aggregate proof and the pairing both pass, the mmr leaf is only believed once the commitment
/// is, and the headers only once the leaf is. Any commitment picked up from those headers is
/// therefore learned from something already proven.
pub fn verify_apk_consensus<H: Keccak256 + Send + Sync>(
	trusted_state: ApkConsensusState,
	proof: ApkConsensusMessage,
	verifying_key: &[u8],
) -> Result<(ApkConsensusState, Vec<ParachainHeader>), Error> {
	let (mut state, heads_root) =
		verify_apk_mmr_update_proof::<H>(trusted_state, proof.mmr, verifying_key)?;
	let headers = crate::verify_parachain_headers::<H>(heads_root, proof.parachain)?;

	// Forward chaining: a verified header may carry the commitment for a set this client has no
	// keys for yet. Picking it up here is what makes the next update verifiable at all, and is
	// why the digest names the next set rather than the current one.
	for header in headers.iter() {
		if let Some(digest) = read_apk_digest(&header.header) {
			state.learn_commitment(digest.set_id, H256(digest.commitment));
			break;
		}
	}

	Ok((state, headers))
}

/// Verify the signed mmr root and roll the authority sets forward.
pub fn verify_apk_mmr_update_proof<H: Keccak256 + Send + Sync>(
	mut trusted_state: ApkConsensusState,
	mmr: ApkMmrProof,
	verifying_key: &[u8],
) -> Result<(ApkConsensusState, H256), Error> {
	if trusted_state.latest_beefy_height >= mmr.commitment.block_number {
		return Err(Error::StaleHeight {
			trusted_height: trusted_state.latest_beefy_height,
			current_height: mmr.commitment.block_number,
		});
	}

	let set_id = mmr.commitment.validator_set_id;
	let authority_set = if set_id == trusted_state.current_authorities.id {
		&trusted_state.current_authorities
	} else if set_id == trusted_state.next_authorities.id {
		&trusted_state.next_authorities
	} else {
		return Err(Error::UnknownAuthoritySet { id: set_id });
	};

	// A set whose commitment has not been learned from a digest yet cannot be verified against.
	// Refusing is deliberate: proceeding would check the proof against a zero commitment, which
	// establishes nothing at all.
	if authority_set.apk_commitment.is_zero() {
		return Err(Error::ApkCommitmentMissing { id: set_id });
	}
	let apk_commitment = authority_set.apk_commitment;
	let authority_count = authority_set.len;

	verify_signed_by_apk(
		&mmr.commitment.encode(),
		&mmr,
		apk_commitment,
		authority_count,
		verifying_key,
	)?;

	let mmr_root = mmr
		.commitment
		.payload
		.get_raw(&sp_consensus_beefy::known_payloads::MMR_ROOT_ID)
		.and_then(|raw| H256::decode(&mut &raw[..]).ok())
		.ok_or(Error::MmrRootHashMissing)?;

	crate::verify_mmr_leaf::<H>(&mmr.latest_mmr_leaf, &mmr.mmr_proof, mmr_root)?;

	// The leaf names the incoming set and its size, but says nothing about Poseidon2, so its
	// commitment starts empty and waits for a digest.
	let next = &mmr.latest_mmr_leaf.beefy_next_authority_set;
	if next.id > trusted_state.next_authorities.id {
		trusted_state.current_authorities = trusted_state.next_authorities.clone();
		trusted_state.next_authorities =
			ApkAuthoritySet { id: next.id, len: next.len, apk_commitment: H256::zero() };
	}
	trusted_state.latest_beefy_height = mmr.commitment.block_number;
	trusted_state.mmr_root_hash = mmr_root;

	Ok((trusted_state, mmr.latest_mmr_leaf.leaf_extra))
}

/// Read an APK commitment out of a scale encoded parachain header, if it carries one.
fn read_apk_digest(header: &[u8]) -> Option<ApkCommitmentDigest> {
	let header = sp_runtime::generic::Header::<u32, BlakeTwo256>::decode(&mut &header[..]).ok()?;
	ApkCommitmentDigest::find_in(&header.digest)
}

/// Verify that a supermajority of `authority_set` signed `commitment`.
///
/// Two separate facts are established and both are needed. The proof says the aggregate key
/// belongs to the set, and the pairing says that key produced the signature. Counting the
/// signers is left here, because the circuit proves who signed but has no opinion on whether it
/// is enough.
pub fn verify_signed_by_apk(
	encoded_commitment: &[u8],
	mmr: &ApkMmrProof,
	apk_commitment: H256,
	authority_count: u32,
	verifying_key: &[u8],
) -> Result<(), Error> {
	if !supermajority(count_signers(&mmr.bitlist), authority_count) {
		return Err(Error::SuperMajorityRequired);
	}

	verify_apk_proof(mmr, apk_commitment, verifying_key)?;

	let message = hash_commitment_to_g1(encoded_commitment)?;
	let apk = read_g1(&mmr.apk)?;
	let apk2 = read_g2(&mmr.apk2)?;
	let signature = read_g1(&mmr.signature)?;

	if !verify_aggregate(message, apk, apk2, signature) {
		return Err(Error::ApkPairingFailed);
	}

	Ok(())
}

/// Check the PLONK proof against the set's commitment.
///
/// The public inputs are the bitlist, then the commitment, then the aggregate key as twelve
/// limbs, which is the order the circuit exposes them in and the order the Solidity verifier
/// reads them.
fn verify_apk_proof(
	mmr: &ApkMmrProof,
	apk_commitment: H256,
	verifying_key: &[u8],
) -> Result<(), Error> {
	let vk = gnark_plonk_verifier::VerifyingKey::try_from(verifying_key)
		.map_err(|_| Error::ApkVerifyingKeyInvalid)?;
	let proof = gnark_plonk_verifier::PlonkProof::try_from((&mmr.apk_proof[..], vk.qcp.len()))
		.map_err(|_| Error::ApkProofMalformed)?;

	let mut public_inputs = Vec::with_capacity(APK_BITLIST_WORDS + 1 + 12);
	for word in mmr.bitlist.iter() {
		public_inputs.push(Fr::from_be_bytes_mod_order(word));
	}
	public_inputs.push(Fr::from_be_bytes_mod_order(apk_commitment.as_bytes()));
	// The aggregate key travels as public input too, as six limbs per coordinate, which is how
	// the circuit represents a base field element it cannot hold in one scalar.
	for coordinate in [&mmr.apk[..COORDINATE], &mmr.apk[COORDINATE..]] {
		for limb in coordinate.chunks(8) {
			public_inputs.push(Fr::from_be_bytes_mod_order(limb));
		}
	}

	gnark_plonk_verifier::verify(&proof, &vk, &public_inputs)
		.map_err(|_| Error::ApkProofVerificationFailed)
}

/// The batched signature and binding check, matching `ApkProof._verifyBls`:
///
/// ```text
///   e(sig + t·apk, -g2) · e(msg + t·g1, apk2) == 1
/// ```
///
/// which is the random linear combination of the signature equation and the equation binding
/// `apk2` to `apk`. Doing them together costs one pairing rather than two, and the challenge `t`
/// is derived from all four points so a caller cannot pick points to suit a known `t`.
fn verify_aggregate(message: G1Affine, apk: G1Affine, apk2: G2Affine, signature: G1Affine) -> bool {
	let t = challenge(&message, &signature, &apk, &apk2);

	let lhs: G1Affine = (signature.into_group() + apk * t).into_affine();
	let rhs: G1Affine = (message.into_group() + G1Projective::generator() * t).into_affine();

	let result = Bls12_381::multi_pairing([lhs, rhs], [-G2Affine::generator(), apk2]);
	result.0 == ark_bls12_381::Fq12::one()
}

/// `expand_message_xmd` with SHA-256 over the four points, taking 48 bytes and reducing them into
/// the scalar field. An empty domain separation tag, matching the contract.
fn challenge(message: &G1Affine, signature: &G1Affine, apk: &G1Affine, apk2: &G2Affine) -> Fr {
	let mut data = Vec::with_capacity(APK_G1_LEN * 3 + APK_G2_LEN);
	data.extend_from_slice(&write_g1(message));
	data.extend_from_slice(&write_g1(signature));
	data.extend_from_slice(&write_g1(apk));
	data.extend_from_slice(&write_g2(apk2));

	// b0 = SHA256(Z_pad || data || I2OSP(48, 2) || I2OSP(0, 1) || DST_prime), with a 64 byte zero
	// pad for SHA-256's block size and an empty DST, so DST_prime is a single zero length byte.
	let mut hasher = Sha256::new();
	hasher.update([0u8; 64]);
	hasher.update(&data);
	hasher.update([0x00, 0x30]);
	hasher.update([0x00]);
	hasher.update([0x00]);
	let b0: [u8; 32] = hasher.finalize().into();

	let mut hasher = Sha256::new();
	hasher.update(b0);
	hasher.update([0x01, 0x00]);
	let b1: [u8; 32] = hasher.finalize().into();

	let mut xored = [0u8; 32];
	for (i, byte) in xored.iter_mut().enumerate() {
		*byte = b0[i] ^ b1[i];
	}
	let mut hasher = Sha256::new();
	hasher.update(xored);
	hasher.update([0x02, 0x00]);
	let b2: [u8; 32] = hasher.finalize().into();

	// t = b1 ‖ the top 16 bytes of b2, reduced. 48 bytes of uniform output, as the spec asks for.
	let mut wide = [0u8; 48];
	wide[..32].copy_from_slice(&b1);
	wide[32..].copy_from_slice(&b2[..16]);
	Fr::from_be_bytes_mod_order(&wide)
}

/// Population count over the bitlist. Fixed cost regardless of how many signed.
pub fn count_signers(bitlist: &[[u8; 32]; APK_BITLIST_WORDS]) -> u32 {
	bitlist.iter().flatten().map(|byte| byte.count_ones()).sum()
}

/// Two thirds plus one, matching substrate's own rule and the Solidity client.
fn supermajority(signed: u32, total: u32) -> bool {
	total > 0 && (signed as u64) * 3 > (total as u64) * 2
}

/// Hash the encoded commitment onto G1 the way `w3f-bls` does.
///
/// Note this is not the textbook construction. The ciphersuite is prepended to the message rather
/// than used as the domain separation tag, and the tag itself is the single byte `0x01`. Getting
/// this wrong yields a well formed point and a pairing that fails with nothing to explain why.
fn hash_commitment_to_g1(encoded_commitment: &[u8]) -> Result<G1Affine, Error> {
	use ark_ec::hashing::{
		HashToCurve, curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher,
	};
	use ark_ff::field_hashers::DefaultFieldHasher;

	let mut preimage = Vec::with_capacity(CIPHER_SUITE.len() + encoded_commitment.len());
	preimage.extend_from_slice(CIPHER_SUITE);
	preimage.extend_from_slice(encoded_commitment);

	let hasher = MapToCurveBasedHasher::<
		G1Projective,
		DefaultFieldHasher<Sha256, 128>,
		WBMap<ark_bls12_381::g1::Config>,
	>::new(&[0x01])
	.map_err(|_| Error::ApkHashToCurveFailed)?;

	hasher.hash(&preimage).map_err(|_| Error::ApkHashToCurveFailed)
}

/// The ciphersuite `w3f-bls` prepends to the message. Substrate signs with the basic scheme, so
/// the tail is `NUL` rather than `POP`.
const CIPHER_SUITE: &[u8] = b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_";

/// Read a G1 point from raw big-endian coordinates.
fn read_g1(bytes: &[u8; APK_G1_LEN]) -> Result<G1Affine, Error> {
	let x = read_fq(&bytes[..COORDINATE])?;
	let y = read_fq(&bytes[COORDINATE..])?;
	let point = G1Affine::new_unchecked(x, y);
	if !point.is_on_curve() || !point.is_in_correct_subgroup_assuming_on_curve() {
		return Err(Error::ApkPointInvalid);
	}
	Ok(point)
}

/// Read a G2 point, whose coordinates are pairs in the quadratic extension.
fn read_g2(bytes: &[u8; APK_G2_LEN]) -> Result<G2Affine, Error> {
	let x = Fq2::new(read_fq(&bytes[..COORDINATE])?, read_fq(&bytes[COORDINATE..2 * COORDINATE])?);
	let y = Fq2::new(
		read_fq(&bytes[2 * COORDINATE..3 * COORDINATE])?,
		read_fq(&bytes[3 * COORDINATE..])?,
	);
	let point = G2Affine::new_unchecked(x, y);
	if !point.is_on_curve() || !point.is_in_correct_subgroup_assuming_on_curve() {
		return Err(Error::ApkPointInvalid);
	}
	Ok(point)
}

fn read_fq(bytes: &[u8]) -> Result<Fq, Error> {
	Fq::deserialize_uncompressed(&reverse(bytes)[..]).map_err(|_| Error::ApkPointInvalid)
}

/// Arkworks serialises field elements little-endian while the circuit and the contract both work
/// big-endian, so every coordinate is reversed on the way in and out.
fn reverse(bytes: &[u8]) -> Vec<u8> {
	bytes.iter().rev().copied().collect()
}

fn write_g1(point: &G1Affine) -> Vec<u8> {
	let (x, y) = point.xy().unwrap_or((Fq::zero(), Fq::zero()));
	let mut out = Vec::with_capacity(APK_G1_LEN);
	out.extend_from_slice(&x.into_bigint().to_bytes_be());
	out.extend_from_slice(&y.into_bigint().to_bytes_be());
	out
}

fn write_g2(point: &G2Affine) -> Vec<u8> {
	let (x, y) = point.xy().unwrap_or((Fq2::zero(), Fq2::zero()));
	let mut out = Vec::with_capacity(APK_G2_LEN);
	for coordinate in [&x.c0, &x.c1, &y.c0, &y.c1] {
		out.extend_from_slice(&coordinate.into_bigint().to_bytes_be());
	}
	out
}
