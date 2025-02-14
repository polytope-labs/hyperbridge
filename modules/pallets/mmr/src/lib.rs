// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

//! # Merkle Mountain Range
//!
//! ## Overview
//!
//! Details on Merkle Mountain Ranges (MMRs) can be found here:
//! <https://github.com/mimblewimble/grin/blob/master/doc/mmr.md>
//!
//! The MMR pallet constructs an MMR from leaves provided by the [`MerkleMountainRangeTree::push`]
//! method. MMR nodes are stored both in:
//! - on-chain storage - hashes only; not full leaf content;
//! - off-chain storage - via Indexing API we push full leaf content (and all internal nodes as
//! well) to the Off-chain DB, so that the data is available for Off-chain workers.
//! Hashing used for MMR is configurable independently from the rest of the runtime (i.e. not using
//! `frame_system::Hashing`) so something compatible with external chains can be used (like
//! Keccak256 for Ethereum compatibility).
//!
//! Depending on the usage context (off-chain vs on-chain) the pallet is able to:
//! - verify MMR leaf proofs (on-chain)
//! - generate leaf proofs (off-chain)
//!
//! See [primitives::Compact] documentation for how you can optimize proof size for leafs that are
//! composed from multiple elements.
//!
//! ## What for?
//!
//! Primary use case for this pallet is to generate MMR root hashes, that can latter on be used by
//! BEEFY protocol (see <https://github.com/paritytech/grandpa-bridge-gadget>).
//! MMR root hashes along with BEEFY will make it possible to build Super Light Clients (SLC) of
//! Substrate-based chains. The SLC will be able to follow finality and can be shown proofs of more
//! details that happened on the source chain.
//! In that case the chain which contains the pallet generates the Root Hashes and Proofs, which
//! are then presented to another chain acting as a light client which can verify them.
//!
//! Secondary use case is to archive historical data, but still be able to retrieve them on-demand
//! if needed. For instance if parent block hashes are stored in the MMR it's possible at any point
//! in time to provide an MMR proof about some past block hash, while this data can be safely pruned
//! from on-chain storage.
//!
//! NOTE This pallet is experimental and not proven to work in production.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use core::marker::PhantomData;
use frame_system::pallet_prelude::{BlockNumberFor, HeaderFor};
use log;
use merkle_mountain_range::MMRStore;
use polkadot_sdk::*;
use sp_core::H256;

use sp_runtime::traits::{self, One};
use sp_std::prelude::*;

use mmr_primitives::DataOrHash;
pub use pallet::*;
use pallet_ismp::{
	child_trie,
	offchain::{ForkIdentifier, FullLeaf, LeafMetadata, OffchainDBProvider, Proof, ProofKeys},
};
use sp_mmr_primitives::{
	mmr_lib::leaf_index_to_pos, utils::NodesUtils, Error, LeafIndex, NodeIndex,
};

pub use mmr::storage::{OffchainStorage, Storage};

pub mod mmr;

/// An MMR specific to the pallet.
type ModuleMmr<StorageType, T, I> = mmr::Mmr<StorageType, T, I, LeafOf<T, I>>;

/// Leaf data.
type LeafOf<T, I> = <T as Config<I>>::Leaf;

