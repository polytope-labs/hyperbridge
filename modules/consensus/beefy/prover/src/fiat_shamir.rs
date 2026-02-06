// Copyright (C) 2022 Polytope Labs.
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

//! Fiat-Shamir transcript implementation for deterministic validator sampling
//! using a signers bitmap.
//!
//! This module mirrors the Solidity `Transcript.sol` and `BeefyV1FiatShamir.sol`
//! contracts exactly, so that the prover and on-chain verifier derive identical
//! challenged authority indices from the same inputs.
//!
//! # Protocol
//!
//! 1. The prover observes the BEEFY signed commitment and constructs a **signers bitmap** — a
//!    4×uint256 (1024-bit) bitfield where bit `i` is set iff authority `i` signed the commitment.
//!
//! 2. Both prover and verifier build an identical Fiat-Shamir transcript seeded with the commitment
//!    hash, the authority set commitment (root + length), and the signers bitmap.
//!
//! 3. From the transcript, [`SAMPLE_SIZE`] unique indices in `[0, signer_count)` are derived, then
//!    mapped to actual authority indices via the bitmap (the n-th set bit is the n-th signer).
//!
//! 4. The proof contains only those [`SAMPLE_SIZE`] signatures and a merkle multi-proof of their
//!    membership in the authority set.

use anyhow::anyhow;
use codec::Encode;
use polkadot_sdk::*;
use primitive_types::{H256, U256};
use sp_consensus_beefy::ecdsa_crypto::Signature;
use sp_io::hashing::keccak_256;

use beefy_verifier_primitives::SignatureWithAuthorityIndex;

/// Number of validator signatures sampled and verified on-chain.
/// Must match `SAMPLE_SIZE` in `BeefyV1FiatShamir.sol`.
pub const SAMPLE_SIZE: usize = 10;

/// Domain separator for the Fiat-Shamir transcript.
/// Must match `TRANSCRIPT_DOMAIN` in `BeefyV1FiatShamir.sol`.
pub const TRANSCRIPT_DOMAIN: &[u8] = b"BEEFY_FIAT_SHAMIR_V1";

/// Number of uint256 words in the signers bitmap (4 × 256 = 1024 bits).
/// Must match `BITMAP_WORDS` in `BeefyV1FiatShamir.sol`.
pub const BITMAP_WORDS: usize = 4;

/// Maximum number of validators supported by the bitmap.
pub const MAX_VALIDATORS: usize = BITMAP_WORDS * 256;

/// The 8-byte squeeze label used by the Solidity transcript.
/// In Solidity: `bytes8("squeeze")` is 7 ASCII bytes + 1 trailing zero byte.
const SQUEEZE_LABEL: [u8; 8] = *b"squeeze\0";

// ─────────────────────────────────────────────────
//  Signers Bitmap
// ─────────────────────────────────────────────────

/// A 1024-bit signers bitmap stored as 4 × U256 words.
///
/// Bit `i` is stored in `words[i / 256]` at position `i % 256`.
/// This mirrors the Solidity `uint256[4]` representation exactly.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SignersBitmap {
	/// The 4 uint256 words, indexed identically to Solidity's `uint256[4]`.
	pub words: [U256; BITMAP_WORDS],
}

impl SignersBitmap {
	/// Constructs a bitmap from the signatures in a BEEFY signed commitment.
	///
	/// Bit `i` is set iff `signed_commitment.signatures[i]` is `Some`.
	pub fn from_signed_commitment(
		signed_commitment: &sp_consensus_beefy::SignedCommitment<u32, Signature>,
	) -> Self {
		let mut bitmap = Self::default();
		for (i, sig) in signed_commitment.signatures.iter().enumerate() {
			if sig.is_some() {
				bitmap.set_bit(i);
			}
		}
		bitmap
	}

	/// Sets bit `index` in the bitmap.
	pub fn set_bit(&mut self, index: usize) {
		let word = index >> 8; // index / 256
		let bit = index & 0xFF; // index % 256
		if word < BITMAP_WORDS {
			self.words[word] = self.words[word] | (U256::one() << bit);
		}
	}

