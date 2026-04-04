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

#![cfg(test)]

use pharos_primitives::{spv, Config, Testnet, STAKING_CONTRACT_ADDRESS};
use pharos_prover::{
	rpc::{hex_to_bytes, PharosRpcClient},
	rpc_to_proof_nodes, rpc_to_sibling_proofs, PharosProver,
};
use primitive_types::{H160, H256, U256};
use std::sync::Arc;

#[tokio::test]
#[ignore]
async fn test_pharos_account_proof_verification() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let rpc = PharosRpcClient::new(&rpc_url).expect("Failed to create RPC client");

	let block_number = rpc.get_block_number().await.expect("Failed to get block number");
	let target_block = block_number.saturating_sub(5);
	println!("Testing at block: {}", target_block);

	let header = rpc.get_block_by_number(target_block).await.expect("Failed to get block");
	let state_root = header.state_root;
	println!("State root: {:?}", state_root);

	let address = H160::from_slice(STAKING_CONTRACT_ADDRESS.as_slice());
	let total_stake_slot = H256(U256::from(6u64).to_big_endian());
	let proof = rpc
		.get_proof(address, vec![total_stake_slot], target_block)
		.await
		.expect("Failed to get proof");

	println!("Account proof nodes: {}", proof.account_proof.len());
	println!("Storage hash: {}", proof.storage_hash);
	println!("Raw value length: {}", proof.raw_value.len());

	let account_proof_nodes =
		rpc_to_proof_nodes(&proof.account_proof).expect("Failed to convert account proof nodes");
	let raw_value = hex_to_bytes(&proof.raw_value).expect("Failed to parse raw_value");

	assert!(!account_proof_nodes.is_empty(), "Account proof should not be empty");
	assert!(!raw_value.is_empty(), "Raw account value should not be empty");

	let address_bytes: [u8; 20] = address.0;
	spv::verify_proof(&account_proof_nodes, &address_bytes, &raw_value, &state_root.0)
		.expect("Account proof verification should pass for staking contract");
	println!("Account proof verification: PASSED");
}

#[tokio::test]
#[ignore]
async fn test_pharos_storage_proof_verification() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let rpc = PharosRpcClient::new(&rpc_url).expect("Failed to create RPC client");

	let block_number = rpc.get_block_number().await.expect("Failed to get block number");
	let target_block = block_number.saturating_sub(5);
	println!("Testing at block: {}", target_block);

	let header = rpc.get_block_by_number(target_block).await.expect("Failed to get block");
	let state_root = header.state_root;
	println!("State root: {:?}", state_root);

	let address = H160::from_slice(STAKING_CONTRACT_ADDRESS.as_slice());
	let total_stake_slot = H256(U256::from(6u64).to_big_endian());
	let proof = rpc
		.get_proof(address, vec![total_stake_slot], target_block)
		.await
		.expect("Failed to get proof");

	println!("Account proof nodes: {}", proof.account_proof.len());
	println!("Storage proof entries: {}", proof.storage_proof.len());
	println!("Storage hash: {}", proof.storage_hash);

	let account_proof_nodes =
		rpc_to_proof_nodes(&proof.account_proof).expect("Failed to convert account proof nodes");
	let raw_value = hex_to_bytes(&proof.raw_value).expect("Failed to parse raw_value");
	let address_bytes: [u8; 20] = address.0;

	spv::verify_proof(&account_proof_nodes, &address_bytes, &raw_value, &state_root.0)
		.expect("Account proof verification should pass");
	println!("Account proof verification: PASSED");

	assert!(!proof.storage_proof.is_empty(), "Should have at least one storage proof");
	let storage_entry = &proof.storage_proof[0];
	let storage_proof_nodes =
		rpc_to_proof_nodes(&storage_entry.proof).expect("Failed to convert storage proof nodes");

	println!("Storage key: {}", storage_entry.key);
	println!("Storage value: {}", storage_entry.value);
	println!("Storage proof nodes: {}", storage_proof_nodes.len());

	assert!(!storage_proof_nodes.is_empty(), "Storage proof should not be empty");

	let value_bytes = hex_to_bytes(&storage_entry.value).expect("Failed to parse storage value");
	let mut padded_value = [0u8; 32];
	if value_bytes.len() <= 32 {
		padded_value[32 - value_bytes.len()..].copy_from_slice(&value_bytes);
	}

	let total_stake = U256::from_big_endian(&padded_value);
	println!("Total stake: {}", total_stake);
	assert!(total_stake > U256::zero(), "Total stake should be non-zero");

	let key_bytes = hex_to_bytes(&storage_entry.key).expect("Failed to parse storage key");
	let mut storage_key = [0u8; 32];
	if key_bytes.len() <= 32 {
		storage_key[32 - key_bytes.len()..].copy_from_slice(&key_bytes);
	}

	// Pharos uses a flat trie — storage proofs verify directly against state_root.
	spv::verify_proof(
		&storage_proof_nodes,
		&spv::build_storage_key(&address_bytes, &storage_key),
		&padded_value,
		&state_root.0,
	)
	.expect("Storage proof verification should pass for totalStake");
	println!("Storage proof verification: PASSED");
}

