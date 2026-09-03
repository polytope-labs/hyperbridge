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

use super::write_bytes_to_lower_hex;
use alloc::vec::Vec;
use core::{
	cmp::Ordering,
	fmt,
	hash::{Hash, Hasher},
	ops::{Deref, DerefMut},
};
use ssz_types::{typenum::Unsigned, FixedVector};

/// SCALE decoding is length checked.
///
/// The derived `Decode` delegates to `FixedVector`, whose own `Decode` rejects any value that is
/// not exactly `N` elements. That check matters: the SSZ hash root is not sensitive to trailing
/// zero bytes, since a value packs into `ceil(N / 32)` chunks whose tail is already zero padded.
/// An over-length value therefore hashes identically and a consumer authenticating only by
/// `hash_tree_root` would accept it, with the length going unnoticed until some later operation.
/// `decode_length_tests` below pins the behaviour here, where it is relied upon.
#[derive(Default, Clone, codec::Encode, codec::Decode)]
pub struct ByteVector<N: Unsigned>(FixedVector<u8, N>);

// Derived `Eq` would demand `N: Eq`, but `N` is a type level integer that never appears in a
// value, so the bound is spurious.
impl<N: Unsigned> Eq for ByteVector<N> {}

// Hex in, hex out, matching what the beacon API emits. The `serde(with)` attribute cannot be used
// on the field any more because it applies to `FixedVector`, which is not `AsRef<[u8]>`.
#[cfg(feature = "std")]
impl<N: Unsigned> serde::Serialize for ByteVector<N> {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serde_hex_utils::as_hex::serialize(self, serializer)
	}
}

#[cfg(feature = "std")]
impl<'de, N: Unsigned> serde::Deserialize<'de> for ByteVector<N> {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		serde_hex_utils::as_hex::deserialize(deserializer)
	}
}

/// SSZ and merkleization delegate straight to the inner vector, which is what the derive would
/// have produced were tuple structs supported.
impl<N: Unsigned> ssz::Encode for ByteVector<N> {
	fn is_ssz_fixed_len() -> bool {
		<FixedVector<u8, N> as ssz::Encode>::is_ssz_fixed_len()
	}

	fn ssz_fixed_len() -> usize {
		<FixedVector<u8, N> as ssz::Encode>::ssz_fixed_len()
	}

	fn ssz_bytes_len(&self) -> usize {
		self.0.ssz_bytes_len()
	}

	fn ssz_append(&self, buf: &mut Vec<u8>) {
		self.0.ssz_append(buf)
	}
}

impl<N: Unsigned> ssz::Decode for ByteVector<N> {
	fn is_ssz_fixed_len() -> bool {
		<FixedVector<u8, N> as ssz::Decode>::is_ssz_fixed_len()
	}

	fn ssz_fixed_len() -> usize {
		<FixedVector<u8, N> as ssz::Decode>::ssz_fixed_len()
	}

	fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
		FixedVector::from_ssz_bytes(bytes).map(Self)
	}
}

impl<N: Unsigned> tree_hash::TreeHash for ByteVector<N> {
	fn tree_hash_type() -> tree_hash::TreeHashType {
		<FixedVector<u8, N> as tree_hash::TreeHash>::tree_hash_type()
	}

	fn tree_hash_packed_encoding(&self) -> tree_hash::PackedEncoding {
		self.0.tree_hash_packed_encoding()
	}

	fn tree_hash_packing_factor() -> usize {
		<FixedVector<u8, N> as tree_hash::TreeHash>::tree_hash_packing_factor()
	}

	fn tree_hash_root(&self) -> tree_hash::Hash256 {
		self.0.tree_hash_root()
	}
}

impl<N: Unsigned> TryFrom<&[u8]> for ByteVector<N> {
	type Error = ssz_types::Error;

	fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
		FixedVector::new(bytes.to_vec()).map(Self)
	}
}

impl<N: Unsigned> TryFrom<Vec<u8>> for ByteVector<N> {
	type Error = ssz_types::Error;

	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		FixedVector::new(bytes).map(Self)
	}
}

// impl here to satisfy clippy
impl<N: Unsigned> PartialEq for ByteVector<N> {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl<N: Unsigned> PartialOrd for ByteVector<N> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl<N: Unsigned> Ord for ByteVector<N> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.as_ref().cmp(other.as_ref())
	}
}