	/// Returns true if bit `index` is set.
	pub fn is_bit_set(&self, index: usize) -> bool {
		let word = index >> 8;
		let bit = index & 0xFF;
		if word >= BITMAP_WORDS {
			return false;
		}
		(self.words[word] & (U256::one() << bit)) != U256::zero()
	}

	/// Counts the number of set bits in positions `[0, authority_set_len)`.
	/// Uses byte-level `count_ones()` for O(1)-per-word performance.
	pub fn count_set_bits(&self, authority_set_len: u32) -> u32 {
		let mut count = 0u32;
		for w in 0..BITMAP_WORDS {
			let remaining = if authority_set_len > (w as u32) * 256 {
				authority_set_len - (w as u32) * 256
			} else {
				0
			};
			if remaining == 0 {
				break;
			}

			let mut word = self.words[w];
			if remaining < 256 {
				// Mask off bits beyond authority_set_len
				word = word & ((U256::one() << remaining as usize) - U256::one());
			}
			// Sum popcount of each byte in the 32-byte big-endian representation
			count += word.to_big_endian().iter().map(|b| b.count_ones()).sum::<u32>();
		}
		count
	}

	/// Enumerates all set bit positions in the bitmap in a single pass,
	/// returning a `Vec` of authority indices for every signer.
	/// Only considers positions `[0, authority_set_len)`.
	pub fn enumerate_signers(&self, authority_set_len: u32) -> Vec<u32> {
		let mut signers = Vec::new();
		for i in 0..(authority_set_len as usize) {
			if self.is_bit_set(i) {
				signers.push(i as u32);
			}
		}
		signers
	}
}

// ─────────────────────────────────────────────────
//  Fiat-Shamir Transcript
// ─────────────────────────────────────────────────

/// A Fiat-Shamir transcript that mirrors the Solidity `Transcript` library exactly.
///
/// The transcript maintains a running keccak256 hash state. Data is absorbed by
/// concatenating it with the current state and re-hashing. Challenges are squeezed
/// out by hashing the state with a fixed label, then advancing the state.
///
/// **Important:** Every operation here must produce byte-identical intermediate
/// hashes to the Solidity implementation for the prover and verifier to agree on
/// the challenged indices.
#[derive(Clone, Debug)]
pub struct Transcript {
	hash: [u8; 32],
}

impl Transcript {
	/// Initialize a new transcript with a domain separator.
	///
	/// Mirrors: `Transcript.init(domainSeparator)` in Solidity.
	pub fn init(domain_separator: &[u8]) -> Self {
		Self { hash: keccak_256(domain_separator) }
	}

	/// Absorb a 32-byte value into the transcript.
	///
	/// Mirrors: `Transcript.absorbBytes32(self, data)` in Solidity.
	///
	/// `state = keccak256(state || data)`
	pub fn absorb_bytes32(&mut self, data: [u8; 32]) {
		let mut buf = [0u8; 64];
		buf[..32].copy_from_slice(&self.hash);
		buf[32..64].copy_from_slice(&data);
		self.hash = keccak_256(&buf);
	}

	/// Absorb a uint256 value into the transcript.
	///
	/// Mirrors: `Transcript.absorbUint256(self, data)` in Solidity.
	///
	/// The value is encoded as 32 bytes big-endian (matching Solidity's
	/// `abi.encodePacked(uint256)`).
	pub fn absorb_uint256(&mut self, value: U256) {
		let be_bytes = value.to_big_endian();
		self.absorb_bytes32(be_bytes);
	}

	/// Squeeze a pseudo-random 32-byte challenge from the transcript.
	///
	/// Mirrors: `Transcript.squeeze(self)` in Solidity.
	///
	/// ```text
	/// challenge = keccak256(state || bytes8("squeeze"))
	/// state     = keccak256(state || challenge)
	/// ```
	pub fn squeeze(&mut self) -> [u8; 32] {
		let mut buf = [0u8; 40]; // 32 + 8
		buf[..32].copy_from_slice(&self.hash);
		buf[32..40].copy_from_slice(&SQUEEZE_LABEL);
		let challenge = keccak_256(&buf);

		let mut buf2 = [0u8; 64];
		buf2[..32].copy_from_slice(&self.hash);
		buf2[32..64].copy_from_slice(&challenge);
		self.hash = keccak_256(&buf2);

		challenge
	}

