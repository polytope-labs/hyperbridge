// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! A 32 byte merkle root.
//!
//! This exists to pin the SCALE encoding. `ssz-rs`'s `Node` wrote 32 raw bytes, and consensus
//! states already in storage, plus every relayer on the current release, are encoded that way.
//! Routing `Root` through `ByteVector<U32>` would have encoded it as a `Vec<u8>`, adding a compact
//! length prefix and making it 33 bytes, which silently breaks decoding of stored `ConsensusState`
//! and of updates from relayers that have not been rebuilt.
//!
//! SSZ and merkleization are unaffected either way, since a 32 byte vector serializes as its bytes.
//! Only SCALE differs, so only SCALE is hand written here.
//!
//! Being a plain array also keeps `Root` `Copy`, which is what `Node` was.

use alloc::{vec, vec::Vec};
use core::{
	cmp::Ordering,
	fmt,
	hash::{Hash, Hasher},
};
use ssz_types::{typenum::U32, FixedVector};

/// A 32 byte merkle root.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Root(pub [u8; 32]);

impl Root {
	/// The bytes of this root.
	pub fn as_bytes(&self) -> &[u8; 32] {
		&self.0
	}
}

impl codec::Encode for Root {
	fn size_hint(&self) -> usize {
		32
	}

	fn encode_to<T: codec::Output + ?Sized>(&self, dest: &mut T) {
		// Raw, with no length prefix: this is what `ssz-rs`'s `Node` wrote, and what every
		// consensus state currently in storage contains.
		dest.write(&self.0)
	}
}

impl codec::EncodeLike for Root {}

impl codec::Decode for Root {
	fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
		let mut bytes = [0u8; 32];
		input.read(&mut bytes)?;
		Ok(Self(bytes))
	}
}

impl ssz::Encode for Root {
	fn is_ssz_fixed_len() -> bool {
		true
	}

	fn ssz_fixed_len() -> usize {
		32
	}

	fn ssz_bytes_len(&self) -> usize {
		32
	}

	fn ssz_append(&self, buf: &mut Vec<u8>) {
		buf.extend_from_slice(&self.0)
	}
}

impl ssz::Decode for Root {
	fn is_ssz_fixed_len() -> bool {
		true
	}

	fn ssz_fixed_len() -> usize {
		32
	}

	fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
		if bytes.len() != 32 {
			return Err(ssz::DecodeError::InvalidByteLength { len: bytes.len(), expected: 32 });
		}
		let mut out = [0u8; 32];
		out.copy_from_slice(bytes);
		Ok(Self(out))
	}
}

impl tree_hash::TreeHash for Root {
	fn tree_hash_type() -> tree_hash::TreeHashType {
		<FixedVector<u8, U32> as tree_hash::TreeHash>::tree_hash_type()
	}

	fn tree_hash_packed_encoding(&self) -> tree_hash::PackedEncoding {
		tree_hash::PackedEncoding::from_slice(&self.0)
	}

	fn tree_hash_packing_factor() -> usize {
		<FixedVector<u8, U32> as tree_hash::TreeHash>::tree_hash_packing_factor()
	}

	fn tree_hash_root(&self) -> tree_hash::Hash256 {
		tree_hash::Hash256::from_slice(&self.0)
	}
}

impl From<tree_hash::Hash256> for Root {
	fn from(hash: tree_hash::Hash256) -> Self {
		let mut out = [0u8; 32];
		out.copy_from_slice(hash.as_slice());
		Self(out)
	}
}

impl From<&Root> for tree_hash::Hash256 {
	fn from(root: &Root) -> Self {
		tree_hash::Hash256::from_slice(&root.0)
	}
}

impl From<Root> for tree_hash::Hash256 {
	fn from(root: Root) -> Self {
		tree_hash::Hash256::from_slice(&root.0)
	}
}

impl TryFrom<&[u8]> for Root {
	type Error = ssz_types::Error;

	fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
		if bytes.len() != 32 {
			return Err(ssz_types::Error::OutOfBounds { i: bytes.len(), len: 32 });
		}
		let mut out = [0u8; 32];
		out.copy_from_slice(bytes);
		Ok(Self(out))
	}
}

impl TryFrom<Vec<u8>> for Root {
	type Error = ssz_types::Error;

	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		Self::try_from(bytes.as_slice())
	}
}

impl AsRef<[u8]> for Root {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl core::ops::Deref for Root {
	type Target = [u8; 32];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl PartialOrd for Root {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Root {
	fn cmp(&self, other: &Self) -> Ordering {
		self.0.cmp(&other.0)
	}
}

impl Hash for Root {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.hash(state)
	}
}

impl fmt::LowerHex for Root {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if f.alternate() {
			write!(f, "0x")?;
		}
		for byte in self.0.iter() {
			write!(f, "{byte:02x}")?;
		}
		Ok(())
	}
}

impl fmt::Debug for Root {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:#x}")
	}
}

impl fmt::Display for Root {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:#x}")
	}
}

#[cfg(feature = "std")]
impl serde::Serialize for Root {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serde_hex_utils::as_hex::serialize(self, serializer)
	}
}

#[cfg(feature = "std")]
impl<'de> serde::Deserialize<'de> for Root {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		serde_hex_utils::as_hex::deserialize(deserializer)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::{Decode, Encode};

	/// The reason this type exists. `ssz-rs`'s `Node` wrote 32 raw bytes; anything with a compact
	/// length prefix would break every `ConsensusState` already in storage and every relayer that
	/// has not been rebuilt.
	#[test]
	fn scale_encoding_is_thirty_two_raw_bytes() {
		let root = Root([7u8; 32]);
		let encoded = root.encode();

		assert_eq!(encoded.len(), 32, "a length prefix would break stored consensus states");
		assert_eq!(encoded, [7u8; 32].to_vec());
		// and byte for byte what a bare array encodes to
		assert_eq!(encoded, [7u8; 32].encode());
	}

	#[test]
	fn scale_round_trips() {
		let root = Root([3u8; 32]);
		assert_eq!(Root::decode(&mut &root.encode()[..]).unwrap(), root);
	}

	/// A struct carrying a `Root` must not gain a prefix either, since that is how it reaches the
	/// runtime inside a consensus update.
	#[test]
	fn nested_in_a_struct_stays_raw() {
		#[derive(Encode, Decode, PartialEq, Eq, Debug)]
		struct Header {
			slot: u64,
			state_root: Root,
		}

		let header = Header { slot: 1, state_root: Root([9u8; 32]) };
		let encoded = header.encode();
		assert_eq!(encoded.len(), 8 + 32);
		assert_eq!(Header::decode(&mut &encoded[..]).unwrap(), header);
	}

	#[test]
	fn ssz_serializes_as_thirty_two_bytes() {
		use ssz::{Decode, Encode as SszEncode};
		let root = Root([5u8; 32]);
		assert_eq!(root.as_ssz_bytes().len(), 32);
		assert_eq!(Root::from_ssz_bytes(&root.as_ssz_bytes()).unwrap(), root);
		assert!(Root::from_ssz_bytes(&[0u8; 31]).is_err());
	}

	/// `Node` was `Copy`; keeping that avoids threading clones through the prover.
	#[test]
	fn root_is_copy() {
		fn takes_copy<T: Copy>(_: T) {}
		takes_copy(Root::default());
	}
}
