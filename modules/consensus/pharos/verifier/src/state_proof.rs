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

//! State proof verification for Pharos validator sets.
//!
//! This module handles verification of Merkle-Patricia trie proofs for the
//! validator set stored in the staking contract at
//! `0x4100000000000000000000000000000000000000`.
//!
//! ## Proof Structure
//!
//! The validator set proof consists of:
//! 1. Account proof - proves the staking contract exists at the expected address
//! 2. Storage proof - proves the validator set data in the contract's storage
//!
//! ## Verification Steps
//!
//! 1. Verify the account proof against the state root from the block header
//! 2. Extract the storage root from the proven account
//! 3. Verify the storage proof against the storage root
//! 4. Decode the validator set from the proven storage values

use crate::error::Error;
use alloc::vec::Vec;
use evm_state_machine::{get_contract_account, get_values_from_proof};
use ismp::messaging::Keccak256;
use pharos_primitives::{ValidatorInfo, ValidatorSet, ValidatorSetProof, STAKING_CONTRACT_ADDRESS};
use primitive_types::{H256, U256};
use trie_db::DBValue;

/// This function verifies that the provided validator set is correctly stored
/// in the staking contract at the given block.
///
/// The `epoch` parameter is the epoch this validator set will be valid for,
pub fn verify_validator_set_proof<H: Keccak256 + Send + Sync>(
	state_root: H256,
	proof: &ValidatorSetProof,
	epoch: u64,
) -> Result<ValidatorSet, Error> {
	let account = get_contract_account::<H>(
		proof.account_proof.clone(),
		STAKING_CONTRACT_ADDRESS.as_slice(),
		state_root,
	)
	.map_err(|_| Error::AccountTrieLookupFailed)?;
	let storage_root = H256::from_slice(account.storage_root.as_slice());

	let layout = StakingContractLayout::default();

	// Compute and fetch global storage slots (totalStake, array length, pool IDs)
	let base_slots = vec![
		layout.raw_slot_key(layout.total_stake_slot),
		layout.raw_slot_key(layout.active_pool_set_slot),
	];
	let base_values = get_values_from_proof::<H>(
		base_slots.iter().map(|s| H::keccak256(s.as_bytes()).0.to_vec()).collect(),
		storage_root,
		proof.storage_proof.clone(),
	)
	.map_err(|_| Error::StorageProofLookupFailed)?;

	// validator count from the proof (slot 22 = activePoolSets array length)
	let validator_count = base_values[1]
		.as_ref()
		.map(|v| decode_u256_from_storage(v))
		.transpose()?
		.unwrap_or_default()
		.low_u64();

	let mut pool_id_slots = Vec::new();
	for i in 0..validator_count {
		pool_id_slots.push(layout.array_element_key::<H>(layout.active_pool_set_slot, i));
	}

	let pool_id_values = get_values_from_proof::<H>(
		pool_id_slots.iter().map(|s| H::keccak256(s.as_bytes()).0.to_vec()).collect(),
		storage_root,
		proof.storage_proof.clone(),
	)
	.map_err(|_| Error::StorageProofLookupFailed)?;

	// Extract pool IDs
	let mut pool_ids = Vec::new();
	for value in &pool_id_values {
		let pool_id = value
			.as_ref()
			.map(|v| {
				let mut bytes = [0u8; 32];
				if v.len() <= 32 {
					bytes[32 - v.len()..].copy_from_slice(v);
				}
				H256::from(bytes)
			})
			.unwrap_or_default();
		pool_ids.push(pool_id);
	}

	// validator for each pool ID
	let mut validator_slots = Vec::new();
	for pool_id in &pool_ids {
		validator_slots.extend(layout.get_validator_keys::<H>(pool_id));
	}

	let validator_values = get_values_from_proof::<H>(
		validator_slots.iter().map(|s| H::keccak256(s.as_bytes()).0.to_vec()).collect(),
		storage_root,
		proof.storage_proof.clone(),
	)
	.map_err(|_| Error::StorageProofLookupFailed)?;

	let mut all_slots = base_slots;
	all_slots.extend(pool_id_slots);
	all_slots.extend(validator_slots);

	let mut all_values = base_values;
	all_values.extend(pool_id_values);
	all_values.extend(validator_values);

	let decoded_set = decode_validator_set_from_storage::<H>(&all_slots, &all_values, epoch)?;

	validate_validator_set(&decoded_set)?;

	Ok(decoded_set)
}

