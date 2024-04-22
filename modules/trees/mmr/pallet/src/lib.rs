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

use frame_system::pallet_prelude::{BlockNumberFor, HeaderFor};
use log;
use sp_core::H256;
use std::marker::PhantomData;

use sp_runtime::{
    traits::{self, One, Zero},
    RuntimeDebug,
};
use sp_std::prelude::*;

pub use pallet::*;
use sp_mmr_primitives::mmr_lib::leaf_index_to_pos;
pub use sp_mmr_primitives::{
    self as primitives, utils::NodesUtils, Error, LeafDataProvider, LeafIndex, NodeIndex,
};

pub use mmr::storage::{OffchainStorage, Storage};

mod mmr;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

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
    pub trait Config<I: 'static = ()>: frame_system::Config {
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
        type Leaf: primitives::FullLeaf + codec::FullCodec + scale_info::TypeInfo;
    }

    /// Latest MMR Root hash.
    #[pallet::storage]
    pub type RootHash<T: Config<I>, I: 'static = ()> = StorageValue<_, HashOf<T, I>, ValueQuery>;

    /// Current size of the MMR (number of leaves).
    #[pallet::storage]
    #[pallet::getter(fn mmr_leaves)]
    pub type NumberOfLeaves<T: Config<I>, I: 'static = ()> = StorageValue<_, LeafIndex, ValueQuery>;

    /// Height at which the pallet started inserting leaves into offchain storage.
    #[pallet::storage]
    #[pallet::getter(fn initial_height)]
    pub type InitialHeight<T: Config<I>, I: 'static = ()> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

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
            if NumberOfLeaves::<T, I>::get() > 0 && InitialHeight::<T, I>::get() == Zero::zero() {
                InitialHeight::<T, I>::put(frame_system::Pallet::<T>::block_number() - One::one())
            }

            Default::default()
        }
    }
}

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
/// into the tree. They can insert as many as they need and request the computed root hash at a
/// later time. This is so that the mmr root is only computed once per block.
///
/// Internally, the pallet makes use of temporary storage item where it places leaves that have not
/// yet been finalized.
pub trait MerkleMountainRangeTree {
    /// Associated leaf type.
    type Leaf;

    /// Returns the total number of leaves that have been committed to the tree.
    fn leaf_count() -> LeafIndex;

    /// Generate an MMR proof for the given `leaf_indices`.
    /// Generates a proof for the MMR at the current block height.
    fn generate_proof(
        indices: Vec<LeafIndex>,
    ) -> Result<(Vec<Self::Leaf>, primitives::Proof<H256>), Error>;

    /// Push a new leaf into the MMR. Doesn't actually perform any expensive tree recomputation.
    /// Simply adds the leaves to a buffer where they can be recalled when the tree actually
    /// needs to be finalized.
    fn push(leaf: Self::Leaf) -> LeafMetadata;

    /// Finalize the tree and compute it's new root hash. Ideally this should only be called once a
    /// block. This will pull the leaves from the buffer and commit them to the underlying tree.
    fn finalize() -> Result<H256, Error>;
}

/// NoOp tree can be used as a drop in replacement for when the underlying mmr tree is unneeded.
pub struct NoOpTree<T>(PhantomData<T>);

impl<T> MerkleMountainRangeTree for NoOpTree<T> {
    type Leaf = T;

    fn leaf_count() -> LeafIndex {
        0
    }

    fn generate_proof(
        _indices: Vec<LeafIndex>,
    ) -> Result<(Vec<Self::Leaf>, primitives::Proof<H256>), Error> {
        Err(Error::GenerateProof)?
    }

    fn push(_leaf: T) -> LeafMetadata {
        Default::default()
    }

    fn finalize() -> Result<H256, Error> {
        Ok(H256::default())
    }
}

