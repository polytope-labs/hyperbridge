// Copyright (c) 2025 Polytope Labs.
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

//! Offchain DB interfaces and utilities
use polkadot_sdk::*;

use codec::{Decode, Encode};
use ismp::router::{Request, Response};
use scale_info::TypeInfo;
use sp_core::{RuntimeDebug, H256};
use sp_mmr_primitives::NodeIndex;
use sp_std::prelude::*;

/// Queries a request leaf in the mmr
#[derive(codec::Encode, codec::Decode, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct LeafIndexQuery {
	/// Request or response commitment
	pub commitment: H256,
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
)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct LeafIndexAndPos {
	/// Leaf index
	pub leaf_index: u64,
	/// Leaf position
	pub pos: u64,
}

/// A concrete Leaf for the offchain DB
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, scale_info::TypeInfo)]
pub enum Leaf {
	/// A request variant
	Request(Request),
	/// A response variant
	Response(Response),
}

impl FullLeaf for Leaf {
	fn preimage(&self) -> Vec<u8> {
		match self {
			Leaf::Request(req) => req.encode(),
			Leaf::Response(res) => res.encode(),
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

/// The pallet-ismp will use this interface to insert leaves into the offchain db.
/// This allows for batch insertions and asychronous root hash computation, so that
/// the root is only computed once per block.
pub trait OffchainDBProvider {
	/// Concrete leaf type used by the implementation.
	type Leaf;

	/// Returns the total number of leaves that have been persisted to the db.
	fn count() -> u64;

	/// Push a new leaf into the offchain db.
	fn push(leaf: Self::Leaf) -> LeafMetadata;

	/// Merkelize the offchain db and compute it's new root hash. This should only be called once a
	/// block. This should pull the leaves from the buffer and commit them.
	fn finalize() -> Result<H256, sp_mmr_primitives::Error>;

	/// Given the leaf position, return the leaf from the offchain db
	fn leaf(pos: NodeIndex) -> Result<Option<Self::Leaf>, sp_mmr_primitives::Error>;

	/// Generate a proof for the given leaf indices. The implementation should provide
	/// a proof for the leaves at the current block height.
	fn proof(
		indices: Vec<NodeIndex>,
	) -> Result<(Vec<Self::Leaf>, sp_mmr_primitives::LeafProof<H256>), sp_mmr_primitives::Error>;
}

/// Offchain key for storing requests using the commitment as identifiers
pub fn leaf_default_key(commitment: H256) -> Vec<u8> {
	let prefix = b"no_op";
	(prefix, commitment).encode()
}

impl OffchainDBProvider for () {
	type Leaf = Leaf;

	fn count() -> u64 {
		0
	}

	fn proof(
		_indices: Vec<u64>,
	) -> Result<(Vec<Self::Leaf>, sp_mmr_primitives::LeafProof<H256>), sp_mmr_primitives::Error> {
		Err(sp_mmr_primitives::Error::GenerateProof)?
	}

	fn push(leaf: Self::Leaf) -> LeafMetadata {
		let encoded = leaf.preimage();
		let commitment = sp_io::hashing::keccak_256(&encoded);
		let offchain_key = leaf_default_key(commitment.into());
		sp_io::offchain_index::set(&offchain_key, &leaf.encode());
		Default::default()
	}

	fn finalize() -> Result<H256, sp_mmr_primitives::Error> {
		Ok(H256::default())
	}

	fn leaf(_pos: NodeIndex) -> Result<Option<Self::Leaf>, sp_mmr_primitives::Error> {
		Ok(None)
	}
}

/// A full leaf content stored in the offchain-db.
pub trait FullLeaf: Clone + PartialEq + core::fmt::Debug + codec::FullCodec {
	/// Compute the leaf preimage to be hashed.
	fn preimage(&self) -> Vec<u8>;
}

/// This trait should provide a hash that is unique to each block
/// This hash will be used as an identifier when creating the non canonical offchain key
pub trait ForkIdentifier<T: frame_system::Config> {
	/// Returns a unique identifier for the current block
	fn identifier() -> T::Hash;
}

/// Distinguish between requests and responses
#[derive(TypeInfo, Encode, Decode, serde::Deserialize, serde::Serialize)]
pub enum ProofKeys {
	/// Request commitments
	Requests(Vec<H256>),
	/// Response commitments
	Responses(Vec<H256>),
}

/// An MMR proof data for a group of leaves.
#[derive(codec::Encode, codec::Decode, RuntimeDebug, Clone, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Proof<Hash> {
	/// The indices and positions of the leaves in the proof.
	pub leaf_indices_and_pos: Vec<LeafIndexAndPos>,
	/// Number of leaves in MMR, when the proof was generated.
	pub leaf_count: NodeIndex,
	/// Proof elements (hashes of siblings of inner nodes on the path to the leaf).
	pub items: Vec<Hash>,
}