/// Decode validator set from storage values.
///
/// This function interprets the raw storage values according to the
/// Pharos staking contract's actual storage layout.
///
/// ## Pharos Storage Layout
///
/// The contract at `0x4100000000000000000000000000000000000000` uses:
/// - Slot 0: validators mapping (mapping(bytes32 => Validator))
/// - Slot 6: totalStake (uint256)
/// - Slot 22: activePoolSets (EnumerableSet._values array)
fn decode_validator_set_from_storage<H: Keccak256>(
	slots: &[H256],
	values: &[Option<DBValue>],
	epoch: u64,
) -> Result<ValidatorSet, Error> {
	if slots.len() != values.len() {
		return Err(Error::SlotValueLengthMismatch { slots: slots.len(), values: values.len() });
	}

	// We need 2 slots at minimum: totalStake, activePoolSets length
	if values.len() < 2 {
		return Err(Error::InsufficientStorageValues { expected: 2, got: values.len() });
	}

	// Parse global state
	// Index 0: totalStake
	let total_stake = values[0]
		.as_ref()
		.map(|v| decode_u256_from_storage(v))
		.transpose()?
		.unwrap_or_default();

	// Index 1: activePoolSets array length (slot 22)
	let validator_count = values[1]
		.as_ref()
		.map(|v| decode_u256_from_storage(v))
		.transpose()?
		.unwrap_or_default();

	let count = validator_count.low_u64() as usize;

	// Pool IDs start at index 2 (after totalStake, array length)
	let pool_set_start = 2;
	let pool_ids_end = pool_set_start + count;

	if values.len() < pool_ids_end {
		return Err(Error::InsufficientPoolIds {
			expected: pool_ids_end,
			validators: count,
			got: values.len(),
		});
	}

	let mut validator_set = ValidatorSet::new(epoch);
	validator_set.total_stake = total_stake;

	// Slots per validator in the detailed proof:
	// - 1 BLS string slot (header)
	// - 3 BLS data slots (for long string)
	// - 1 totalStake
	const VALIDATOR_FIELDS: usize = 5;

	let validators_data_start = pool_ids_end;
	let expected_total = validators_data_start + (count * VALIDATOR_FIELDS);

	// If we have detailed validator data
	if values.len() >= expected_total {
		for i in 0..count {
			// Pool ID from activePoolSets array
			let pool_id = values[pool_set_start + i]
				.as_ref()
				.map(|v| {
					let mut bytes = [0u8; 32];
					if v.len() <= 32 {
						bytes[32 - v.len()..].copy_from_slice(v);
					}
					H256::from(bytes)
				})
				.unwrap_or_default();

			let base_idx = validators_data_start + (i * VALIDATOR_FIELDS);

			// BLS public key: string slot at base_idx, data slots at base_idx+1..base_idx+4
			let bls_string_slot = &values[base_idx];
			let bls_data_slots = &values[base_idx + 1..base_idx + 4];
			let bls_key = decode_bls_key_from_string_slot(bls_string_slot, Some(bls_data_slots))?;

			// totalStake at base_idx + 4
			let stake = values[base_idx + 4]
				.as_ref()
				.map(|v| decode_u256_from_storage(v))
				.transpose()?
				.unwrap_or_default();

			let validator = ValidatorInfo { bls_public_key: bls_key, pool_id, stake };

			validator_set.validators.push(validator);
		}
	} else {
		// Minimal proof - only pool IDs, no detailed validator data
		// This is used when the validator set is provided separately
		// and only pool IDs need to be verified
		log::warn!(
			"Minimal validator proof: only {} values provided, expected {} for full decoding",
			values.len(),
			expected_total
		);
	}

	log::debug!(
		"Decoded validator set: {} validators, epoch {}, total stake {}",
		validator_set.validators.len(),
		validator_set.epoch,
		validator_set.total_stake
	);

	Ok(validator_set)
}

