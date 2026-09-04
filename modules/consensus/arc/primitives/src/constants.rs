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

//! Constants for the Arc network.

use hex_literal::hex;
use primitive_types::{H160, H256};

/// Consensus client id for Arc.
pub const ARC_CONSENSUS_ID: [u8; 4] = *b"ARCC";

/// The ValidatorRegistry system contract (AdminUpgradeableProxy). The active
/// validator set that signs commit certificates lives in this contract's storage.
pub const VALIDATOR_REGISTRY_ADDRESS: H160 = H160(hex!("3600000000000000000000000000000000000002"));

/// ERC-7201 base slot of `arc.storage.ValidatorRegistry`:
/// `keccak256(abi.encode(uint256(keccak256("arc.storage.ValidatorRegistry")) - 1)) &
/// ~bytes32(uint256(0xff))`.
///
/// Layout (relative to this base):
/// - base + 0: `mapping(uint256 => Validator) _validatorsByRegistrationId`
/// - base + 1: `EnumerableSet.UintSet _activeValidatorRegistrations` (`bytes32[] _values`)
/// - base + 2: the set's `mapping(bytes32 => uint256) _positions`
pub const VALIDATOR_REGISTRY_STORAGE_SLOT: H256 =
	H256(hex!("b58da0dce03316992faea3e12c60705b8ac05a309e27e3bc8421e5b271c9d200"));

/// `ValidatorStatus.Active` discriminant in the ValidatorRegistry contract
/// (`Unknown = 0, Registered = 1, Active = 2`).
pub const VALIDATOR_STATUS_ACTIVE: u8 = 2;

/// Arc testnet EVM chain id.
pub const ARC_TESTNET_CHAIN_ID: u32 = 5042002;
