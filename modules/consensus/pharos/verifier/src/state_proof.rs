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
use alloy_rlp::Decodable;
use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use ethereum_triedb::{EIP1186Layout, StorageProof};
use hash256_std_hasher::Hash256StdHasher;
use hash_db::Hasher;
use ismp::messaging::Keccak256;
use pharos_primitives::{
	ValidatorInfo, ValidatorSet, ValidatorSetProof, STAKING_CONTRACT_ADDRESS,
};
use primitive_types::{H160, H256, U256};
use trie_db::{DBValue, Trie, TrieDBBuilder};

/// Keccak256 hasher for trie operations.
pub struct KeccakHasher<H: Keccak256>(core::marker::PhantomData<H>);

impl<H: Keccak256 + Send + Sync> Hasher for KeccakHasher<H> {
	type Out = H256;
	type StdHasher = Hash256StdHasher;
	const LENGTH: usize = 32;

	fn hash(x: &[u8]) -> Self::Out {
		H::keccak256(x)
	}
}

/// The ethereum account stored in the global state trie.
#[derive(RlpDecodable, RlpEncodable, Debug, Clone)]
pub struct Account {
	pub nonce: u64,
	pub balance: alloy_primitives::U256,
	pub storage_root: alloy_primitives::B256,
	pub code_hash: alloy_primitives::B256,
}

