//! `no_std` Poseidon2 commitment over a validator set's BLS12-381 G1 public keys.
//!
//! A port of `gnark-plonk-verifier`'s `commitment` module so it can run inside a substrate
//! runtime, where hyperbridge has to compute the same value the APK circuit binds to before it can
//! publish it in a header digest. Byte-compatible with the gnark circuit's `PublicKeysCommitment`
//! and with the Go reference `apk.NativePublicKeysCommitment`.
//!
//! Two changes from the upstream module, both forced by `no_std`:
//!
//! - the round keys are derived once per call and threaded through, rather than cached in a
//!   `OnceLock`. The derivation is 62 keccaks against 12288 permutations for a full validator set,
//!   so it does not show up in the cost.
//! - arkworks 0.4 rather than 0.5, to match `w3f-bls` and keep a single arkworks in the runtime.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use ark_bls12_381::{Fq, Fr, G1Affine};
use ark_ff::{BigInteger, Field, PrimeField, Zero};
use sha3::{Digest, Keccak256};

/// gnark-crypto default Poseidon2 parameters for BLS12-381 in compression mode.
const WIDTH: usize = 2;
const FULL_ROUNDS: usize = 6;
const PARTIAL_ROUNDS: usize = 50;

/// Equals gnark-crypto's `Parameters.String()` for these parameters. The round keys are a
/// Keccak-256 chain seeded with it, so this string is consensus critical: change a character and
/// every commitment changes.
const SEED: &str = "Poseidon2-BLS12_381[t=2,rF=6,rP=50,d=5]";

/// Number of validator slots the APK circuit is fixed to. A shorter set is padded to this with the
/// identity point.
pub const NUM_VALIDATORS: usize = 1024;

/// Round keys, reproducing gnark-crypto's `Parameters.initRC`: `rnd0 = Keccak(seed)`,
/// `rnd(k+1) = Keccak(rnd k)`, each key taken as `rnd mod r` big-endian. Full rounds carry
/// `WIDTH` keys, partial rounds one, since only lane 0 is keyed.
fn round_keys() -> Vec<Vec<Fr>> {
	let half_full = FULL_ROUNDS / 2;
	let total = FULL_ROUNDS + PARTIAL_ROUNDS;
	let mut rnd: [u8; 32] = Keccak256::digest(SEED.as_bytes()).into();
	let mut keys = Vec::with_capacity(total);
	for round in 0..total {
		let n = if round < half_full || round >= half_full + PARTIAL_ROUNDS { WIDTH } else { 1 };
		let mut row = Vec::with_capacity(n);
		for _ in 0..n {
			rnd = Keccak256::digest(rnd).into();
			row.push(Fr::from_be_bytes_mod_order(&rnd));
		}
		keys.push(row);
	}
	keys
}

/// In-place x^5 S-box.
#[inline]
fn sbox(x: &mut Fr) {
	let base = *x;
	x.square_in_place();
	x.square_in_place();
	*x *= base;
}

/// External (full-round) MDS for t=2: `[[2,1],[1,2]]`.
#[inline]
fn mat_mul_external(s: &mut [Fr; WIDTH]) {
	let sum = s[0] + s[1];
	s[0] += sum;
	s[1] += sum;
}

/// Internal (partial-round) matrix for t=2: `[[2,1],[1,3]]`.
#[inline]
fn mat_mul_internal(s: &mut [Fr; WIDTH]) {
	let sum = s[0] + s[1];
	s[0] += sum;
	s[1].double_in_place();
	s[1] += sum;
}

/// The Poseidon2 permutation on a width-2 state.
fn permutation(state: &mut [Fr; WIDTH], rk: &[Vec<Fr>]) {
	let half_full = FULL_ROUNDS / 2;
	let first_full = &rk[..half_full];
	let partial = &rk[half_full..half_full + PARTIAL_ROUNDS];
	let last_full = &rk[half_full + PARTIAL_ROUNDS..];

	mat_mul_external(state);
	for keys in first_full {
		for (s, k) in state.iter_mut().zip(keys) {
			*s += *k;
		}
		for s in state.iter_mut() {
			sbox(s);
		}
		mat_mul_external(state);
	}
	for keys in partial {
		state[0] += keys[0];
		sbox(&mut state[0]);
		mat_mul_internal(state);
	}
	for keys in last_full {
		for (s, k) in state.iter_mut().zip(keys) {
			*s += *k;
		}
		for s in state.iter_mut() {
			sbox(s);
		}
		mat_mul_external(state);
	}
}

