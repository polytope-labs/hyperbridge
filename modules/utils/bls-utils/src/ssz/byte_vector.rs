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
use alloc::{vec, vec::Vec};
use core::{
	cmp::Ordering,
	fmt,
	hash::{Hash, Hasher},
	ops::{Deref, DerefMut},
};
use ssz_rs::prelude::*;

#[derive(Default, Clone, Eq, SimpleSerialize, codec::Encode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ByteVector<const N: usize>(
	#[cfg_attr(feature = "serde", serde(with = "serde_hex_utils::as_hex"))] Vector<u8, N>,
);

/// Length-checked SCALE decoding.
///
/// A derived `Decode` here delegates to the inner `Vector`, which carries its own derive and
/// so accepts any length — leaving a `ByteVector<N>` holding something other than `N` bytes.
/// That matters because the SSZ hash root is not sensitive to trailing zero bytes: a value
/// packs into `ceil(N / 32)` chunks with the tail already zero-padded, so appending zeros up
/// to the next chunk boundary consumes padding that was already there and leaves the root
/// unchanged. A consumer that authenticates only by `hash_tree_root` would therefore accept an
/// over-length value and persist it, and the length would not be caught until some later
/// operation — for a BLS key, a point decompression far away from this decode.
///
/// Reject at the boundary instead, reusing the same `deserialize` the byte-slice conversions
/// above go through so there is one definition of the length rule.
///
/// The wire format is unchanged: `Vector`'s SCALE representation is just its inner `Vec`, as
/// its merkle cache is `codec(skip)`.
impl<const N: usize> codec::Decode for ByteVector<N> {
	fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
		let bytes = Vec::<u8>::decode(input)?;
		if bytes.len() != N {
			return Err(codec::Error::from("ByteVector: decoded length does not equal N"));
		}
		ByteVector::<N>::try_from(bytes)
			.map_err(|_| codec::Error::from("ByteVector: SSZ deserialization failed"))
	}
}

impl<const N: usize> TryFrom<&[u8]> for ByteVector<N> {
	type Error = ssz_rs::DeserializeError;

	fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
		ByteVector::<N>::deserialize(bytes)
	}
}

impl<const N: usize> TryFrom<Vec<u8>> for ByteVector<N> {
	type Error = ssz_rs::DeserializeError;

	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		ByteVector::<N>::deserialize(&bytes)
	}
}

// impl here to satisfy clippy
impl<const N: usize> PartialEq for ByteVector<N> {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl<const N: usize> PartialOrd for ByteVector<N> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl<const N: usize> Ord for ByteVector<N> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.as_ref().cmp(other.as_ref())
	}
}

impl<const N: usize> Hash for ByteVector<N> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.as_ref().hash(state);
	}
}

impl<const N: usize> fmt::LowerHex for ByteVector<N> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write_bytes_to_lower_hex(f, self)
	}
}

impl<const N: usize> fmt::Debug for ByteVector<N> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "ByteVector<{N}>({self:#x})")
	}
}

impl<const N: usize> fmt::Display for ByteVector<N> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:#x}")
	}
}

impl<const N: usize> AsRef<[u8]> for ByteVector<N> {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl<const N: usize> Deref for ByteVector<N> {
	type Target = Vector<u8, N>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<const N: usize> DerefMut for ByteVector<N> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[cfg(test)]
mod decode_length_tests {
	use super::*;
	use codec::{Decode, Encode};

	/// BLS public keys are the case that motivated this: 48 bytes, i.e. two SSZ chunks whose
	/// second is half padding.
	const N: usize = 48;

	fn encoded(len: usize) -> Vec<u8> {
		vec![7u8; len].encode()
	}

	#[test]
	fn accepts_exactly_n_bytes_and_round_trips() {
		let original = ByteVector::<N>::try_from(vec![7u8; N]).expect("N bytes is valid");
		let decoded = ByteVector::<N>::decode(&mut &original.encode()[..]).expect("round trip");
		assert_eq!(decoded, original);
		assert_eq!(decoded.as_ref().len(), N);
	}

	/// The reported vector: one trailing byte past `N`. The SSZ root is unchanged by it, so the
	/// decode boundary is the only place this can be caught.
	#[test]
	fn rejects_one_byte_over() {
		assert!(ByteVector::<N>::decode(&mut &encoded(N + 1)[..]).is_err());
	}

	/// Every length up to the next chunk boundary shares the 48-byte root, so each one has to
	/// be rejected — not just the first.
	#[test]
	fn rejects_every_length_sharing_the_hash_root() {
		for len in (N + 1)..=64 {
			assert!(
				ByteVector::<N>::decode(&mut &encoded(len)[..]).is_err(),
				"over-length value of {len} bytes was accepted",
			);
		}
	}

	#[test]
	fn rejects_under_length() {
		for len in [0usize, 1, N - 1] {
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

		let good = Wire { key: vec![7u8; N], tail: 1 }.encode();
		assert!(Parsed::decode(&mut &good[..]).is_ok());

		let bad = Wire { key: vec![7u8; N + 1], tail: 1 }.encode();
		assert!(Parsed::decode(&mut &bad[..]).is_err());
	}
}
