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

use pharos_primitives::{spv, Config, PharosProofNode, Testnet, STAKING_CONTRACT_ADDRESS};
use pharos_prover::{
	rpc::{hex_to_bytes, hex_to_h256, PharosRpcClient, RpcProofNode},
	PharosProver,
};
use primitive_types::{H160, H256, U256};
use std::sync::Arc;

const ATLANTIC_RPC: &str = "https://atlantic.dplabs-internal.com";

/// Convert RPC proof nodes to PharosProofNode format.
fn rpc_to_proof_nodes(nodes: &[RpcProofNode]) -> Vec<PharosProofNode> {
	nodes
		.iter()
		.filter_map(|n| {
			let proof_node = hex_to_bytes(&n.proof_node).ok()?;
			Some(PharosProofNode {
				proof_node,
				next_begin_offset: n.next_begin_offset,
				next_end_offset: n.next_end_offset,
			})
		})
		.collect()
}

#[tokio::test]
#[ignore]
async fn test_pharos_account_proof_verification() {
	let rpc = PharosRpcClient::new(ATLANTIC_RPC);

	let block_number = rpc.get_block_number().await.expect("Failed to get block number");
	let target_block = block_number.saturating_sub(5);
	println!("Testing at block: {}", target_block);

	let block = rpc.get_block_by_number(target_block).await.expect("Failed to get block");
	let state_root = hex_to_h256(&block.state_root).expect("Failed to parse state_root");
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

	let account_proof_nodes = rpc_to_proof_nodes(&proof.account_proof);
	let raw_value = hex_to_bytes(&proof.raw_value).expect("Failed to parse raw_value");

	assert!(!account_proof_nodes.is_empty(), "Account proof should not be empty");
	assert!(!raw_value.is_empty(), "Raw account value should not be empty");

	let address_bytes: [u8; 20] = address.0;
	let is_valid =
		spv::verify_account_proof(&account_proof_nodes, &address_bytes, &raw_value, &state_root.0);

	assert!(is_valid, "Account proof verification should pass for staking contract");
	println!("Account proof verification: PASSED");
}

#[tokio::test]
#[ignore]
async fn test_pharos_storage_proof_verification() {
	let rpc = PharosRpcClient::new(ATLANTIC_RPC);

	let block_number = rpc.get_block_number().await.expect("Failed to get block number");
	let target_block = block_number.saturating_sub(5);
	println!("Testing at block: {}", target_block);

	let block = rpc.get_block_by_number(target_block).await.expect("Failed to get block");
	let state_root = hex_to_h256(&block.state_root).expect("Failed to parse state_root");
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

	let account_proof_nodes = rpc_to_proof_nodes(&proof.account_proof);
	let raw_value = hex_to_bytes(&proof.raw_value).expect("Failed to parse raw_value");
	let address_bytes: [u8; 20] = address.0;

	let account_valid =
		spv::verify_account_proof(&account_proof_nodes, &address_bytes, &raw_value, &state_root.0);
	assert!(account_valid, "Account proof verification should pass");
	println!("Account proof verification: PASSED");

	assert!(!proof.storage_proof.is_empty(), "Should have at least one storage proof");
	let storage_entry = &proof.storage_proof[0];
	let storage_proof_nodes = rpc_to_proof_nodes(&storage_entry.proof);
	let storage_hash = hex_to_h256(&proof.storage_hash).expect("Failed to parse storage_hash");

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

	let storage_valid = spv::verify_storage_proof(
		&storage_proof_nodes,
		&address_bytes,
		&storage_key,
		&padded_value,
		&storage_hash.0,
	);

	assert!(storage_valid, "Storage proof verification should pass for totalStake");
	println!("Storage proof verification: PASSED");
}

#[tokio::test]
#[ignore]
async fn test_pharos_multiple_storage_proofs() {
	let rpc = PharosRpcClient::new(ATLANTIC_RPC);

	let block_number = rpc.get_block_number().await.expect("Failed to get block number");
	let target_block = block_number.saturating_sub(5);
	println!("Testing at block: {}", target_block);

	let block = rpc.get_block_by_number(target_block).await.expect("Failed to get block");
	let state_root = hex_to_h256(&block.state_root).expect("Failed to parse state_root");

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

	let account_proof_nodes = rpc_to_proof_nodes(&proof_stake.account_proof);
	let raw_value = hex_to_bytes(&proof_stake.raw_value).expect("Failed to parse raw_value");
	let address_bytes: [u8; 20] = address.0;

	let account_valid =
		spv::verify_account_proof(&account_proof_nodes, &address_bytes, &raw_value, &state_root.0);
	assert!(account_valid, "Account proof verification should pass");
	println!("Account proof verification: PASSED");

	let storage_hash =
		hex_to_h256(&proof_stake.storage_hash).expect("Failed to parse storage_hash");
	assert!(!proof_stake.storage_proof.is_empty(), "Should have storage proof for totalStake");
	let stake_entry = &proof_stake.storage_proof[0];
	let stake_proof_nodes = rpc_to_proof_nodes(&stake_entry.proof);

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

	let stake_valid = spv::verify_storage_proof(
		&stake_proof_nodes,
		&address_bytes,
		&stake_key,
		&stake_padded,
		&storage_hash.0,
	);
	assert!(stake_valid, "Storage proof for totalStake should pass");
	println!("Storage proof [totalStake] verification: PASSED");

	let epoch_storage_hash =
		hex_to_h256(&proof_epoch.storage_hash).expect("Failed to parse storage_hash");
	assert!(
		!proof_epoch.storage_proof.is_empty(),
		"Should have storage proof for epochLength"
	);
	let epoch_entry = &proof_epoch.storage_proof[0];
	let epoch_proof_nodes = rpc_to_proof_nodes(&epoch_entry.proof);

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

	let epoch_valid = spv::verify_storage_proof(
		&epoch_proof_nodes,
		&address_bytes,
		&epoch_key,
		&epoch_padded,
		&epoch_storage_hash.0,
	);
	assert!(epoch_valid, "Storage proof for epochLength should pass");
	println!("Storage proof [epochLength] verification: PASSED");
}
