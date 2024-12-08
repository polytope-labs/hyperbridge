#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{vec, vec::Vec};
use codec::Encode;
use core::marker::PhantomData;
use merkle_mountain_range::helper::{get_peaks, parent_offset, pos_height_in_tree, sibling_offset};
use sp_core::H256;
use sp_mmr_primitives as primitives;
use sp_mmr_primitives::NodeIndex;
use sp_runtime::{scale_info, traits, RuntimeDebug};
use sp_std::fmt;

/// Leaf index and position
#[derive(
	codec::Encode,
	codec::Decode,
	scale_info::TypeInfo,
	Ord,
	PartialOrd,
	Eq,
	PartialEq,
	Clone,
	Copy,
	RuntimeDebug,
	Default,
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct LeafMetadata {
	/// Leaf index in the tree
	pub index: u64,
	/// Leaf node position in the tree
	pub position: u64,
}

/// Public interface for this pallet. Other runtime pallets will use this interface to insert leaves
/// into the offchain db. This allows for batch insertions and asychronous root hash computation
/// This is so that the root is only computed once per block.
///
/// Internally, the pallet makes use of temporary storage item where it places leaves that have not
/// yet been finalized.
pub trait OffchainDBProvider {
	/// Concrete leaf type used by the implementation.
	type Leaf;

	/// Returns the total number of leaves that have been persisted to the db.
	fn count() -> u64;

	/// Push a new leaf into the offchain db.
	fn push(leaf: Self::Leaf) -> LeafMetadata;

	/// Merkelize the offchain db and compute it's new root hash. This should only be called once a
	/// block. This should pull the leaves from the buffer and commit them.
	fn finalize() -> Result<H256, primitives::Error>;

	/// Given the leaf position, return the leaf from the offchain db
	fn leaf(pos: NodeIndex) -> Result<Option<Self::Leaf>, primitives::Error>;

	/// Generate a proof for the given leaf indices. The implementation should provide
	/// a proof for the leaves at the current block height.
	fn proof(
		indices: Vec<NodeIndex>,
	) -> Result<(Vec<Self::Leaf>, primitives::LeafProof<H256>), primitives::Error>;
}

/// The `PlainOffChainDB` simply persists requests and responses directly to the offchain-db.
pub struct PlainOffChainDB<T, H>(PhantomData<(T, H)>);

impl<T, H> PlainOffChainDB<T, H> {
	/// Offchain key for storing requests using the commitment as identifiers
	pub fn offchain_key(commitment: H256) -> Vec<u8> {
		let prefix = b"no_op";
		(prefix, commitment).encode()
	}
}

impl<T: FullLeaf, H: ismp::messaging::Keccak256> OffchainDBProvider for PlainOffChainDB<T, H> {
	type Leaf = T;

	fn count() -> u64 {
		0
	}

	fn proof(
		_indices: Vec<u64>,
	) -> Result<(Vec<Self::Leaf>, primitives::LeafProof<H256>), primitives::Error> {
		Err(primitives::Error::GenerateProof)?
	}

	fn push(leaf: T) -> LeafMetadata {
		let encoded = leaf.preimage();
		let commitment = H::keccak256(&encoded);
		let offchain_key = Self::offchain_key(commitment);
		sp_io::offchain_index::set(&offchain_key, &leaf.encode());
		Default::default()
	}

	fn finalize() -> Result<H256, primitives::Error> {
		Ok(H256::default())
	}

	fn leaf(_pos: NodeIndex) -> Result<Option<Self::Leaf>, primitives::Error> {
		Ok(None)
	}
}

/// A full leaf content stored in the offchain-db.
pub trait FullLeaf: Clone + PartialEq + fmt::Debug + codec::FullCodec {
	/// Compute the leaf preimage to be hashed.
	fn preimage(&self) -> Vec<u8>;
}

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

/// This trait should provide a hash that is unique to each block
/// This hash will be used as an identifier when creating the non canonical offchain key
pub trait ForkIdentifier<T: frame_system::Config> {
	/// Returns a unique identifier for the current block
	fn identifier() -> T::Hash;
}
