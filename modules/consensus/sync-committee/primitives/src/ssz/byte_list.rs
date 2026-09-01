use tree_hash::Hash256;
use super::write_bytes_to_lower_hex;
use alloc::{vec, vec::Vec};
use core::{
	fmt,
	hash::{Hash, Hasher},
	ops::{Deref, DerefMut},
};
use ssz_types::{typenum::Unsigned, FixedVector, VariableList};

#[derive(Default, Clone, codec::Encode, codec::Decode)]
pub struct ByteList<N: Unsigned>(VariableList<u8, N>);

// Derived `Eq` would demand `N: Eq`, but `N` is a type level integer that never appears in a value.
impl<N: Unsigned> Eq for ByteList<N> {}

/// SSZ and merkleization delegate to the inner list, which is what the derive would have produced
/// were tuple structs supported.
impl<N: Unsigned> ssz::Encode for ByteList<N> {
	fn is_ssz_fixed_len() -> bool {
		<VariableList<u8, N> as ssz::Encode>::is_ssz_fixed_len()
	}

	fn ssz_fixed_len() -> usize {
		<VariableList<u8, N> as ssz::Encode>::ssz_fixed_len()
	}

	fn ssz_bytes_len(&self) -> usize {
		self.0.ssz_bytes_len()
	}

	fn ssz_append(&self, buf: &mut Vec<u8>) {
		self.0.ssz_append(buf)
	}
}

impl<N: Unsigned> ssz::Decode for ByteList<N> {
	fn is_ssz_fixed_len() -> bool {
		<VariableList<u8, N> as ssz::Decode>::is_ssz_fixed_len()
	}

	fn ssz_fixed_len() -> usize {
		<VariableList<u8, N> as ssz::Decode>::ssz_fixed_len()
	}

	fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
		VariableList::from_ssz_bytes(bytes).map(Self)
	}
}

impl<N: Unsigned> tree_hash::TreeHash for ByteList<N> {
	fn tree_hash_type() -> tree_hash::TreeHashType {
		<VariableList<u8, N> as tree_hash::TreeHash>::tree_hash_type()
	}

	fn tree_hash_packed_encoding(&self) -> tree_hash::PackedEncoding {
		self.0.tree_hash_packed_encoding()
	}

	fn tree_hash_packing_factor() -> usize {
		<VariableList<u8, N> as tree_hash::TreeHash>::tree_hash_packing_factor()
	}

	fn tree_hash_root(&self) -> tree_hash::Hash256 {
		self.0.tree_hash_root()
	}
}

#[cfg(feature = "std")]
impl<N: Unsigned> serde::Serialize for ByteList<N> {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serde_hex_utils::as_hex::serialize(self, serializer)
	}
}

#[cfg(feature = "std")]
impl<'de, N: Unsigned> serde::Deserialize<'de> for ByteList<N> {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		serde_hex_utils::as_hex::deserialize(deserializer)
	}
}

impl<N: Unsigned> TryFrom<&[u8]> for ByteList<N> {
	type Error = ssz_types::Error;

	fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
		VariableList::new(bytes.to_vec()).map(Self)
	}
}

// impl here to satisfy clippy
impl<N: Unsigned> PartialEq for ByteList<N> {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl<N: Unsigned> Hash for ByteList<N> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.as_ref().hash(state);
	}
}

impl<N: Unsigned> fmt::LowerHex for ByteList<N> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write_bytes_to_lower_hex(f, self)
	}
}

impl<N: Unsigned> fmt::Debug for ByteList<N> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "ByteList<{}>(len={})({:#x})", N::to_usize(), self.len(), self)
	}
}

impl<N: Unsigned> fmt::Display for ByteList<N> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:#x}")
	}
}

impl<N: Unsigned> AsRef<[u8]> for ByteList<N> {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl<N: Unsigned> Deref for ByteList<N> {
	type Target = VariableList<u8, N>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<N: Unsigned> DerefMut for ByteList<N> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_byte_list_serde() {
		let list = ByteList::<32>::try_from([255u8, 255u8].as_ref()).unwrap();
		let encoding = ssz::serialize(&list).unwrap();
		assert_eq!(encoding, [255, 255]);

		let recovered_list = ByteList::<32>::deserialize(&encoding).unwrap();
		assert_eq!(list, recovered_list);
	}
}

impl<N: Unsigned> TryFrom<Vec<u8>> for ByteList<N> {
	type Error = ssz_types::Error;

	fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
		VariableList::new(bytes).map(Self)
	}
}