/// Decode BLS public key from a Solidity string storage slot.
///
/// In Solidity, strings are stored as:
/// - Short strings (< 32 bytes): data is stored directly in the slot, length in lowest byte
/// - Long strings (>= 32 bytes): slot contains (length * 2 + 1), data at keccak256(slot)
///
/// The BLS public key is a 48-byte value, typically stored as a 96-character hex string
/// (or 98 with "0x" prefix), so it's a long string requiring 3 data slots.
fn decode_bls_key_from_string_slot(
	header_value: &Option<DBValue>,
	data_slots: Option<&[Option<DBValue>]>,
) -> Result<pharos_primitives::BlsPublicKey, Error> {
	use alloc::string::String;

	let header = header_value.as_ref().ok_or(Error::MissingBlsKeySlot)?;

	if header.is_empty() {
		return Err(Error::EmptyBlsKeySlot);
	}

	let header_val = decode_u256_from_storage(header)?;
	let header_bytes = header_val.to_big_endian();
	let lowest_byte = header_bytes[31];

	let bls_hex: String = if lowest_byte & 1 == 0 {
		// Short string: data is in the slot, length = lowest_byte / 2
		let len = (lowest_byte / 2) as usize;
		if len > 31 {
			return Err(Error::InvalidBlsStringLength);
		}
		// String data is stored in the high bytes of the slot
		String::from_utf8(header_bytes[..len].to_vec()).map_err(|_| Error::InvalidBlsKeyUtf8)?
	} else {
		// Long string: header contains (length * 2 + 1)
		let length = (header_val - 1) / 2;
		let str_len = length.low_u64() as usize;

		// For BLS keys, we expect a 96 or 98 character hex string
		// This requires 3 data slots (ceil(96/32) = 3)
		let data_slots = data_slots.ok_or(Error::LongStringBlsKeyUnsupported)?;

		let slots_needed = (str_len + 31) / 32;
		if data_slots.len() < slots_needed {
			return Err(Error::InsufficientStorageValues {
				expected: slots_needed,
				got: data_slots.len(),
			});
		}

		let mut string_data = Vec::with_capacity(str_len);
		for (i, slot_value) in data_slots.iter().take(slots_needed).enumerate() {
			let slot_data = slot_value.as_ref().ok_or(Error::MissingBlsKeySlot)?;
			let decoded = decode_u256_from_storage(slot_data)?;
			let bytes = decoded.to_big_endian();

			let remaining = str_len - (i * 32);
			let take = remaining.min(32);
			string_data.extend_from_slice(&bytes[..take]);
		}

		String::from_utf8(string_data).map_err(|_| Error::InvalidBlsKeyUtf8)?
	};

	let bls_hex = bls_hex.trim_start_matches("0x");
	let bls_bytes = hex::decode(bls_hex).map_err(|_| Error::InvalidBlsKeyHex)?;

	if bls_bytes.len() != 48 {
		return Err(Error::InvalidBlsKeyLength { expected: 48, got: bls_bytes.len() });
	}

	bls_bytes.try_into().map_err(|_| Error::BlsKeyConversionFailed)
}

/// Validate the internal consistency of a validator set.
pub fn validate_validator_set(validator_set: &ValidatorSet) -> Result<(), Error> {
	if validator_set.validators.is_empty() {
		return Err(Error::EmptyValidatorSet);
	}

	let computed_total: U256 = validator_set
		.validators
		.iter()
		.fold(U256::zero(), |acc, v| acc.saturating_add(v.stake));

	if computed_total != validator_set.total_stake {
		return Err(Error::ComputedStakeMismatch {
			computed: computed_total,
			claimed: validator_set.total_stake,
		});
	}

	for (i, v1) in validator_set.validators.iter().enumerate() {
		for v2 in validator_set.validators.iter().skip(i + 1) {
			if v1.bls_public_key == v2.bls_public_key {
				return Err(Error::DuplicateValidator);
			}
		}
	}

	for validator in &validator_set.validators {
		if validator.stake.is_zero() {
			return Err(Error::ZeroStakeValidator);
		}
	}

	Ok(())
}