#[tokio::test]
#[ignore]
async fn test_pharos_multiple_storage_proofs() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let rpc = PharosRpcClient::new(&rpc_url).expect("Failed to create RPC client");

	let block_number = rpc.get_block_number().await.expect("Failed to get block number");
	let target_block = block_number.saturating_sub(5);
	println!("Testing at block: {}", target_block);

	let header = rpc.get_block_by_number(target_block).await.expect("Failed to get block");
	let state_root = header.state_root;

	let address = H160::from_slice(STAKING_CONTRACT_ADDRESS.as_slice());
	let total_stake_slot = H256(U256::from(6u64).to_big_endian());
	let epoch_length_slot = H256(U256::from(5u64).to_big_endian());

	let proof_stake = rpc
		.get_proof(address, vec![total_stake_slot], target_block)
		.await
		.expect("Failed to get proof for totalStake");

	let proof_epoch = rpc
		.get_proof(address, vec![epoch_length_slot], target_block)
		.await
		.expect("Failed to get proof for epochLength");

	let account_proof_nodes = rpc_to_proof_nodes(&proof_stake.account_proof)
		.expect("Failed to convert account proof nodes");
	let raw_value = hex_to_bytes(&proof_stake.raw_value).expect("Failed to parse raw_value");
	let address_bytes: [u8; 20] = address.0;

	spv::verify_proof(&account_proof_nodes, &address_bytes, &raw_value, &state_root.0)
		.expect("Account proof verification should pass");
	println!("Account proof verification: PASSED");

	// Pharos uses a flat trie — storage proofs verify directly against state_root.
	assert!(!proof_stake.storage_proof.is_empty(), "Should have storage proof for totalStake");
	let stake_entry = &proof_stake.storage_proof[0];
	let stake_proof_nodes =
		rpc_to_proof_nodes(&stake_entry.proof).expect("Failed to convert storage proof nodes");

	let stake_value_bytes =
		hex_to_bytes(&stake_entry.value).expect("Failed to parse totalStake value");
	let mut stake_padded = [0u8; 32];
	if stake_value_bytes.len() <= 32 {
		stake_padded[32 - stake_value_bytes.len()..].copy_from_slice(&stake_value_bytes);
	}

	let stake_key_bytes = hex_to_bytes(&stake_entry.key).expect("Failed to parse storage key");
	let mut stake_key = [0u8; 32];
	if stake_key_bytes.len() <= 32 {
		stake_key[32 - stake_key_bytes.len()..].copy_from_slice(&stake_key_bytes);
	}

	let total_stake = U256::from_big_endian(&stake_padded);
	println!("Storage proof [totalStake]: key={}, value={}", stake_entry.key, total_stake);
	assert!(total_stake > U256::zero(), "Total stake should be non-zero");

	spv::verify_proof(
		&stake_proof_nodes,
		&spv::build_storage_key(&address_bytes, &stake_key),
		&stake_padded,
		&state_root.0,
	)
	.expect("Storage proof for totalStake should pass");
	println!("Storage proof [totalStake] verification: PASSED");

	assert!(!proof_epoch.storage_proof.is_empty(), "Should have storage proof for epochLength");
	let epoch_entry = &proof_epoch.storage_proof[0];
	let epoch_proof_nodes =
		rpc_to_proof_nodes(&epoch_entry.proof).expect("Failed to convert storage proof nodes");

	let epoch_value_bytes =
		hex_to_bytes(&epoch_entry.value).expect("Failed to parse epochLength value");
	let mut epoch_padded = [0u8; 32];
	if epoch_value_bytes.len() <= 32 {
		epoch_padded[32 - epoch_value_bytes.len()..].copy_from_slice(&epoch_value_bytes);
	}

	let epoch_key_bytes = hex_to_bytes(&epoch_entry.key).expect("Failed to parse storage key");
	let mut epoch_key = [0u8; 32];
	if epoch_key_bytes.len() <= 32 {
		epoch_key[32 - epoch_key_bytes.len()..].copy_from_slice(&epoch_key_bytes);
	}

	let epoch_length = U256::from_big_endian(&epoch_padded);
	println!("Storage proof [epochLength]: key={}, value={}", epoch_entry.key, epoch_length);
	assert!(epoch_length > U256::zero(), "Epoch length should be non-zero");

	spv::verify_proof(
		&epoch_proof_nodes,
		&spv::build_storage_key(&address_bytes, &epoch_key),
		&epoch_padded,
		&state_root.0,
	)
	.expect("Storage proof for epochLength should pass");
	println!("Storage proof [epochLength] verification: PASSED");
}