	/// Squeeze a pseudo-random index in `[0, modulus)`.
	///
	/// Mirrors: `Transcript.squeezeIndex(self, modulus)` in Solidity.
	pub fn squeeze_index(&mut self, modulus: u64) -> u64 {
		let challenge = self.squeeze();
		let value = U256::from_big_endian(&challenge);
		let modulus_u256 = U256::from(modulus);
		(value % modulus_u256).as_u64()
	}

	/// Sample `count` unique random indices in `[0, modulus)`.
	///
	/// Mirrors: `Transcript.sampleUniqueIndices(self, count, modulus)` in Solidity.
	///
	/// Uses rejection sampling, then sorts ascending.
	pub fn sample_unique_indices(&mut self, count: usize, modulus: u64) -> Vec<u64> {
		assert!(count as u64 <= modulus, "Transcript: count exceeds modulus");

		let mut indices = Vec::with_capacity(count);

		while indices.len() < count {
			let candidate = self.squeeze_index(modulus);
			if !indices.contains(&candidate) {
				indices.push(candidate);
			}
		}

		indices.sort_unstable();
		indices
	}
}

// ─────────────────────────────────────────────────
//  Challenge derivation
// ─────────────────────────────────────────────────

/// Derives the [`SAMPLE_SIZE`] authority indices that the Fiat-Shamir verifier
/// will challenge, using the signers bitmap.
///
/// The transcript absorbs:
///   - `commitment_hash`: binds to this specific block
///   - `authority_root` + `authority_set_len`: binds to the validator set
///   - all 4 bitmap words: binds to the exact signer set
///
/// Then we sample `SAMPLE_SIZE` unique indices from `[0, signer_count)` and
/// map each to the actual authority index (the n-th set bit in the bitmap).
///
/// Returns a sorted `Vec` of authority indices.
pub fn derive_authority_challenge(
	commitment_hash: [u8; 32],
	authority_root: H256,
	authority_set_len: u32,
	bitmap: &SignersBitmap,
	signer_count: u32,
) -> Vec<u32> {
	let mut transcript = Transcript::init(TRANSCRIPT_DOMAIN);

	// Absorb commitment + authority set (same order as Solidity)
	transcript.absorb_bytes32(commitment_hash);
	transcript.absorb_bytes32(authority_root.0);
	transcript.absorb_uint256(U256::from(authority_set_len));

	// Absorb the entire bitmap
	for w in 0..BITMAP_WORDS {
		transcript.absorb_uint256(bitmap.words[w]);
	}

	// Build the signers array once — O(authority_set_len)
	let signers = bitmap.enumerate_signers(authority_set_len);

	// Sample SAMPLE_SIZE unique indices from [0, signer_count)
	let sampled_positions = transcript.sample_unique_indices(SAMPLE_SIZE, signer_count as u64);

	// Map each sampled position to the actual authority index — O(SAMPLE_SIZE)
	sampled_positions.iter().map(|&pos| signers[pos as usize]).collect()
}

/// Computes the commitment hash used as input to the Fiat-Shamir transcript.
///
/// This SCALE-encodes the commitment and keccak256-hashes it, mirroring
/// `keccak256(Codec.Encode(commitment))` in Solidity.
pub fn compute_commitment_hash(commitment: &sp_consensus_beefy::Commitment<u32>) -> [u8; 32] {
	keccak_256(&commitment.encode())
}