/// Storage layout information for the Pharos staking contract.
///
/// Based on the actual Pharos staking contract at `0x4100000000000000000000000000000000000000`.
///
/// ## Contract Storage Layout (StakingStorageV1)
///
/// ```solidity
/// mapping(bytes32 => Validator) public validators;               // slot 0
/// bytes32[] public activePoolIds;                                // slot 1
/// bytes32[] public pendingAddPoolIds;                            // slot 2
/// bytes32[] public pendingUpdatePoolIds;                         // slot 3
/// bytes32[] public pendingExitPoolIds;                           // slot 4
/// uint256 public currentEpoch;                                   // slot 5
/// uint256 public totalStake;                                     // slot 6
/// IChainConfig public cfg;                                       // slot 7
/// mapping(address => uint256) public pendingWithdrawStakes;      // slot 8
/// uint256 public totalSupply;                                    // slot 9
/// uint256 public currentInflationRate;                           // slot 10
/// uint256 public lastInflationAdjustmentTime;                    // slot 11
/// uint256 public lastInflationTotalSupplySnapshot;               // slot 12
/// address internal implAddress;                                  // slot 13
/// ```
///
/// ## Contract Storage Layout (StakingStorageV2)
///
/// ```solidity
/// uint256 lastEpochStartTime;                                    // slot 14
/// mapping(bytes32 => mapping(address => Delegator)) delegators;  // slot 15
/// mapping(bytes32 => mapping(address => bool)) validatorWhitelists; // slot 16
/// mapping(bytes32 => uint256) accumulatedRewardPerShares;        // slot 17
/// mapping(bytes32 => uint256) commissionRates;                   // slot 18
/// mapping(bytes32 => bool) delegationEnabledMapping;             // slot 19
/// mapping(bytes32 => uint256) delegatorCounts;                   // slot 20
/// PendingUndelegation[] pendingUndelegations;                    // slot 21
/// EnumerableSet.Bytes32Set activePoolSets;                       // slot 22-23
/// EnumerableSet.Bytes32Set pendingAddPoolSets;                   // slot 24-25
/// EnumerableSet.Bytes32Set pendingUpdatePoolSets;                // slot 26-27
/// EnumerableSet.Bytes32Set pendingExitPoolSets;                  // slot 28-29
/// ```
///
/// OpenZeppelin's EnumerableSet uses 2 storage slots:
/// - Slot N: The internal values array (bytes32[])
/// - Slot N+1: The indices mapping (mapping(bytes32 => uint256))
///
/// Active pool IDs are stored in `activePoolSets._values` at slot 22.
///
/// ## Validator Struct
///
/// ```solidity
/// struct Validator {
///     string description;          // offset 0
///     string publicKey;            // offset 1
///     string publicKeyPop;         // offset 2
///     string blsPublicKey;         // offset 3
///     string blsPublicKeyPop;      // offset 4
///     string endpoint;             // offset 5
///     uint8 status;                // offset 6
///     bytes32 poolId;              // offset 7
///     uint256 totalStake;          // offset 8
///     address owner;               // offset 9
///     uint256 stakeSnapshot;       // offset 10
///     uint256 pendingWithdrawStake; // offset 11
///     uint8 pendingWithdrawWindow; // offset 12
/// }
/// ```
#[derive(Debug, Clone)]
pub struct StakingContractLayout {
	/// Storage slot for the validators mapping
	pub validators_mapping_slot: u64,
	/// Storage slot for activePoolSets (EnumerableSet._values)
	pub active_pool_set_slot: u64,
	/// Storage slot for totalStake
	pub total_stake_slot: u64,
}

/// Offsets within the Validator struct for each field.
#[derive(Debug, Clone, Copy)]
pub struct ValidatorStructOffsets {
	/// Offset for description (string)
	pub description: u64,
	/// Offset for publicKey (string)
	pub public_key: u64,
	/// Offset for publicKeyPop (string)
	pub public_key_pop: u64,
	/// Offset for blsPublicKey (string)
	pub bls_public_key: u64,
	/// Offset for blsPublicKeyPop (string)
	pub bls_public_key_pop: u64,
	/// Offset for endpoint (string)
	pub endpoint: u64,
	/// Offset for status (uint8)
	pub status: u64,
	/// Offset for poolId (bytes32)
	pub pool_id: u64,
	/// Offset for totalStake (uint256)
	pub total_stake: u64,
	/// Offset for owner (address)
	pub owner: u64,
	/// Offset for stakeSnapshot (uint256)
	pub stake_snapshot: u64,
	/// Offset for pendingWithdrawStake (uint256)
	pub pending_withdraw_stake: u64,
	/// Offset for pendingWithdrawWindow (uint8)
	pub pending_withdraw_window: u64,
}

