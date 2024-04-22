//! Pallet-mmr runtime Apis

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![deny(missing_docs)]

use sp_mmr_primitives::{Error, LeafIndex};

sp_api::decl_runtime_apis! {
    /// MmrRuntimeApi
    pub trait MmrRuntimeApi<Hash: codec::Codec, BlockNumber: codec::Codec, Leaf: codec::Codec> {
        /// Return Block number where pallet-mmr was added to the runtime
        fn pallet_genesis() -> Result<BlockNumber, Error>;

        /// Return the number of MMR leaves.
        fn mmr_leaf_count() -> Result<LeafIndex, Error>;

        /// Return the on-chain MMR root hash.
        fn mmr_root() -> Result<Hash, Error>;
    }
}