/// Given the full BEEFY signed commitment and the challenged authority indices,
/// extracts and processes the signatures for exactly those authorities.
///
/// Each signature has its recovery id adjusted (`v += 27`) for Ethereum-compatible
/// ecrecover, matching the processing in the naive `consensus_proof`.
///
/// # Errors
///
/// Returns an error if any challenged authority did not sign the commitment.
pub fn filter_signatures_for_challenge(
	signed_commitment: &sp_consensus_beefy::SignedCommitment<u32, Signature>,
	challenged_indices: &[u32],
) -> Result<Vec<SignatureWithAuthorityIndex>, anyhow::Error> {
	let mut filtered = Vec::with_capacity(challenged_indices.len());

	for &authority_index in challenged_indices {
		let idx = authority_index as usize;

		if idx >= signed_commitment.signatures.len() {
			return Err(anyhow!(
				"Challenged authority index {} is out of range (total signatures: {})",
				authority_index,
				signed_commitment.signatures.len()
			));
		}

		let sig = signed_commitment.signatures[idx].as_ref().ok_or_else(|| {
			anyhow!("Challenged authority at index {} did not sign the commitment", authority_index)
		})?;

		let encoded = sig.encode();
		if encoded.len() != 65 {
			return Err(anyhow!(
				"Signature at index {} has unexpected length: {} (expected 65)",
				authority_index,
				encoded.len()
			));
		}

		let mut temp = [0u8; 65];
		temp.copy_from_slice(&encoded);
		// Adjust recovery id for Ethereum ecrecover compatibility
		let last = temp.last_mut().unwrap();
		*last += 27;

		filtered
			.push(SignatureWithAuthorityIndex { index: authority_index as u32, signature: temp });
	}

	Ok(filtered)
}

#[cfg(test)]
mod tests {
	use super::*;

	// ── Transcript tests ──

	#[test]
	fn transcript_init_produces_keccak_of_domain() {
		let t = Transcript::init(TRANSCRIPT_DOMAIN);
		let expected = keccak_256(TRANSCRIPT_DOMAIN);
		assert_eq!(t.hash, expected);
	}

	#[test]
	fn absorb_bytes32_matches_solidity_abi_encode_packed() {
		let mut t = Transcript::init(b"test");
		let data = [0xabu8; 32];
		t.absorb_bytes32(data);

		let init_hash = keccak_256(b"test");
		let mut concat = [0u8; 64];
		concat[..32].copy_from_slice(&init_hash);
		concat[32..].copy_from_slice(&data);
		assert_eq!(t.hash, keccak_256(&concat));
	}

	#[test]
	fn squeeze_advances_state() {
		let mut t = Transcript::init(b"test");
		let state_before = t.hash;
		let challenge = t.squeeze();

		let mut buf = [0u8; 40];
		buf[..32].copy_from_slice(&state_before);
		buf[32..40].copy_from_slice(&SQUEEZE_LABEL);
		assert_eq!(challenge, keccak_256(&buf));
		assert_ne!(t.hash, state_before);
	}

	#[test]
	fn absorb_uint256_encodes_as_big_endian() {
		let mut t1 = Transcript::init(b"test");
		t1.absorb_uint256(U256::from(42u64));

		let mut t2 = Transcript::init(b"test");
		let mut be = [0u8; 32];
		be[31] = 42;
		t2.absorb_bytes32(be);

		assert_eq!(t1.hash, t2.hash);
	}

	#[test]
	fn sample_unique_indices_returns_sorted_unique_values() {
		let mut t = Transcript::init(b"uniqueness_test");
		let indices = t.sample_unique_indices(10, 300);

		assert_eq!(indices.len(), 10);
		let mut deduped = indices.clone();
		deduped.dedup();
		assert_eq!(deduped.len(), 10);

		for i in 1..indices.len() {
			assert!(indices[i] > indices[i - 1]);
		}
		for &idx in &indices {
			assert!(idx < 300);
		}
	}

	// ── Bitmap tests ──

	#[test]
	fn bitmap_set_and_get() {
		let mut bm = SignersBitmap::default();
		assert!(!bm.is_bit_set(0));
		assert!(!bm.is_bit_set(255));
		assert!(!bm.is_bit_set(256));
		assert!(!bm.is_bit_set(1023));

		bm.set_bit(0);
		bm.set_bit(255);
		bm.set_bit(256);
		bm.set_bit(1023);

		assert!(bm.is_bit_set(0));
		assert!(bm.is_bit_set(255));
		assert!(bm.is_bit_set(256));
		assert!(bm.is_bit_set(1023));
		assert!(!bm.is_bit_set(1));
		assert!(!bm.is_bit_set(257));
	}

