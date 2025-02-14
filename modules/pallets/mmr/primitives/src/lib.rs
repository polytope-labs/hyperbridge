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

//! Some primitives for the pallet-mmr

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{vec, vec::Vec};
use merkle_mountain_range::helper::{get_peaks, parent_offset, pos_height_in_tree, sibling_offset};
use pallet_ismp::offchain::FullLeaf;
use polkadot_sdk::*;
use sp_runtime::{traits, RuntimeDebug};

/// An element representing either full data or its hash.
///
/// See `Compact` to see how it may be used in practice to reduce the size
/// of proofs in case multiple `LeafDataProvider`s are composed together.
/// This is also used internally by the MMR to differentiate leaf nodes (data)
/// and inner nodes (hashes).
///
/// `DataOrHash::hash` method calculates the hash of this element in its compact form,
/// so should be used instead of hashing the encoded form (which will always be non-compact).
#[derive(RuntimeDebug, Clone, PartialEq)]
pub enum DataOrHash<H: traits::Hash, L> {
	/// Arbitrary data in its full form.
	Data(L),
	/// A hash of some data.
	Hash(H::Output),
}

impl<H: traits::Hash, L> From<L> for DataOrHash<H, L> {
	fn from(l: L) -> Self {
		Self::Data(l)
	}
}

mod encoding {
	use super::*;

	/// A helper type to implement [codec::Codec] for [DataOrHash].
	#[derive(codec::Encode, codec::Decode)]
	enum Either<A, B> {
		Left(A),
		Right(B),
	}

	impl<H: traits::Hash, L: FullLeaf> codec::Encode for DataOrHash<H, L> {
		fn encode_to<T: codec::Output + ?Sized>(&self, dest: &mut T) {
			match self {
				Self::Data(l) =>
					l.using_encoded(|data| Either::<&[u8], &H::Output>::Left(data).encode_to(dest)),
				Self::Hash(h) => Either::<&[u8], &H::Output>::Right(h).encode_to(dest),
			}
		}
	}

	impl<H: traits::Hash, L: FullLeaf + codec::Decode> codec::Decode for DataOrHash<H, L> {
		fn decode<I: codec::Input>(value: &mut I) -> Result<Self, codec::Error> {
			let decoded: Either<Vec<u8>, H::Output> = Either::decode(value)?;
			Ok(match decoded {
				Either::Left(l) => DataOrHash::Data(L::decode(&mut &*l)?),
				Either::Right(r) => DataOrHash::Hash(r),
			})
		}
	}
}

impl<H: traits::Hash, L: FullLeaf> DataOrHash<H, L> {
	/// Retrieve a hash of this item.
	///
	/// Depending on the node type it's going to either be a contained value for [DataOrHash::Hash]
	/// node, or a hash of SCALE-encoded [DataOrHash::Data] data.
	pub fn hash(&self) -> H::Output {
		match *self {
			Self::Data(ref leaf) => <H as traits::Hash>::hash(&leaf.preimage()),
			Self::Hash(ref hash) => *hash,
		}
	}
}

/// Converts a node's mmr position, to it's k-index. The k-index is the node's index within a layer
/// of the subtree. Refer to <https://research.polytope.technology/merkle-mountain-range-multi-proofs>
pub fn mmr_position_to_k_index(mut leaves: Vec<u64>, mmr_size: u64) -> Vec<(u64, usize)> {
	let peaks = get_peaks(mmr_size);
	let mut leaves_with_k_indices = vec![];

	for peak in peaks {
		let leaves: Vec<_> = take_while_vec(&mut leaves, |pos| *pos <= peak);

		if leaves.len() > 0 {
			for pos in leaves {
				let height = pos_height_in_tree(peak);
				let mut index = 0;
				let mut parent_pos = peak;
				for height in (1..=height).rev() {
					let left_child = parent_pos - parent_offset(height - 1);
					let right_child = left_child + sibling_offset(height - 1);
					index *= 2;
					if left_child >= pos {
						parent_pos = left_child;
					} else {
						parent_pos = right_child;
						index += 1;
					}
				}

				leaves_with_k_indices.push((pos, index));
			}
		}
	}

	leaves_with_k_indices
}

fn take_while_vec<T, P: Fn(&T) -> bool>(v: &mut Vec<T>, p: P) -> Vec<T> {
	for i in 0..v.len() {
		if !p(&v[i]) {
			return v.drain(..i).collect();
		}
	}
	v.drain(..).collect()
}
