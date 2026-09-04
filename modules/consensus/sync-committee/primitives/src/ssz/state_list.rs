// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! The Gloas shape of the state's lists.
//!
//! Before Gloas these are bounded `VariableList`s. From Gloas, EIP-7688 makes them
//! `ProgressiveList`s, which carry no capacity at all.
//!
//! A plain type alias cannot express that: `type StateList<T, N> = ProgressiveList<T>` leaves `N`
//! unused, which is `E0091`. It worked under `ssz-rs` only because the bound was a `const`
//! parameter there, and const parameters are exempt from the unused check where type parameters
//! are not. So the progressive shape is a newtype that keeps `N` in a `PhantomData`, letting every
//! field declaration stay identical across both forks.
//!
//! The phantom never reaches the wire. Every impl below delegates to the inner list, so the ssz
//! encoding and hash tree root are exactly `ProgressiveList<T>`'s.

use alloc::vec::Vec;
use core::marker::PhantomData;
use ssz_types::ProgressiveList;

pub struct StateList<T, N>(ProgressiveList<T>, PhantomData<N>);

impl<T, N> StateList<T, N> {
	/// The underlying progressive list.
	pub fn inner(&self) -> &ProgressiveList<T> {
		&self.0
	}
}

impl<T, N> Default for StateList<T, N> {
	fn default() -> Self {
		Self(ProgressiveList::empty(), PhantomData)
	}
}

impl<T: Clone, N> Clone for StateList<T, N> {
	fn clone(&self) -> Self {
		Self(self.0.clone(), PhantomData)
	}
}

impl<T: core::fmt::Debug, N> core::fmt::Debug for StateList<T, N> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl<T: PartialEq, N> PartialEq for StateList<T, N> {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl<T: Eq, N> Eq for StateList<T, N> {}

impl<T, N> From<Vec<T>> for StateList<T, N> {
	fn from(vec: Vec<T>) -> Self {
		Self(ProgressiveList::new(vec), PhantomData)
	}
}

impl<T, N> core::ops::Deref for StateList<T, N> {
	type Target = [T];

	fn deref(&self) -> &[T] {
		&self.0
	}
}

impl<T, N> AsRef<[T]> for StateList<T, N> {
	fn as_ref(&self) -> &[T] {
		&self.0
	}
}

impl<T, N> ssz::Encode for StateList<T, N>
where
	T: ssz::Encode,
{
	fn is_ssz_fixed_len() -> bool {
		<ProgressiveList<T> as ssz::Encode>::is_ssz_fixed_len()
	}

	fn ssz_fixed_len() -> usize {
		<ProgressiveList<T> as ssz::Encode>::ssz_fixed_len()
	}

	fn ssz_bytes_len(&self) -> usize {
		self.0.ssz_bytes_len()
	}

	fn ssz_append(&self, buf: &mut Vec<u8>) {
		self.0.ssz_append(buf)
	}
}

impl<T, N> ssz::Decode for StateList<T, N>
where
	T: ssz::Decode,
{
	fn is_ssz_fixed_len() -> bool {
		<ProgressiveList<T> as ssz::Decode>::is_ssz_fixed_len()
	}

	fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
		ProgressiveList::from_ssz_bytes(bytes).map(|list| Self(list, PhantomData))
	}
}

impl<T, N> tree_hash::TreeHash for StateList<T, N>
where
	T: tree_hash::TreeHash,
{
	fn tree_hash_type() -> tree_hash::TreeHashType {
		<ProgressiveList<T> as tree_hash::TreeHash>::tree_hash_type()
	}

	fn tree_hash_packed_encoding(&self) -> tree_hash::PackedEncoding {
		self.0.tree_hash_packed_encoding()
	}

	fn tree_hash_packing_factor() -> usize {
		<ProgressiveList<T> as tree_hash::TreeHash>::tree_hash_packing_factor()
	}

	fn tree_hash_root(&self) -> tree_hash::Hash256 {
		self.0.tree_hash_root()
	}
}

impl<T, N> codec::Encode for StateList<T, N>
where
	T: codec::Encode,
{
	fn encode_to<O: codec::Output + ?Sized>(&self, dest: &mut O) {
		self.0.encode_to(dest)
	}
}

impl<T, N> codec::Decode for StateList<T, N>
where
	T: codec::Decode,
{
	fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
		ProgressiveList::decode(input).map(|list| Self(list, PhantomData))
	}
}

#[cfg(feature = "std")]
impl<T, N> serde::Serialize for StateList<T, N>
where
	T: serde::Serialize,
{
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		self.0.serialize(serializer)
	}
}

#[cfg(feature = "std")]
impl<'de, T, N> serde::Deserialize<'de> for StateList<T, N>
where
	T: serde::Deserialize<'de>,
{
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		ProgressiveList::deserialize(deserializer).map(|list| Self(list, PhantomData))
	}
}
