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

//! Primitive types and traits used by the GRANDPA prover & verifier.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::all)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::collections::BTreeMap;
use codec::{Decode, Encode};
use core::fmt::Debug;
use ismp::host::StateMachine;
use polkadot_sdk::*;
use sp_consensus_grandpa::{AuthorityId, AuthorityList, AuthoritySignature};
use sp_core::{sp_std, H256};
use sp_runtime::traits::Header;
use sp_std::prelude::*;
use sp_storage::StorageKey;

/// GRANDPA justification utilities
pub mod justification;

/// Represents a Hash in this library
pub type Hash = H256;
/// A commit message for this chain's block type.
pub type Commit<H> = finality_grandpa::Commit<
	<H as Header>::Hash,
	<H as Header>::Number,
	AuthoritySignature,
	AuthorityId,
>;

/// The default header type that can be used with the GRANDPA prover.
///
/// This type is compatible with our consensus implementation, using
/// [BlakeTwo256](sp_runtime::traits::BlakeTwo256) for hashing and a 32-bit
/// block number type.
pub type DefaultHeader = sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>;

/// Finality for block B is proved by providing:
/// 1) the justification for the descendant block F;
/// 2) headers sub-chain (B; F] if B != F;
#[derive(Debug, PartialEq, Encode, Decode, Clone)]
pub struct FinalityProof<H: codec::Codec> {
	/// The hash of block F for which justification is provided.
	pub block: Hash,
	/// Justification of the block F.
	pub justification: Vec<u8>,
	/// The set of headers in the range (B; F] that we believe are unknown to the caller. Ordered.
	pub unknown_headers: Vec<H>,
}

/// Previous light client state.
#[derive(Debug, PartialEq, Encode, Decode, Clone)]
pub struct ConsensusState {
	/// Current authority set
	pub current_authorities: AuthorityList,
	/// Id of the current authority set.
	pub current_set_id: u64,
	/// latest finalized height on relay chain or standalone chain
	pub latest_height: u32,
	/// latest finalized hash on relay chain or standalone chain.
	pub latest_hash: Hash,
	/// slot duration for the standalone chain
	pub slot_duration: u64,
	/// State machine for this consensus state
	pub state_machine: StateMachine,
}

/// Holds relavant parachain proofs for both header and timestamp extrinsic.
#[derive(Clone, Debug, Encode, Decode)]
pub struct ParachainHeaderProofs {
	/// State proofs that prove a parachain headers exists at a given relay chain height
	pub state_proof: Vec<Vec<u8>>,
	/// The parachain ids
	pub para_ids: Vec<u32>,
}

/// Parachain headers with a Grandpa finality proof.
#[derive(Clone, Encode, Decode)]
pub struct ParachainHeadersWithFinalityProof<H: codec::Codec> {
	/// The grandpa finality proof: contains relay chain headers from the
	/// last known finalized grandpa block.
	pub finality_proof: FinalityProof<H>,
	/// Contains a map of relay chain header hashes to parachain headers
	/// finalized at the relay chain height. We check for this parachain header finalization
	/// via state proofs. Also contains extrinsic proof for timestamp.
	pub parachain_headers: BTreeMap<Hash, ParachainHeaderProofs>,
}

/// This returns the storage key for a parachain header on the relay chain.
pub fn parachain_header_storage_key(para_id: u32) -> StorageKey {
	let mut storage_key = frame_support::storage::storage_prefix(b"Paras", b"Heads").to_vec();
	let encoded_para_id = para_id.encode();
	storage_key.extend_from_slice(sp_io::hashing::twox_64(&encoded_para_id).as_slice());
	storage_key.extend_from_slice(&encoded_para_id);
	StorageKey(storage_key)
}