	#[test]
	fn bitmap_count_set_bits() {
		let mut bm = SignersBitmap::default();
		bm.set_bit(5);
		bm.set_bit(10);
		bm.set_bit(200);
		bm.set_bit(300); // in second word

		assert_eq!(bm.count_set_bits(200), 2); // only 5, 10
		assert_eq!(bm.count_set_bits(201), 3); // 5, 10, 200
		assert_eq!(bm.count_set_bits(301), 4); // 5, 10, 200, 300
	}

	#[test]
	fn bitmap_enumerate_signers() {
		let mut bm = SignersBitmap::default();
		bm.set_bit(3);
		bm.set_bit(7);
		bm.set_bit(15);
		bm.set_bit(260);

		let signers = bm.enumerate_signers(300);
		assert_eq!(signers, vec![3, 7, 15, 260]);
	}

	#[test]
	fn bitmap_enumerate_signers_respects_authority_set_len() {
		let mut bm = SignersBitmap::default();
		bm.set_bit(3);
		bm.set_bit(7);
		bm.set_bit(15);
		bm.set_bit(260);

		// authority_set_len = 100, so 260 is excluded
		let signers = bm.enumerate_signers(100);
		assert_eq!(signers, vec![3, 7, 15]);
	}

	// ── Challenge derivation tests ──

	#[test]
	fn derive_authority_challenge_is_deterministic() {
		let commitment_hash = keccak_256(b"some_commitment");
		let root = H256::from([0xcc; 32]);

		let mut bm = SignersBitmap::default();
		for i in 0..150 {
			bm.set_bit(i);
		}

		let a = derive_authority_challenge(commitment_hash, root, 200, &bm, 150);
		let b = derive_authority_challenge(commitment_hash, root, 200, &bm, 150);

		assert_eq!(a, b);
		assert_eq!(a.len(), SAMPLE_SIZE);
	}

	#[test]
	fn different_bitmaps_produce_different_challenges() {
		let commitment_hash = keccak_256(b"some_commitment");
		let root = H256::from([0xcc; 32]);

		let mut bm_a = SignersBitmap::default();
		for i in 0..150 {
			bm_a.set_bit(i);
		}

		let mut bm_b = SignersBitmap::default();
		for i in 50..200 {
			bm_b.set_bit(i);
		}

		let a = derive_authority_challenge(commitment_hash, root, 200, &bm_a, 150);
		let b = derive_authority_challenge(commitment_hash, root, 200, &bm_b, 150);

		assert_ne!(a, b);
	}

	#[test]
	fn challenged_indices_are_valid_signer_positions() {
		let commitment_hash = keccak_256(b"test_validity");
		let root = H256::from([0xdd; 32]);

		// Only even-numbered authorities signed
		let mut bm = SignersBitmap::default();
		for i in (0..200).step_by(2) {
			bm.set_bit(i);
		}
		let signer_count = bm.count_set_bits(200);

		let challenged = derive_authority_challenge(commitment_hash, root, 200, &bm, signer_count);

		assert_eq!(challenged.len(), SAMPLE_SIZE);
		for &idx in &challenged {
			assert!(idx < 200, "Index out of authority set range");
			assert!(idx % 2 == 0, "Should only pick even indices (signers)");
			assert!(bm.is_bit_set(idx as usize), "Challenged index must be a signer");
		}
	}

	#[test]
	fn bitmap_word_boundary_crossing() {
		let mut bm = SignersBitmap::default();
		// Set bits around the word boundaries: 254, 255, 256, 257
		bm.set_bit(254);
		bm.set_bit(255);
		bm.set_bit(256);
		bm.set_bit(257);

		assert_eq!(bm.count_set_bits(258), 4);
		assert_eq!(bm.enumerate_signers(258), vec![254, 255, 256, 257]);
	}
}