/// Hashing used for the pallet.
pub(crate) type HashingOf<T, I> = <T as Config<I>>::Hashing;
/// Hash type used for the pallet.
pub(crate) type HashOf<T, I> = <<T as Config<I>>::Hashing as traits::Hash>::Output;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	/// This pallet's configuration trait
	#[pallet::config]
	pub trait Config<I: 'static = ()>:
		polkadot_sdk::frame_system::Config + pallet_ismp::Config
	{
		/// Prefix for elements stored in the Off-chain DB via Indexing API.
		///
		/// Each node of the MMR is inserted both on-chain and off-chain via Indexing API.
		/// The former does not store full leaf content, just its compact version (hash),
		/// and some of the inner mmr nodes might be pruned from on-chain storage.
		/// The latter will contain all the entries in their full form.
		///
		/// Each node is stored in the Off-chain DB under key derived from the
		/// [`Self::INDEXING_PREFIX`] and its in-tree index (MMR position).
		const INDEXING_PREFIX: &'static [u8];

		/// A hasher type for MMR.
		///
		/// To construct trie nodes that result in merging (bagging) two peaks, depending on the
		/// node kind we take either:
		/// - The node (hash) itself if it's an inner node.
		/// - The hash of SCALE-encoding of the leaf data if it's a leaf node.
		///
		/// Then we create a tuple of these two hashes, SCALE-encode it (concatenate) and
		/// hash, to obtain a new MMR inner node - the new peak.
		type Hashing: traits::Hash;

		/// Generic leaf type to be inserted into the MMR.
		type Leaf: FullLeaf + scale_info::TypeInfo;

		/// A type that returns a hash unique to every block as a fork identifer for offchain keys
		type ForkIdentifierProvider: ForkIdentifier<Self>;
	}

	/// Latest MMR Root hash.
	#[pallet::storage]
	#[pallet::getter(fn mmr_root_hash)]
	pub type RootHash<T: Config<I>, I: 'static = ()> = StorageValue<_, HashOf<T, I>, ValueQuery>;

	/// Current size of the MMR (number of leaves).
	#[pallet::storage]
	#[pallet::getter(fn leaf_count)]
	pub type NumberOfLeaves<T: Config<I>, I: 'static = ()> = StorageValue<_, LeafIndex, ValueQuery>;

	/// Height at which the pallet started inserting leaves into offchain storage.
	#[pallet::storage]
	#[pallet::getter(fn initial_height)]
	pub type InitialHeight<T: Config<I>, I: 'static = ()> =
		StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	/// Temporary leaf storage for while the block is still executing.
	#[pallet::storage]
	#[pallet::getter(fn intermediate_leaves)]
	pub type IntermediateLeaves<T: Config<I>, I: 'static = ()> =
		CountedStorageMap<_, Identity, NodeIndex, T::Leaf, OptionQuery>;

	/// Hashes of the nodes in the MMR.
	///
	/// Note this collection only contains MMR peaks, the inner nodes (and leaves)
	/// are pruned and only stored in the Offchain DB.
	#[pallet::storage]
	#[pallet::getter(fn mmr_peak)]
	pub type Nodes<T: Config<I>, I: 'static = ()> =
		CountedStorageMap<_, Identity, NodeIndex, HashOf<T, I>, OptionQuery>;

	// Set the initial height at which leaves were pushed to the offchain db for the offchain
	// mmr gadget. Since this is in on_initialize, then the leaves were set in a previous block.
	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			if NumberOfLeaves::<T, I>::get() > 0 && InitialHeight::<T, I>::get().is_none() {
				InitialHeight::<T, I>::put(frame_system::Pallet::<T>::block_number() - One::one())
			}

			Default::default()
		}
	}
}

impl<T, I> OffchainDBProvider for Pallet<T, I>
where
	I: 'static,
	T: Config<I>,
	HashOf<T, I>: Into<H256>,
{
	type Leaf = T::Leaf;

	fn count() -> LeafIndex {
		NumberOfLeaves::<T, I>::get()
	}

	fn proof(
		indices: Vec<LeafIndex>,
	) -> Result<(Vec<Self::Leaf>, sp_mmr_primitives::LeafProof<H256>), Error> {
		let leaves_count = NumberOfLeaves::<T, I>::get();
		let mmr: ModuleMmr<mmr::storage::OffchainStorage, T, I> = mmr::Mmr::new(leaves_count);
		let (leaves, proof) = mmr.generate_proof(indices)?;
		let proof_nodes = proof.items.into_iter().map(Into::into).collect();
		let new_proof = sp_mmr_primitives::LeafProof {
			leaf_indices: proof.leaf_indices,
			leaf_count: proof.leaf_count,
			items: proof_nodes,
		};

		Ok((leaves, new_proof))
	}

	fn push(leaf: T::Leaf) -> LeafMetadata {
		let temp_count = IntermediateLeaves::<T, I>::count() as u64;
		let index = NumberOfLeaves::<T, I>::get() + temp_count;
		IntermediateLeaves::<T, I>::insert(temp_count, leaf);
		let position = leaf_index_to_pos(index);
		LeafMetadata { position, index }
	}

	fn finalize() -> Result<H256, Error> {
		let buffer_len = IntermediateLeaves::<T, I>::count() as u64;
		// no new leaves? early return
		if buffer_len == 0 {
			return Ok(RootHash::<T, I>::get().into());
		}

		let leaves = NumberOfLeaves::<T, I>::get();
		let mut mmr: ModuleMmr<mmr::storage::RuntimeStorage, T, I> = mmr::Mmr::new(leaves);

		// append new leaves to MMR
		let range = 0u64..buffer_len;
		for index in range {
			let leaf = IntermediateLeaves::<T, I>::get(index)
				.expect("Infallible: Leaf was inserted in this block");
			// Mmr push should never fail
			match mmr.push(leaf) {
				None => {
					log::error!(target: "pallet-mmr", "MMR push failed ");
					// MMR push never fails, but better safe than sorry.
					Err(Error::Push)?
				},
				Some(position) => {
					log::trace!(target: "pallet-mmr", "MMR push {position}");
				},
			}
		}

		// Update the size, `mmr.finalize()` should also never fail.
		let (leaves, root) = match mmr.finalize() {
			Ok((leaves, root)) => (leaves, root),
			Err(e) => {
				log::error!(target: "pallet-mmr", "MMR finalize failed: {:?}", e);
				Err(Error::Commit)?
			},
		};

		let _ = IntermediateLeaves::<T, I>::clear(buffer_len as u32, None);
		NumberOfLeaves::<T, I>::put(leaves);
		RootHash::<T, I>::put(root);

		Ok(root.into())
	}

	fn leaf(pos: NodeIndex) -> Result<Option<Self::Leaf>, Error> {
		let store = Storage::<OffchainStorage, T, _, Self::Leaf>::default();
		store
			.get_elem(pos)
			.map(|val| {
				val.and_then(|inner| match inner {
					DataOrHash::Data(leaf) => Some(leaf),
					_ => None,
				})
			})
			.map_err(|_| Error::LeafNotFound)
	}
}