/// 2-to-1 compression with feed-forward on the right input, matching gnark-crypto's
/// `Permutation.Compress`: `right + permutation([left, right])[1]`.
#[inline]
fn compress(left: Fr, right: Fr, rk: &[Vec<Fr>]) -> Fr {
	let mut s = [left, right];
	permutation(&mut s, rk);
	right + s[1]
}

/// Number of 64-bit limbs packed into one `Fr`. Must match `apk.LimbsPerElement` in the circuit.
const LIMBS_PER_ELEMENT: usize = 3;

/// Decompose a coordinate into six little-endian 64-bit limbs, matching gnark's emulated
/// `BLS12381Fp` layout, and pack them [`LIMBS_PER_ELEMENT`] at a time into `Fr` as
/// `l[0] + l[1]*2^64 + l[2]*2^128`, least significant limb first.
///
/// Three limbs span at most 192 bits, comfortably inside `Fr`, so the packing never wraps and
/// stays injective. It binds exactly as tightly as absorbing each limb on its own, at a third of
/// the compressions.
#[inline]
fn coord_packed(c: Fq) -> [Fr; 2] {
	let limbs = c.into_bigint().0;
	core::array::from_fn(|i| {
		// The positional sum written out as big endian bytes, most significant limb first.
		let mut be = [0u8; 8 * LIMBS_PER_ELEMENT];
		for j in 0..LIMBS_PER_ELEMENT {
			let start = 8 * (LIMBS_PER_ELEMENT - 1 - j);
			be[start..start + 8].copy_from_slice(&limbs[i * LIMBS_PER_ELEMENT + j].to_be_bytes());
		}
		Fr::from_be_bytes_mod_order(&be)
	})
}

/// The Poseidon2 commitment over `points`, in the circuit's absorption order: per point, the two
/// packed halves of `X` then the two of `Y`, absorbed through a Merkle-Damgard chain with a zero
/// IV.
///
/// The caller supplies the same list the circuit binds to, which for a validator set means
/// registration order padded to [`NUM_VALIDATORS`] with the identity point.
pub fn public_keys_commitment(points: &[G1Affine]) -> Fr {
	let rk = round_keys();
	let mut state = Fr::zero();
	for p in points {
		let x = coord_packed(p.x);
		let y = coord_packed(p.y);
		for block in x.into_iter().chain(y) {
			state = compress(state, block, &rk);
		}
	}
	state
}

/// The commitment as a 32-byte big-endian value, the `uint256 publicKeysCommitment` argument of
/// `ApkProof.verify`.
pub fn public_keys_commitment_bytes(points: &[G1Affine]) -> [u8; 32] {
	let mut out = [0u8; 32];
	let be = public_keys_commitment(points).into_bigint().to_bytes_be();
	out[32 - be.len()..].copy_from_slice(&be);
	out
}

/// Pad a validator set to the circuit's fixed width with the identity point.
pub fn padded_to_circuit_width(keys: &[G1Affine]) -> Vec<G1Affine> {
	let mut points = keys.to_vec();
	points.resize(NUM_VALIDATORS, G1Affine::identity());
	points
}

/// Resumable form of [`public_keys_commitment`], for absorbing a validator set a chunk at a time.
///
/// A full set costs roughly 420ms in wasm, which does not fit in one block, but the chain is
/// sequential so it splits cleanly: absorb some points, keep the state, carry on next block. The
/// state is a single field element, so a runtime stores 32 bytes and a cursor between blocks.
///
/// The caller is responsible for feeding the same points in the same order that
/// [`public_keys_commitment`] would, which for a validator set means registration order padded to
/// [`NUM_VALIDATORS`] with the identity point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialCommitment {
	state: Fr,
}