impl<N: Unsigned> Hash for ByteVector<N> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.as_ref().hash(state);
	}
}

impl<N: Unsigned> fmt::LowerHex for ByteVector<N> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write_bytes_to_lower_hex(f, self)
	}
}

impl<N: Unsigned> fmt::Debug for ByteVector<N> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "ByteVector<{}>({self:#x})", N::to_usize())
	}
}

impl<N: Unsigned> fmt::Display for ByteVector<N> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:#x}")
	}
}

impl<N: Unsigned> AsRef<[u8]> for ByteVector<N> {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl<N: Unsigned> Deref for ByteVector<N> {
	type Target = FixedVector<u8, N>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<N: Unsigned> DerefMut for ByteVector<N> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

/// Conversions to and from `tree_hash`'s root type.
///
/// Merkleization deals in `Hash256`, while the consensus types carry 32 byte roots that also need
/// SCALE, so the two representations meet here rather than at every call site.
impl From<tree_hash::Hash256> for ByteVector<ssz_types::typenum::U32> {
	fn from(hash: tree_hash::Hash256) -> Self {
		Self(FixedVector::new(hash.as_slice().to_vec()).expect("a hash is exactly 32 bytes"))
	}
}

impl From<&ByteVector<ssz_types::typenum::U32>> for tree_hash::Hash256 {
	fn from(bytes: &ByteVector<ssz_types::typenum::U32>) -> Self {
		tree_hash::Hash256::from_slice(bytes.as_ref())
	}
}

impl From<ByteVector<ssz_types::typenum::U32>> for tree_hash::Hash256 {
	fn from(bytes: ByteVector<ssz_types::typenum::U32>) -> Self {
		tree_hash::Hash256::from_slice(bytes.as_ref())
	}
}

#[cfg(test)]
mod decode_length_tests {
	use super::*;
	use codec::{Decode, Encode};

	/// BLS public keys are the case that motivated this: 48 bytes, i.e. two SSZ chunks whose
	/// second is half padding.
	type N = ssz_types::typenum::U48;

	fn encoded(len: usize) -> Vec<u8> {
		vec![7u8; len].encode()
	}

	#[test]
	fn accepts_exactly_n_bytes_and_round_trips() {
		let original = ByteVector::<N>::try_from(vec![7u8; 48]).expect("N bytes is valid");
		let decoded = ByteVector::<N>::decode(&mut &original.encode()[..]).expect("round trip");
		assert_eq!(decoded, original);
		assert_eq!(decoded.as_ref().len(), 48);
	}

	/// The reported vector: one trailing byte past `N`. The SSZ root is unchanged by it, so the
	/// decode boundary is the only place this can be caught.
	#[test]
	fn rejects_one_byte_over() {
		assert!(ByteVector::<N>::decode(&mut &encoded(49)[..]).is_err());
	}

	/// Every length up to the next chunk boundary shares the 48-byte root, so each one has to
	/// be rejected — not just the first.
	#[test]
	fn rejects_every_length_sharing_the_hash_root() {
		for len in 49..=64 {
			assert!(
				ByteVector::<N>::decode(&mut &encoded(len)[..]).is_err(),
				"over-length value of {len} bytes was accepted",
			);
		}
	}

	#[test]
	fn rejects_under_length() {
		for len in [0usize, 1, 47] {
			assert!(
				ByteVector::<N>::decode(&mut &encoded(len)[..]).is_err(),
				"under-length value of {len} bytes was accepted",
			);
		}
	}

	/// An over-length value must not survive as a field of a larger SCALE struct either, since
	/// that is how it arrives in a consensus update.
	#[test]
	fn rejects_when_nested_in_a_struct() {
		#[derive(Encode)]
		struct Wire {
			key: Vec<u8>,
			tail: u32,
		}

		#[derive(Decode)]
		struct Parsed {
			#[allow(dead_code)]
			key: ByteVector<N>,
			#[allow(dead_code)]
			tail: u32,
		}

		let good = Wire { key: vec![7u8; 48], tail: 1 }.encode();
		assert!(Parsed::decode(&mut &good[..]).is_ok());

		let bad = Wire { key: vec![7u8; N + 1], tail: 1 }.encode();
		assert!(Parsed::decode(&mut &bad[..]).is_err());
	}
}
