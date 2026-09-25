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

//! Storage slot derivation for the ValidatorRegistry contract.
//!
//! Solidity layout, relative to the ERC-7201 base slot
//! [`crate::VALIDATOR_REGISTRY_STORAGE_SLOT`]:
//!
//! ```solidity
//! struct ValidatorRegistryStorage {
//!     mapping(uint256 => Validator) _validatorsByRegistrationId; // base + 0
//!     EnumerableSet.UintSet _activeValidatorRegistrations;       // base + 1 (values), base + 2 (positions)
//!     mapping(bytes32 => bool) _registeredPublicKeys;            // base + 3
//!     uint256 _nextRegistrationId;                               // base + 4
//! }
//! struct Validator {
//!     ValidatorStatus status; // entry + 0
//!     bytes publicKey;        // entry + 1 (32-byte key, long form: header 65, data at keccak(entry + 1))
//!     uint64 votingPower;     // entry + 2
//! }
//! ```

use crate::VALIDATOR_REGISTRY_STORAGE_SLOT;
use ismp::messaging::Keccak256;
use primitive_types::{H256, U256};

/// Storage slots holding a single validator's record in
/// `_validatorsByRegistrationId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatorSlots {
	/// `Validator.status` (`ValidatorStatus` enum in the low byte)
	pub status: H256,
	/// `Validator.publicKey` bytes header; a 32-byte key stores `2 * 32 + 1 = 65` here
	pub public_key_header: H256,
	/// First data slot of `Validator.publicKey`, holding the full 32-byte key
	pub public_key_data: H256,
	/// `Validator.votingPower`
	pub voting_power: H256,
}

/// Byte length of ed25519 public keys in the registry.
pub const ED25519_PUBLIC_KEY_LENGTH: u64 = 32;

/// The `bytes` header value for a 32-byte long-form public key: `2 * len + 1`.
pub const PUBLIC_KEY_HEADER_VALUE: u64 = 2 * ED25519_PUBLIC_KEY_LENGTH + 1;

fn add(slot: H256, offset: u64) -> H256 {
	let position = U256::from_big_endian(slot.as_bytes()) + U256::from(offset);
	H256(position.to_big_endian())
}

/// Slot holding the number of active validator registrations
/// (`_activeValidatorRegistrations._inner._values.length`).
pub fn active_set_length_slot() -> H256 {
	add(VALIDATOR_REGISTRY_STORAGE_SLOT, 1)
}

/// Slot holding the `index`-th active registration id
/// (`_activeValidatorRegistrations._inner._values[index]`).
pub fn active_set_element_slot<H: Keccak256>(index: u64) -> H256 {
	let data_base = H::keccak256(active_set_length_slot().as_bytes());
	add(data_base, index)
}

/// Storage slots of the `Validator` record for `registration_id`.
pub fn validator_slots<H: Keccak256>(registration_id: H256) -> ValidatorSlots {
	let mut preimage = [0u8; 64];
	preimage[..32].copy_from_slice(registration_id.as_bytes());
	preimage[32..].copy_from_slice(VALIDATOR_REGISTRY_STORAGE_SLOT.as_bytes());
	let entry = H::keccak256(&preimage);

	let public_key_header = add(entry, 1);
	ValidatorSlots {
		status: entry,
		public_key_data: H::keccak256(public_key_header.as_bytes()),
		public_key_header,
		voting_power: add(entry, 2),
	}
}