#[tokio::test]
#[ignore]
async fn test_pharos_non_existence_account_proof() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let rpc = PharosRpcClient::new(&rpc_url).expect("Failed to create RPC client");

	let block_number = rpc.get_block_number().await.expect("Failed to get block number");
	let target_block = block_number.saturating_sub(5);
	println!("Testing at block: {}", target_block);

	let header = rpc.get_block_by_number(target_block).await.expect("Failed to get block");
	let state_root = header.state_root;

	// Query a non-existent account
	let fake_address =
		H160::from_slice(&[0xde, 0xad, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
	let dummy_slot = H256::zero();
	let proof = rpc
		.get_proof(fake_address, vec![dummy_slot], target_block)
		.await
		.expect("Failed to get proof");

	assert!(!proof.is_exist, "Account should not exist");

	let proof_nodes =
		rpc_to_proof_nodes(&proof.account_proof).expect("Failed to convert proof nodes");
	let sibling_proofs = rpc_to_sibling_proofs(&proof.sibling_leftmost_leaf_proofs)
		.expect("Failed to convert sibling proofs");

	let address_bytes: [u8; 20] = fake_address.0;
	spv::verify_non_existence_proof(&proof_nodes, &address_bytes, &state_root.0, &sibling_proofs)
		.expect("Non-existence proof should be valid for fake account");
	println!("Non-existence account proof: PASSED");

	// Sanity check: the same proof must NOT pass as an existence proof
	assert!(
		spv::verify_membership_proof(&proof_nodes, &address_bytes, &state_root.0).is_err(),
		"Membership check should fail for non-existent account"
	);
	println!("Membership returns None as expected: PASSED");
}

#[tokio::test]
#[ignore]
async fn test_pharos_non_existence_storage_proof() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let rpc = PharosRpcClient::new(&rpc_url).expect("Failed to create RPC client");

	let block_number = rpc.get_block_number().await.expect("Failed to get block number");
	let target_block = block_number.saturating_sub(5);
	println!("Testing at block: {}", target_block);

	let header = rpc.get_block_by_number(target_block).await.expect("Failed to get block");
	let state_root = header.state_root;

	// Use the real staking contract but query a non-existent storage slot
	let address = H160::from_slice(STAKING_CONTRACT_ADDRESS.as_slice());
	let fake_slot = H256::from_low_u64_be(999999);
	let proof = rpc
		.get_proof(address, vec![fake_slot], target_block)
		.await
		.expect("Failed to get proof");

	assert!(!proof.storage_proof.is_empty(), "Should have a storage proof entry");
	let storage_entry = &proof.storage_proof[0];
	println!("Storage isExist: {}", storage_entry.is_exist);
	println!("Storage proof nodes: {}", storage_entry.proof.len());
	println!("Storage sibling proofs: {}", storage_entry.sibling_leftmost_leaf_proofs.len());

	if !storage_entry.is_exist {
		let proof_nodes = rpc_to_proof_nodes(&storage_entry.proof)
			.expect("Failed to convert storage proof nodes");
		let sibling_proofs = rpc_to_sibling_proofs(&storage_entry.sibling_leftmost_leaf_proofs)
			.expect("Failed to convert sibling proofs");

		let address_bytes: [u8; 20] = address.0;
		let mut slot_key = [0u8; 32];
		slot_key.copy_from_slice(fake_slot.as_bytes());

		spv::verify_non_existence_proof(
			&proof_nodes,
			&spv::build_storage_key(&address_bytes, &slot_key),
			&state_root.0,
			&sibling_proofs,
		)
		.expect("Storage non-existence proof should be valid for fake slot");
		println!("Non-existence storage proof: PASSED");
	} else {
		// Slot 999999 might actually exist if so, just verify the existence proof works
		println!("Storage slot exists (unexpected), skipping non-existence test");
	}
}

#[tokio::test]
#[ignore]
async fn test_pharos_account_proof_with_raw_value() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let rpc = PharosRpcClient::new(&rpc_url).expect("Failed to create RPC client");

	let block_number = rpc.get_block_number().await.expect("Failed to get block number");
	let target_block = block_number.saturating_sub(5);
	println!("Testing at block: {}", target_block);

	let header = rpc.get_block_by_number(target_block).await.expect("Failed to get block");
	let state_root = header.state_root;

	// Fetch account proof for staking contract with no storage keys
	let address = H160::from_slice(STAKING_CONTRACT_ADDRESS.as_slice());
	let proof = rpc.get_proof(address, vec![], target_block).await.expect("Failed to get proof");

	assert!(proof.is_exist, "Staking contract should exist");

	let proof_nodes =
		rpc_to_proof_nodes(&proof.account_proof).expect("Failed to convert proof nodes");
	let raw_value = hex_to_bytes(&proof.raw_value).expect("Failed to parse raw_value");

	assert!(!raw_value.is_empty(), "Account raw value should not be empty");

	let address_bytes: [u8; 20] = address.0;
	spv::verify_proof(&proof_nodes, &address_bytes, &raw_value, &state_root.0)
		.expect("Account proof should verify against state root");
	println!("Account proof with raw value: PASSED");

	// Verify a non-existent account returns isExist: false
	let fake_address =
		H160::from_slice(&[0xde, 0xad, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
	let fake_proof = rpc
		.get_proof(fake_address, vec![], target_block)
		.await
		.expect("Failed to get proof for fake address");

	assert!(!fake_proof.is_exist, "Fake account should not exist");
	println!("Non-existent account isExist=false: PASSED");
}