impl Default for ValidatorStructOffsets {
	fn default() -> Self {
		Self {
			description: 0,
			public_key: 1,
			public_key_pop: 2,
			bls_public_key: 3,
			bls_public_key_pop: 4,
			endpoint: 5,
			status: 6,
			pool_id: 7,
			total_stake: 8,
			owner: 9,
			stake_snapshot: 10,
			pending_withdraw_stake: 11,
			pending_withdraw_window: 12,
		}
	}
}

impl Default for StakingContractLayout {
	fn default() -> Self {
		Self { validators_mapping_slot: 0, active_pool_set_slot: 22, total_stake_slot: 6 }
	}
}

impl StakingContractLayout {
	/// Calculate the raw storage key for a simple slot (no hashing).
	pub fn raw_slot_key(&self, slot: u64) -> H256 {
		H256::from_low_u64_be(slot)
	}

	/// Calculate the storage key for a dynamic array element.
	pub fn array_element_key<H: Keccak256>(&self, base_slot: u64, index: u64) -> H256 {
		let slot_bytes = U256::from(base_slot).to_big_endian();
		let base_key = H::keccak256(&slot_bytes);
		let base_pos = U256::from_big_endian(&base_key.0);
		let element_pos = base_pos + U256::from(index);
		H256(element_pos.to_big_endian())
	}

	/// Calculate the base storage slot for a validator in the mapping.
	pub fn validator_base_slot<H: Keccak256>(&self, pool_id: &H256) -> H256 {
		let mut data = [0u8; 64];
		data[..32].copy_from_slice(pool_id.as_bytes());
		data[32..64].copy_from_slice(&U256::from(self.validators_mapping_slot).to_big_endian());
		H::keccak256(&data)
	}

	/// Calculate the storage slot for a specific field within a Validator struct.
	pub fn validator_field_slot<H: Keccak256>(&self, pool_id: &H256, field_offset: u64) -> H256 {
		let base = self.validator_base_slot::<H>(pool_id);
		let base_pos = U256::from_big_endian(base.as_bytes());
		let field_pos = base_pos + U256::from(field_offset);
		H256(field_pos.to_big_endian())
	}

	/// Calculate the storage slot for string data.
	pub fn string_data_slot<H: Keccak256>(&self, string_slot: &H256) -> H256 {
		H::keccak256(string_slot.as_bytes())
	}

	/// Get storage keys for a specific validator's data.
	///
	/// Returns keys for:
	/// - BLS public key string slot (offset 3)
	/// - BLS public key data slots (3 slots for long string at keccak256(string_slot))
	/// - totalStake (offset 8)
	pub fn get_validator_keys<H: Keccak256>(&self, pool_id: &H256) -> Vec<H256> {
		let offsets = ValidatorStructOffsets::default();
		let mut keys = Vec::new();

		// BLS public key string slot (stores length for long strings)
		let bls_string_slot = self.validator_field_slot::<H>(pool_id, offsets.bls_public_key);
		keys.push(bls_string_slot);

		// BLS public key data slots (for long strings)
		// Data is stored at keccak256(string_slot) and continues for 3 slots
		// (96-char hex string needs ceil(96/32) = 3 slots)
		let bls_data_base = self.string_data_slot::<H>(&bls_string_slot);
		let bls_data_base_pos = U256::from_big_endian(bls_data_base.as_bytes());
		for i in 0..BLS_STRING_DATA_SLOTS {
			let slot_pos = bls_data_base_pos + U256::from(i);
			keys.push(H256(slot_pos.to_big_endian()));
		}

		// totalStake field
		keys.push(self.validator_field_slot::<H>(pool_id, offsets.total_stake));

		keys
	}
}

/// Number of storage slots needed for BLS public key string data.
/// BLS keys are 48 bytes = 96 hex chars = 3 slots (ceil(96/32))
const BLS_STRING_DATA_SLOTS: u64 = 3;

/// Decode a U256 value from RLP-encoded storage value.
pub fn decode_u256_from_storage(value: &[u8]) -> Result<U256, Error> {
	if value.is_empty() {
		return Ok(U256::zero());
	}

	// Storage values are RLP encoded
	// integers are stored as big-endian bytes
	if value.len() <= 32 {
		let mut padded = [0u8; 32];
		padded[32 - value.len()..].copy_from_slice(value);
		Ok(U256::from_big_endian(&padded))
	} else {
		Err(Error::StorageValueTooLarge)
	}
}
