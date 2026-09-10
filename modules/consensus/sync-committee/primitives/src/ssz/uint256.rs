// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! A 256 bit unsigned integer carrying both SSZ and SCALE.
//!
//! `ssz-rs` shipped its own `U256` implementing both. `alloy_primitives::U256` has SSZ and tree
//! hashing through `ethereum_ssz`, but no SCALE, and the orphan rule stops us adding it: neither
//! the trait nor the type is ours. So the pairing lives in a newtype here, exactly as it did
//! before, with SSZ delegating to alloy and SCALE going through the little endian bytes.

use alloc::vec::Vec;
use alloy_primitives::U256 as AlloyU256;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct U256(pub AlloyU256);

impl From<AlloyU256> for U256 {
	fn from(value: AlloyU256) -> Self {
		Self(value)
	}
}

impl From<U256> for AlloyU256 {
	fn from(value: U256) -> Self {
		value.0
	}
}

impl core::ops::Deref for U256 {
	type Target = AlloyU256;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl codec::Encode for U256 {
	fn encode_to<T: codec::Output + ?Sized>(&self, dest: &mut T) {
		// SSZ serializes a uint256 little endian, so SCALE uses the same byte order to keep the
		// two representations from disagreeing.
		self.0.to_le_bytes::<32>().encode_to(dest)
	}
}

impl codec::Decode for U256 {
	fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
		let bytes = <[u8; 32]>::decode(input)?;
		Ok(Self(AlloyU256::from_le_bytes(bytes)))
	}
}

impl ssz::Encode for U256 {
	fn is_ssz_fixed_len() -> bool {
		<AlloyU256 as ssz::Encode>::is_ssz_fixed_len()
	}

	fn ssz_fixed_len() -> usize {
		<AlloyU256 as ssz::Encode>::ssz_fixed_len()
	}

	fn ssz_bytes_len(&self) -> usize {
		self.0.ssz_bytes_len()
	}

	fn ssz_append(&self, buf: &mut Vec<u8>) {
		self.0.ssz_append(buf)
	}
}

impl ssz::Decode for U256 {
	fn is_ssz_fixed_len() -> bool {
		<AlloyU256 as ssz::Decode>::is_ssz_fixed_len()
	}

	fn ssz_fixed_len() -> usize {
		<AlloyU256 as ssz::Decode>::ssz_fixed_len()
	}

	fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
		AlloyU256::from_ssz_bytes(bytes).map(Self)
	}
}

impl tree_hash::TreeHash for U256 {
	fn tree_hash_type() -> tree_hash::TreeHashType {
		<AlloyU256 as tree_hash::TreeHash>::tree_hash_type()
	}

	fn tree_hash_packed_encoding(&self) -> tree_hash::PackedEncoding {
		self.0.tree_hash_packed_encoding()
	}

	fn tree_hash_packing_factor() -> usize {
		<AlloyU256 as tree_hash::TreeHash>::tree_hash_packing_factor()
	}

	fn tree_hash_root(&self) -> tree_hash::Hash256 {
		self.0.tree_hash_root()
	}
}

#[cfg(feature = "std")]
impl serde::Serialize for U256 {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		self.0.serialize(serializer)
	}
}

#[cfg(feature = "std")]
impl<'de> serde::Deserialize<'de> for U256 {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		AlloyU256::deserialize(deserializer).map(Self)
	}
}
