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
//! This module handles verification of storage proofs for the validator set
//! stored in the staking contract at `0x4100000000000000000000000000000000000000`.
//!
//! Pharos uses a flat trie, so storage slot proofs verify directly against the
//! state root — no separate account proof is needed.
//!
//! ## Verification Steps
//!
//! 1. Recompute expected storage keys from the storage values
//! 2. Verify each storage value against its per-key proof path and the state root
//! 3. Decode the verified storage values into a ValidatorSet

use crate::error::Error;
use alloc::{collections::BTreeMap, vec::Vec};
use ismp::messaging::Keccak256;
use pharos_primitives::{
	spv, PharosProofNode, ValidatorInfo, ValidatorSet, ValidatorSetProof, STAKING_CONTRACT_ADDRESS,
};
use primitive_types::{H256, U256};

/// This function verifies that the provided validator set is correctly stored
/// in the staking contract at the given block.
///
/// The `epoch` parameter is the epoch this validator set will be valid for.
///
/// Verification steps:
/// 1. Recompute expected storage keys from the storage values
/// 2. Verify each storage value against its per-key proof path and the state root
/// 3. Decode the verified storage values into a ValidatorSet
pub fn verify_validator_set_proof<H: Keccak256 + Send + Sync>(
	state_root: H256,
	proof: &ValidatorSetProof,
	epoch: u64,
) -> Result<ValidatorSet, Error> {
	let layout = StakingContractLayout::default();

	// Recompute expected storage keys from the storage values
	let keys = compute_all_storage_keys::<H>(&proof.storage_values, &layout)?;

	// Verify each storage value against its per-key proof path.
	// Pharos uses a flat trie — storage proofs verify directly against state_root.
	verify_all_storage_proofs(&keys, &proof.storage_values, &proof.storage_proof, &state_root)?;

	// Decode the verified storage values into a ValidatorSet
	let decoded_set = decode_validator_set_from_storage::<H>(&proof.storage_values, epoch)?;

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
/// - Slot 21: activePoolSets (EnumerableSet.Bytes32Set)
///
/// Values are ordered: [totalStake, poolCount, poolId_0..poolId_n,
///   validator_0_bls_header, validator_0_bls_data_0..N_0, validator_0_stake, ...]
///
/// The number of BLS data slots per validator varies based on whether the key
/// was registered with or without a "0x" prefix (3 or 4 data slots respectively).
fn decode_validator_set_from_storage<H: Keccak256>(
	values: &[Vec<u8>],
	epoch: u64,
) -> Result<ValidatorSet, Error> {
	// We need 2 values at minimum: totalStake, activePoolSets length
	if values.len() < 2 {
		return Err(Error::InsufficientStorageValues { expected: 2, got: values.len() });
	}

	// Parse global state
	// Index 0: totalStake
	let on_chain_total_stake = decode_u256_from_storage(&values[0])?;

	// Index 1: activePoolSets array length
	let validator_count = decode_u256_from_storage(&values[1])?;

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

	let mut idx = pool_ids_end;
	for i in 0..count {
		// Pool ID from activePoolSets array
		let pool_id = {
			let v = &values[pool_set_start + i];
			let mut bytes = [0u8; 32];
			if v.len() <= 32 {
				bytes[32 - v.len()..].copy_from_slice(v);
			}
			H256::from(bytes)
		};

		// BLS header at current index
		if idx >= values.len() {
			return Err(Error::InsufficientStorageValues { expected: idx + 1, got: values.len() });
		}
		let data_slots = bls_data_slots_from_header(&values[idx])?;

		let bls_string_slot = &Some(values[idx].clone());
		idx += 1;

		// BLS data slots (dynamic count)
		if idx + data_slots > values.len() {
			return Err(Error::InsufficientStorageValues {
				expected: idx + data_slots,
				got: values.len(),
			});
		}
		let bls_data_slots: Vec<Option<Vec<u8>>> =
			values[idx..idx + data_slots].iter().map(|v| Some(v.clone())).collect();
		idx += data_slots;

		let bls_key = decode_bls_key_from_string_slot(bls_string_slot, Some(&bls_data_slots))?;

		// totalStake
		if idx >= values.len() {
			return Err(Error::InsufficientStorageValues { expected: idx + 1, got: values.len() });
		}
		let stake = decode_u256_from_storage(&values[idx])?;
		idx += 1;

		let validator = ValidatorInfo { bls_public_key: bls_key, pool_id, stake };

		if !validator_set.add_validator(validator) {
			return Err(Error::DuplicateValidator);
		}
	}

	if validator_set.total_stake != on_chain_total_stake {
		return Err(Error::TotalStakeMismatch {
			computed: validator_set.total_stake,
			on_chain: on_chain_total_stake,
		});
	}

	Ok(validator_set)
}

/// Decode BLS public key from a Solidity string storage slot.
///
/// In Solidity, strings are stored as:
/// - Short strings (< 32 bytes): data is stored directly in the slot, length in lowest byte
/// - Long strings (>= 32 bytes): slot contains (length * 2 + 1), data at keccak256(slot)
///
/// The BLS public key is a 48-byte value, stored as a hex string. The number of
/// data slots varies based on whether the key includes a "0x" prefix:
/// - With prefix: 98 chars → ceil(98/32) = 4 data slots
/// - Without prefix: 96 chars → ceil(96/32) = 3 data slots
fn decode_bls_key_from_string_slot(
	header_value: &Option<Vec<u8>>,
	data_slots: Option<&[Option<Vec<u8>>]>,
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

	// The staking contract may store a prefix before the 48-byte BLS key.
	// Extract the last 48 bytes which contain the actual G1 compressed key.
	if bls_bytes.len() < 48 {
		return Err(Error::InvalidBlsKeyLength { expected: 48, got: bls_bytes.len() });
	}

	let key_start = bls_bytes.len() - 48;
	bls_bytes[key_start..].try_into().map_err(|_| Error::BlsKeyConversionFailed)
}

/// Recompute the expected storage keys in the same order as `storage_values`.
///
/// The order matches the prover's output:
/// [totalStake, activePoolSets length, pool_id_0..n,
///  validator_0_bls_header, validator_0_bls_data_0..N_0, validator_0_stake, ...]
///
/// The number of BLS data slots per validator is dynamically determined from
/// each validator's BLS string header value in `storage_values`.
fn compute_all_storage_keys<H: Keccak256>(
	storage_values: &[Vec<u8>],
	layout: &StakingContractLayout,
) -> Result<Vec<H256>, Error> {
	if storage_values.len() < 2 {
		return Err(Error::InsufficientStorageValues { expected: 2, got: storage_values.len() });
	}

	let mut keys = Vec::new();

	// Index 0: totalStake
	keys.push(layout.raw_slot_key(layout.total_stake_slot));

	// Index 1: activePoolSets length
	keys.push(layout.raw_slot_key(layout.active_pool_set_slot));

	// Parse validator count from storage_values[1]
	let count_val = decode_u256_from_storage(&storage_values[1])?;
	let count = count_val.low_u64() as usize;

	// Pool ID array element keys
	for i in 0..count {
		keys.push(layout.array_element_key::<H>(layout.active_pool_set_slot, i as u64));
	}

	// Extract pool IDs from storage values to compute validator keys
	let pool_set_start = 2;
	let pool_ids_end = pool_set_start + count;

	if storage_values.len() < pool_ids_end {
		return Err(Error::InsufficientPoolIds {
			expected: pool_ids_end,
			validators: count,
			got: storage_values.len(),
		});
	}

	// For each validator, dynamically determine the BLS data slot count
	// from the header value in storage_values
	let mut idx = pool_ids_end;
	for i in 0..count {
		let v = &storage_values[pool_set_start + i];
		let mut bytes = [0u8; 32];
		if v.len() <= 32 {
			bytes[32 - v.len()..].copy_from_slice(v);
		}
		let pool_id = H256::from(bytes);

		// The BLS header value is at the current index
		if idx >= storage_values.len() {
			return Err(Error::InsufficientStorageValues {
				expected: idx + 1,
				got: storage_values.len(),
			});
		}
		let data_slots = bls_data_slots_from_header(&storage_values[idx])?;

		let validator_keys = layout.get_validator_keys::<H>(&pool_id, data_slots);
		keys.extend(validator_keys);

		// Advance index: 1 (header) + data_slots + 1 (stake)
		idx += 1 + data_slots + 1;
	}

	Ok(keys)
}

/// Verify each storage value against its per-key proof path in the storage trie.
fn verify_all_storage_proofs(
	keys: &[H256],
	values: &[Vec<u8>],
	storage_proof: &BTreeMap<H256, Vec<PharosProofNode>>,
	storage_hash: &H256,
) -> Result<(), Error> {
	if keys.len() != values.len() {
		return Err(Error::SlotValueLengthMismatch { slots: keys.len(), values: values.len() });
	}

	let address: [u8; 20] = STAKING_CONTRACT_ADDRESS.0 .0;

	for (key, value) in keys.iter().zip(values.iter()) {
		let proof_nodes = storage_proof
			.get(key)
			.ok_or(Error::MissingStorageValue { field: "storage proof for key" })?;

		let mut padded_value = [0u8; 32];
		if value.len() <= 32 {
			padded_value[32 - value.len()..].copy_from_slice(value);
		} else {
			return Err(Error::StorageValueTooLarge);
		}

		spv::verify_proof(
			proof_nodes,
			&spv::build_storage_key(&address, &key.0),
			&padded_value,
			&storage_hash.0,
		)?;
	}

	Ok(())
}

/// Validate the internal consistency of a validator set.
pub fn validate_validator_set(validator_set: &ValidatorSet) -> Result<(), Error> {
	if validator_set.is_empty() {
		return Err(Error::EmptyValidatorSet);
	}

	for validator in validator_set.validators.values() {
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
/// bytes32[] public activePoolIds;                                 // slot 1
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
/// EnumerableSet.Bytes32Set activePoolSets;                       // slot 21-22
/// EnumerableSet.Bytes32Set pendingAddPoolSets;                   // slot 24-25
/// EnumerableSet.Bytes32Set pendingUpdatePoolSets;                // slot 26-27
/// EnumerableSet.Bytes32Set pendingExitPoolSets;                  // slot 28-29
/// ```
///
/// The contract currently uses the V2 layout. Active pool IDs are stored
/// in `activePoolSets` at slot 21 as an `EnumerableSet.Bytes32Set`
/// (slot 21 = `_inner._values` array, slot 22 = `_inner._positions` mapping).
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
	/// Storage slot for activePoolSets (EnumerableSet._inner._values)
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
		Self { validators_mapping_slot: 0, active_pool_set_slot: 21, total_stake_slot: 6 }
	}
}

impl StakingContractLayout {
	/// Calculate the raw storage key for a simple slot (no hashing).
	pub fn raw_slot_key(&self, slot: u64) -> H256 {
		H256::from_low_u64_be(slot)
	}

	/// Calculate the storage key for a dynamic array element.
	pub fn array_element_key<H: Keccak256>(&self, base_slot: u64, index: u64) -> H256 {
		self.array_element_key_with(base_slot, index, H::keccak256)
	}

	/// Non-generic variant that accepts a concrete hash function.
	pub fn array_element_key_with(
		&self,
		base_slot: u64,
		index: u64,
		keccak: impl FnOnce(&[u8]) -> H256,
	) -> H256 {
		let slot_bytes = U256::from(base_slot).to_big_endian();
		let base_key = keccak(&slot_bytes);
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
	/// - BLS public key data slots (dynamic count based on string length)
	/// - totalStake (offset 8)
	///
	/// The `bls_data_slot_count` parameter specifies how many data slots to include
	/// for the BLS public key string. This is derived from the string header value:
	/// - Keys with "0x" prefix (98 chars): ceil(98/32) = 4 slots
	/// - Keys without prefix (96 chars): ceil(96/32) = 3 slots
	pub fn get_validator_keys<H: Keccak256>(
		&self,
		pool_id: &H256,
		bls_data_slot_count: usize,
	) -> Vec<H256> {
		let offsets = ValidatorStructOffsets::default();
		let mut keys = Vec::new();

		// BLS public key string slot (stores length for long strings)
		let bls_string_slot = self.validator_field_slot::<H>(pool_id, offsets.bls_public_key);
		keys.push(bls_string_slot);

		// BLS public key data slots (for long strings)
		// Data is stored at keccak256(string_slot) for `bls_data_slot_count` slots
		let bls_data_base = self.string_data_slot::<H>(&bls_string_slot);
		let bls_data_base_pos = U256::from_big_endian(bls_data_base.as_bytes());
		for i in 0..bls_data_slot_count {
			let slot_pos = bls_data_base_pos + U256::from(i);
			keys.push(H256(slot_pos.to_big_endian()));
		}

		// totalStake field
		keys.push(self.validator_field_slot::<H>(pool_id, offsets.total_stake));

		keys
	}
}

/// Determine the number of BLS data slots from the Solidity string header value.
///
/// For long strings (>= 32 bytes), the header slot contains `length * 2 + 1`.
/// The actual byte length is `(header_value - 1) / 2`, and the number of 32-byte
/// data slots is `ceil(length / 32)`.
///
/// For short strings (< 32 bytes), the data is stored directly in the header slot
/// and no additional data slots are needed (returns 0).
pub fn bls_data_slots_from_header(header_value: &[u8]) -> Result<usize, Error> {
	let header_val = decode_u256_from_storage(header_value)?;
	let header_bytes = header_val.to_big_endian();
	let lowest_byte = header_bytes[31];

	if lowest_byte & 1 == 0 {
		// Short string - data is in the header itself
		Ok(0)
	} else {
		// Long string - header = length * 2 + 1
		let length = (header_val - 1) / 2;
		let str_len = length.low_u64() as usize;
		Ok((str_len + 31) / 32)
	}
}

/// Decode a U256 value from raw big-endian storage bytes.
pub fn decode_u256_from_storage(value: &[u8]) -> Result<U256, Error> {
	if value.is_empty() {
		return Ok(U256::zero());
	}

	if value.len() <= 32 {
		let mut padded = [0u8; 32];
		padded[32 - value.len()..].copy_from_slice(value);
		Ok(U256::from_big_endian(&padded))
	} else {
		Err(Error::StorageValueTooLarge)
	}
}