impl<T, I> Pallet<T, I>
where
	I: 'static,
	T: Config<I>,
	HashOf<T, I>: Into<H256>,
{
	/// Build offchain key from a combination of a fork resistant hash, position and indexing prefix
	///
	/// This combination makes the offchain (key,value) entry resilient to chain forks.
	fn node_temp_offchain_key(
		pos: NodeIndex,
		fork_identifier: <T as frame_system::Config>::Hash,
	) -> sp_std::prelude::Vec<u8> {
		NodesUtils::node_temp_offchain_key::<HeaderFor<T>>(
			&T::INDEXING_PREFIX,
			pos,
			fork_identifier,
		)
	}

	/// Build canonical offchain key for node `pos` in MMR.
	///
	/// Used for nodes added by now finalized blocks.
	/// Never read keys using `node_canon_offchain_key` unless you sure that
	/// there's no `node_offchain_key` key in the storage.
	fn node_canon_offchain_key(pos: NodeIndex) -> sp_std::prelude::Vec<u8> {
		NodesUtils::node_canon_offchain_key(&T::INDEXING_PREFIX, pos)
	}

	/// Return the on-chain MMR root hash.
	pub fn mmr_root() -> HashOf<T, I> {
		RootHash::<T, I>::get()
	}

	/// Generate an MMR proof for the given `leaf_indices`.
	/// Note this method can only be used from an off-chain context
	/// (Offchain Worker or Runtime API call), since it requires
	/// all the leaves to be present.
	/// It may return an error or panic if used incorrectly.
	pub fn generate_proof(
		keys: ProofKeys,
	) -> Result<(Vec<T::Leaf>, Proof<H256>), sp_mmr_primitives::Error> {
		let leaf_indices_and_positions = match keys {
			ProofKeys::Requests(commitments) => commitments
				.into_iter()
				.map(|commitment| {
					let val = child_trie::RequestCommitments::<T>::get(commitment)
						.ok_or_else(|| sp_mmr_primitives::Error::LeafNotFound)?
						.offchain;
					Ok(val)
				})
				.collect::<Result<Vec<_>, _>>()?,
			ProofKeys::Responses(commitments) => commitments
				.into_iter()
				.map(|commitment| {
					let val = child_trie::ResponseCommitments::<T>::get(commitment)
						.ok_or_else(|| sp_mmr_primitives::Error::LeafNotFound)?
						.offchain;
					Ok(val)
				})
				.collect::<Result<Vec<_>, _>>()?,
		};
		let indices =
			leaf_indices_and_positions.iter().map(|val| val.leaf_index).collect::<Vec<_>>();
		let (leaves, proof) = Self::proof(indices)?;
		let proof = Proof {
			leaf_indices_and_pos: leaf_indices_and_positions,
			leaf_count: proof.leaf_count,
			items: proof.items,
		};

		Ok((leaves, proof))
	}
}