impl Default for PartialCommitment {
	fn default() -> Self {
		Self::new()
	}
}

impl PartialCommitment {
	/// A fresh chain, matching the zero IV the gnark hasher starts from.
	pub fn new() -> Self {
		Self { state: Fr::zero() }
	}

	/// Absorb the next run of points. Round keys are derived once per call, so callers should
	/// prefer fewer, larger chunks over many single-point calls.
	pub fn absorb(&mut self, points: &[G1Affine]) {
		let rk = round_keys();
		for p in points {
			let x = coord_packed(p.x);
			let y = coord_packed(p.y);
			for block in x.into_iter().chain(y) {
				self.state = compress(self.state, block, &rk);
			}
		}
	}

	/// The commitment so far, as the 32-byte big-endian value the contract takes. Only meaningful
	/// once every point has been absorbed.
	pub fn finish(&self) -> [u8; 32] {
		let mut out = [0u8; 32];
		let be = self.state.into_bigint().to_bytes_be();
		out[32 - be.len()..].copy_from_slice(&be);
		out
	}

	/// Encode the in-progress state for storage between blocks.
	pub fn to_bytes(&self) -> [u8; 32] {
		self.finish()
	}

	/// Restore a state written by [`Self::to_bytes`].
	pub fn from_bytes(bytes: &[u8; 32]) -> Self {
		Self { state: Fr::from_be_bytes_mod_order(bytes) }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use ark_ec::AffineRepr;

	/// Ground truth from `circuits/apk/commitment_vectors_test.go`, which locks the Poseidon2
	/// digest over `k * G1::generator()` point sets. If gnark-crypto's parameters ever change
	/// these move, and so does every commitment.
	const VECTORS: [(usize, &str); 4] = [
		(1, "4df3ca8a29f6b37c04fefb167022ae638df17383caf668b718bf3b65aa320652"),
		(2, "20b814b4a4cd0249ffee16a12c0e883eac49a18e91f104e0c777d7de9a797267"),
		(3, "1d8d8ce5d1437ebe81c7a10c59d25ec6f53bffb9966d019f460157750a7a1cff"),
		(10, "5f9529f2a793ad64450341a6ef732dc1e1b71ddcca7d83f3704ff5e637a4b3bd"),
	];

	fn k_times_generator(n: usize) -> Vec<G1Affine> {
		let g = G1Affine::generator();
		(1..=n).map(|k| (g * Fr::from(k as u64)).into()).collect()
	}

	#[test]
	fn matches_the_gnark_vectors() {
		for (n, expected) in VECTORS {
			let got = hex::encode(public_keys_commitment_bytes(&k_times_generator(n)));
			assert_eq!(got, expected, "commitment diverged from gnark for n={n}");
		}
	}

	/// The resumable form must agree with the one-shot form for any split, since a runtime chooses
	/// chunk sizes by weight and must not change the result by doing so.
	#[test]
	fn chunked_absorption_matches_the_one_shot_commitment() {
		let points = padded_to_circuit_width(&k_times_generator(3));
		let expected = public_keys_commitment_bytes(&points);

		for chunk in [1usize, 7, 64, 512, NUM_VALIDATORS] {
			let mut partial = PartialCommitment::new();
			for run in points.chunks(chunk) {
				partial.absorb(run);
				// round-trip through storage on every boundary, as a runtime would
				partial = PartialCommitment::from_bytes(&partial.to_bytes());
			}
			assert_eq!(partial.finish(), expected, "chunk size {chunk} changed the commitment");
		}
	}

	#[test]
	fn identity_padding_reaches_circuit_width() {
		let keys = k_times_generator(2);
		let padded = padded_to_circuit_width(&keys);
		assert_eq!(padded.len(), NUM_VALIDATORS);
		assert!(padded[2..].iter().all(|p| p.is_zero()), "padding is not the identity point");
		assert_ne!(
			public_keys_commitment_bytes(&padded),
			public_keys_commitment_bytes(&keys),
			"padding must change the commitment, else set sizes collide"
		);
	}
}