impl<T, I> MerkleMountainRangeTree for Pallet<T, I>
where
    I: 'static,
    T: Config<I>,
    HashOf<T, I>: Into<H256>,
{
    type Leaf = T::Leaf;

    fn leaf_count() -> LeafIndex {
        NumberOfLeaves::<T, I>::get()
    }

    fn generate_proof(
        indices: Vec<LeafIndex>,
    ) -> Result<(Vec<Self::Leaf>, primitives::Proof<H256>), Error> {
        let (leaves, proof) = Pallet::<T, I>::generate_proof(indices)?;
        let proof_nodes = proof.items.into_iter().map(Into::into).collect();
        let new_proof = primitives::Proof {
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
            return Ok(RootHash::<T, I>::get().into())
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
}

/// Stateless MMR proof verification for batch of leaves.
///
/// This function can be used to verify received MMR [primitives::Proof] (`proof`)
/// for given leaves set (`leaves`) against a known MMR root hash (`root`).
/// Note, the leaves should be sorted such that corresponding leaves and leaf indices have the
/// same position in both the `leaves` vector and the `leaf_indices` vector contained in the
/// [primitives::Proof].
pub fn verify_leaves_proof<H, L>(
    root: H::Output,
    leaves: Vec<mmr::Node<H, L>>,
    proof: primitives::Proof<H::Output>,
) -> Result<(), primitives::Error>
where
    H: traits::Hash,
    L: primitives::FullLeaf,
{
    let is_valid = mmr::verify_leaves_proof::<H, L>(root, leaves, proof)?;
    if is_valid {
        Ok(())
    } else {
        Err(primitives::Error::Verify.log_debug(("The proof is incorrect.", root)))
    }
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
    /// Build offchain key from `parent_hash` of block that originally added node `pos` to MMR.
    ///
    /// This combination makes the offchain (key,value) entry resilient to chain forks.
    fn node_temp_offchain_key(
        pos: NodeIndex,
        parent_hash: <T as frame_system::Config>::Hash,
    ) -> sp_std::prelude::Vec<u8> {
        NodesUtils::node_temp_offchain_key::<HeaderFor<T>>(&T::INDEXING_PREFIX, pos, parent_hash)
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
    /// Generates a proof for the MMR at the current block height.
    ///
    /// Note this method can only be used from an off-chain context
    /// (Offchain Worker or Runtime API call), since it requires
    /// all the leaves to be present.
    /// It may return an error or panic if used incorrectly.
    pub fn generate_proof(
        indices: Vec<LeafIndex>,
    ) -> Result<(Vec<LeafOf<T, I>>, primitives::Proof<HashOf<T, I>>), primitives::Error> {
        let leaves_count = NumberOfLeaves::<T, I>::get();
        let mmr: ModuleMmr<mmr::storage::OffchainStorage, T, I> = mmr::Mmr::new(leaves_count);
        mmr.generate_proof(indices)
    }

    /// Verify MMR proof for given `leaves`.
    ///
    /// This method is safe to use within the runtime code.
    /// It will return `Ok(())` if the proof is valid
    /// and an `Err(..)` if MMR is inconsistent (some leaves are missing)
    /// or the proof is invalid.
    pub fn verify_leaves(
        leaves: Vec<LeafOf<T, I>>,
        proof: primitives::Proof<HashOf<T, I>>,
    ) -> Result<(), primitives::Error> {
        if proof.leaf_count > NumberOfLeaves::<T, I>::get() ||
            proof.leaf_count == 0 ||
            (proof.items.len().saturating_add(leaves.len())) as u64 > proof.leaf_count
        {
            return Err(primitives::Error::Verify
                .log_debug("The proof has incorrect number of leaves or proof items."))
        }

        let mmr: ModuleMmr<mmr::storage::OffchainStorage, T, I> = mmr::Mmr::new(proof.leaf_count);
        let is_valid = mmr.verify_leaves_proof(leaves, proof)?;
        if is_valid {
            Ok(())
        } else {
            Err(primitives::Error::Verify.log_debug("The proof is incorrect."))
        }
    }
}
