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

//! Type definitions for Arc consensus.

use alloc::{collections::BTreeMap, vec::Vec};
use codec::{Decode, Encode};
use geth_primitives::CodecHeader;
use ismp::messaging::Keccak256;
use primitive_types::{H160, H256};

/// An ed25519 signature over the SSZ-encoded precommit vote.
pub type VoteSignature = [u8; 64];

/// A validator in the active set.
///
/// The address is the first 20 bytes of `keccak256(public_key)` — Arc's
/// consensus-layer address derivation, which is what commit signatures
/// are keyed by.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Validator {
	/// Consensus address, `keccak256(public_key)[..20]`
	pub address: H160,
	/// The validator's ed25519 public key
	pub public_key: H256,
	/// The validator's voting power
	pub voting_power: u64,
}

impl Validator {
	/// Construct a validator from its public key, deriving the address.
	pub fn new<H: Keccak256>(public_key: H256, voting_power: u64) -> Self {
		Self { address: derive_address::<H>(&public_key), public_key, voting_power }
	}
}

/// Derive the consensus address from an ed25519 public key.
pub fn derive_address<H: Keccak256>(public_key: &H256) -> H160 {
	H160::from_slice(&H::keccak256(public_key.as_bytes()).0[..20])
}

/// The active validator set, keyed by consensus address.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidatorSet {
	/// Validators keyed by consensus address
	pub validators: BTreeMap<H160, Validator>,
	/// Total voting power across all validators
	pub total_voting_power: u64,
}

impl ValidatorSet {
	/// Build a validator set, deduplicating by address. Returns `None` if the
	/// set is empty or the total voting power overflows.
	pub fn new(validators: impl IntoIterator<Item = Validator>) -> Option<Self> {
		let mut map = BTreeMap::new();
		for validator in validators {
			map.insert(validator.address, validator);
		}
		if map.is_empty() {
			return None;
		}
		let total_voting_power = map
			.values()
			.try_fold(0u64, |acc, validator| acc.checked_add(validator.voting_power))?;
		Some(Self { validators: map, total_voting_power })
	}

	/// Look up a validator by consensus address.
	pub fn get(&self, address: &H160) -> Option<&Validator> {
		self.validators.get(address)
	}

	/// The number of validators in the set.
	pub fn len(&self) -> usize {
		self.validators.len()
	}

	/// Whether the set is empty.
	pub fn is_empty(&self) -> bool {
		self.validators.is_empty()
	}

	/// Whether `signed_power` constitutes a quorum (strictly more than 2/3 of
	/// the total voting power), matching Malachite's threshold rule.
	pub fn has_quorum(&self, signed_power: u64) -> bool {
		(signed_power as u128) * 3 > (self.total_voting_power as u128) * 2
	}
}

/// A single validator's precommit signature within a commit certificate.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct CommitSignature {
	/// The signing validator's consensus address
	pub address: H160,
	/// ed25519 signature over the SSZ-encoded precommit vote
	pub signature: VoteSignature,
}

/// Arc's finality artifact: 2/3+ of the active validator set's precommits for
/// an execution block hash, as served by `arc_getCertificate`.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct CommitCertificate {
	/// The finalized height
	pub height: u64,
	/// The consensus round in which the value was decided
	pub round: u32,
	/// The decided value: the execution-layer block hash
	pub block_hash: H256,
	/// Precommit signatures from the active validator set
	pub commit_signatures: Vec<CommitSignature>,
}

/// EIP-1186 proof of the ValidatorRegistry contract's active validator set
/// against an execution header's state root.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidatorSetProof {
	/// Merkle-Patricia proof of the ValidatorRegistry account
	pub account_proof: Vec<Vec<u8>>,
	/// Merkle-Patricia proof nodes for all storage slots that make up the
	/// active validator set (set length, registration ids, and each
	/// validator's status, public key and voting power)
	pub storage_proof: Vec<Vec<u8>>,
}

/// The trusted state maintained by the Arc consensus client.
#[derive(Debug, Clone, Encode, Decode)]
pub struct VerifierState {
	/// The active validator set as of `finalized_height`
	pub current_validators: ValidatorSet,
	/// The latest finalized block number
	pub finalized_height: u64,
	/// The hash of the latest finalized header
	pub finalized_hash: H256,
}

/// Data required to advance the verifier state.
///
/// The certificate is verified against the *trusted* validator set, then the
/// active set is read from the newly finalized header's state root via
/// `validator_set_proof` and adopted as the new trusted set. Because the set
/// can change at any block (there are no epochs), every update re-proves it.
#[derive(Debug, Clone, Encode, Decode)]
pub struct VerifierStateUpdate {
	/// The execution header being finalized
	pub header: CodecHeader,
	/// The commit certificate for `header`'s block hash
	pub certificate: CommitCertificate,
	/// Proof of the active validator set against `header.state_root`
	pub validator_set_proof: ValidatorSetProof,
}