/// This function verifies that the provided validator set is correctly stored
/// in the staking contract at the given block.
pub fn verify_validator_set_proof<H: Keccak256 + Send + Sync>(
	state_root: H256,
	proof: &ValidatorSetProof,
) -> Result<(), Error> {
	let account = get_staking_contract_account::<H>(&proof.account_proof, state_root)?;
	let storage_root = H256::from_slice(account.storage_root.as_slice());

	let layout = StakingContractLayout::default();
	let validator_count = proof.validator_set.validators.len() as u64;

	// Compute and fetch global storage slots (epoch, totalStake, array length, pool IDs)
	let global_slots = layout.get_validator_set_keys::<H>(validator_count);
	let global_values = get_values_from_proof::<H>(
		global_slots.iter().map(|s| H::keccak256(s.as_bytes()).0.to_vec()).collect(),
		storage_root,
		proof.storage_proof.clone(),
	)?;

	// Extract pool IDs from the proven values
	let pool_ids_start = 3; // After epoch, totalStake, array length
	let mut pool_ids = Vec::new();
	for i in 0..validator_count as usize {
		let pool_id = global_values[pool_ids_start + i]
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

	let mut validator_slots = Vec::new();
	for pool_id in &pool_ids {
		validator_slots.extend(layout.get_validator_keys::<H>(pool_id));
	}

	let validator_values = get_values_from_proof::<H>(
		validator_slots.iter().map(|s| H::keccak256(s.as_bytes()).0.to_vec()).collect(),
		storage_root,
		proof.storage_proof.clone(),
	)?;

	let mut all_slots = global_slots;
	all_slots.extend(validator_slots);

	let mut all_values = global_values;
	all_values.extend(validator_values);

	let decoded_set = decode_validator_set_from_storage::<H>(&all_slots, &all_values)?;

	verify_validator_set_matches(&proof.validator_set, &decoded_set)?;

	Ok(())
}

/// Get the staking contract account from the account proof.
fn get_staking_contract_account<H: Keccak256 + Send + Sync>(
	account_proof: &[Vec<u8>],
	state_root: H256,
) -> Result<Account, Error> {
	let db = StorageProof::new(account_proof.to_vec()).into_memory_db::<KeccakHasher<H>>();
	let trie = TrieDBBuilder::<EIP1186Layout<KeccakHasher<H>>>::new(&db, &state_root).build();

	let contract_address = STAKING_CONTRACT_ADDRESS.as_slice();
	let key = H::keccak256(contract_address).0;

	let result = trie
		.get(&key)
		.map_err(|_| Error::AccountTrieLookupFailed)?
		.ok_or(Error::StakingContractNotFound)?;

	let account = Account::decode(&mut &*result).map_err(|_| Error::AccountRlpDecodeFailed)?;

	Ok(account)
}

/// Get values from storage proof.
fn get_values_from_proof<H: Keccak256 + Send + Sync>(
	keys: Vec<Vec<u8>>,
	storage_root: H256,
	proof: Vec<Vec<u8>>,
) -> Result<Vec<Option<DBValue>>, Error> {
	let proof_db = StorageProof::new(proof).into_memory_db::<KeccakHasher<H>>();
	let trie =
		TrieDBBuilder::<EIP1186Layout<KeccakHasher<H>>>::new(&proof_db, &storage_root).build();

	let mut values = Vec::new();
	for key in keys {
		let val = trie.get(&key).map_err(|_| Error::StorageProofLookupFailed)?;
		values.push(val);
	}

	Ok(values)
}

/// Decode validator set from storage values.
///
/// This function interprets the raw storage values according to the
/// Pharos staking contract's actual storage layout.
///
/// ## Pharos Storage Layout
///
/// The contract at `0x4100000000000000000000000000000000000000` uses:
/// - Slot 5: currentEpoch (uint256)
/// - Slot 6: totalStake (uint256)
/// - Slot 1: activePoolIds array (length at slot, elements at keccak256(slot))
/// - Slot 0: validators mapping (mapping(bytes32 => Validator))
fn decode_validator_set_from_storage<H: Keccak256>(
	slots: &[H256],
	values: &[Option<DBValue>],
) -> Result<ValidatorSet, Error> {
	if slots.len() != values.len() {
		return Err(Error::SlotValueLengthMismatch { slots: slots.len(), values: values.len() });
	}

	// We need 3 slots at minimum. currentEpoch, totalStake, activePoolIds length (3 slots)
	if values.len() < 3 {
		return Err(Error::InsufficientStorageValues { expected: 3, got: values.len() });
	}

	// Parse global state
	// Index 0: currentEpoch
	let epoch = values[0]
		.as_ref()
		.map(|v| decode_u256_from_storage(v))
		.transpose()?
		.unwrap_or_default();

	// Index 1: totalStake
	let total_stake = values[1]
		.as_ref()
		.map(|v| decode_u256_from_storage(v))
		.transpose()?
		.unwrap_or_default();

	// Index 2: activePoolIds array length
	let validator_count = values[2]
		.as_ref()
		.map(|v| decode_u256_from_storage(v))
		.transpose()?
		.unwrap_or_default();

	let count = validator_count.low_u64() as usize;
	let epoch_num = epoch.low_u64();

	// Each validator needs: poolId (1) + validator data (4) = 5 slots
	// But the proof structure may vary based on what's included
	let pool_ids_start = 3;
	let pool_ids_end = pool_ids_start + count;

	if values.len() < pool_ids_end {
		return Err(Error::InsufficientPoolIds {
			expected: pool_ids_end,
			validators: count,
			got: values.len(),
		});
	}

	let mut validator_set = ValidatorSet::new(epoch_num);
	validator_set.total_stake = total_stake;

	// Slots per validator in the detailed proof
	const VALIDATOR_FIELDS: usize = 4; // blsPublicKey slot, totalStake, owner, poolId

	let validators_data_start = pool_ids_end;
	let expected_total = validators_data_start + (count * VALIDATOR_FIELDS);

	// If we have detailed validator data
	if values.len() >= expected_total {
		for i in 0..count {
			// Pool ID from activePoolIds array
			let pool_id = values[pool_ids_start + i]
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

			// BLS public key is stored as a string in Solidity
			// The slot contains either the string data (if short) or a pointer
			// For now, we expect the proof to include the decoded BLS key bytes
			let bls_key = decode_bls_key_from_string_slot(&values[base_idx])?;

			// totalStake
			let stake = values[base_idx + 1]
				.as_ref()
				.map(|v| decode_u256_from_storage(v))
				.transpose()?
				.unwrap_or_default();

			// owner (address, 20 bytes, right-aligned in 32-byte slot)
			let address = values[base_idx + 2]
				.as_ref()
				.map(|v| {
					if v.len() >= 20 {
						let start = v.len() - 20;
						H160::from_slice(&v[start..])
					} else {
						let mut padded = [0u8; 20];
						if !v.is_empty() {
							padded[20 - v.len()..].copy_from_slice(v);
						}
						H160::from_slice(&padded)
					}
				})
				.unwrap_or_default();

			let validator = ValidatorInfo { address, bls_public_key: bls_key, pool_id, stake };

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
/// - Short strings (< 32 bytes): data is stored directly in the slot
/// - Long strings (>= 32 bytes): slot contains (length * 2 + 1), data at keccak256(slot)
///
/// The BLS public key is a 48-byte value, typically stored as a 96-character hex string
/// (or 98 with "0x" prefix), so it's a long string.
fn decode_bls_key_from_string_slot(
	value: &Option<DBValue>,
) -> Result<pharos_primitives::BlsPublicKey, Error> {
	use alloc::string::String;

	let value = value.as_ref().ok_or(Error::MissingBlsKeySlot)?;

	// Check if this is a short string (last byte is even and < 64)
	// or a long string (last byte is odd)
	if value.is_empty() {
		return Err(Error::EmptyBlsKeySlot);
	}

	let last_byte = value[value.len() - 1];

	let bls_hex: String = if last_byte & 1 == 0 {
		// Short string: data is in the slot, length = last_byte / 2
		let len = (last_byte / 2) as usize;
		if len > 31 || len > value.len() {
			return Err(Error::InvalidBlsStringLength);
		}
		// String data is at the beginning of the slot
		String::from_utf8(value[..len].to_vec()).map_err(|_| Error::InvalidBlsKeyUtf8)?
	} else {
		// Long string: this slot contains (length * 2 + 1)
		return Err(Error::LongStringBlsKeyUnsupported);
	};

	let bls_hex = bls_hex.trim_start_matches("0x");
	let bls_bytes = hex::decode(bls_hex).map_err(|_| Error::InvalidBlsKeyHex)?;

	if bls_bytes.len() != 48 {
		return Err(Error::InvalidBlsKeyLength { expected: 48, got: bls_bytes.len() });
	}

	bls_bytes.try_into().map_err(|_| Error::BlsKeyConversionFailed)
}

/// Verify that the decoded validator set matches the claimed set.
fn verify_validator_set_matches(
	claimed: &ValidatorSet,
	decoded: &ValidatorSet,
) -> Result<(), Error> {
	validate_validator_set(claimed)?;

	if claimed.epoch != decoded.epoch {
		return Err(Error::ValidatorSetEpochMismatch { claimed: claimed.epoch, decoded: decoded.epoch });
	}

	if claimed.validators.len() != decoded.validators.len() {
		return Err(Error::ValidatorCountMismatch {
			claimed: claimed.validators.len(),
			decoded: decoded.validators.len(),
		});
	}

	if claimed.total_stake != decoded.total_stake {
		return Err(Error::TotalStakeMismatch {
			claimed: claimed.total_stake,
			decoded: decoded.total_stake,
		});
	}

	for (i, (c, d)) in claimed.validators.iter().zip(decoded.validators.iter()).enumerate() {
		if c.address != d.address {
			return Err(Error::ValidatorAddressMismatch { index: i });
		}
		if c.bls_public_key != d.bls_public_key {
			return Err(Error::ValidatorBlsKeyMismatch { index: i });
		}
		if c.pool_id != d.pool_id {
			return Err(Error::ValidatorPoolIdMismatch { index: i });
		}
		if c.stake != d.stake {
			return Err(Error::ValidatorStakeMismatch { index: i, claimed: c.stake, decoded: d.stake });
		}
	}

	Ok(())
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
/// https://silken-muskox-24e.notion.site/Pharos-Staking-Contract-2b18ec314f7580c1b885e2fa8d8a70e9
///
/// ## Contract Storage Layout
///
/// ```solidity
/// abstract contract StakingStorageV1 is IStaking {
///     mapping(bytes32 => Validator) public validators;  // slot 0
///     bytes32[] public activePoolIds;                   // slot 1
///     bytes32[] public pendingAddPoolIds;               // slot 2
///     bytes32[] public pendingUpdatePoolIds;            // slot 3
///     bytes32[] public pendingExitPoolIds;              // slot 4
///     uint256 public currentEpoch;                      // slot 5
///     uint256 public totalStake;                        // slot 6
///     IChainConfig public cfg;                          // slot 7
///     mapping(address => uint256) public pendingWithdrawStakes; // slot 8
/// }
///
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
	/// Storage slot for activePoolIds array
	pub active_pool_ids_slot: u64,
	/// Storage slot for currentEpoch
	pub current_epoch_slot: u64,
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
		Self {
			validators_mapping_slot: 0,
			active_pool_ids_slot: 1,
			current_epoch_slot: 5,
			total_stake_slot: 6,
		}
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

	/// Get storage keys needed to read the validator set.
	pub fn get_validator_set_keys<H: Keccak256>(&self, validator_count: u64) -> Vec<H256> {
		let mut keys = Vec::new();

		keys.push(self.raw_slot_key(self.current_epoch_slot));
		keys.push(self.raw_slot_key(self.total_stake_slot));

		keys.push(self.raw_slot_key(self.active_pool_ids_slot));

		for i in 0..validator_count {
			keys.push(self.array_element_key::<H>(self.active_pool_ids_slot, i));
		}

		keys
	}

	/// Get storage keys for a specific validator's data.
	pub fn get_validator_keys<H: Keccak256>(&self, pool_id: &H256) -> Vec<H256> {
		let offsets = ValidatorStructOffsets::default();
		let mut keys = Vec::new();

		keys.push(self.validator_field_slot::<H>(pool_id, offsets.bls_public_key));
		keys.push(self.validator_field_slot::<H>(pool_id, offsets.total_stake));
		keys.push(self.validator_field_slot::<H>(pool_id, offsets.owner));
		keys.push(self.validator_field_slot::<H>(pool_id, offsets.pool_id));

		keys
	}
}

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
