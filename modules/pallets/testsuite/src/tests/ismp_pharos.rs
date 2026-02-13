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

use crate::runtime::{Ismp, Test};
use codec::{Decode, Encode};
use ismp::{
	consensus::{ConsensusClient, StateMachineId},
	host::StateMachine,
};
use ismp_pharos::{ConsensusState, PharosClient, PHAROS_CONSENSUS_CLIENT_ID};
use pharos_primitives::{Config, Testnet, PHAROS_ATLANTIC_CHAIN_ID};
use pharos_prover::PharosProver;
use primitive_types::H256;

const ATLANTIC_RPC: &str = "https://atlantic.dplabs-internal.com";

#[tokio::test]
#[ignore]
async fn test_ismp_pharos_non_epoch_boundary_consensus_verification() {
	let prover = PharosProver::<Testnet>::new(ATLANTIC_RPC);

	let latest_block_num = prover.get_latest_block().await.expect("Failed to get block number");
	println!("Latest block: {}", latest_block_num);

	let mut target_block = latest_block_num.saturating_sub(5);

	// ensuring we're not at an epoch boundary so as to avoid needing staking contract verification
	while Testnet::is_epoch_boundary(target_block) {
		target_block = target_block.saturating_sub(1);
	}
	println!("Target block: {}", target_block);

	let validator_info =
		prover.rpc.get_validator_info(None).await.expect("Failed to get validator info");
	println!("Validators: {}", validator_info.validator_set.len());

	let current_epoch = Testnet::compute_epoch(target_block);
	let validator_set = prover
		.build_validator_set(&validator_info.validator_set, current_epoch)
		.expect("Failed to build validator set");
	println!("Total stake: {}", validator_set.total_stake);

	let initial_block = target_block - 1;
	let initial_consensus_state = ConsensusState {
		current_validators: validator_set,
		finalized_height: initial_block,
		finalized_hash: H256::zero(),
		current_epoch,
		chain_id: PHAROS_ATLANTIC_CHAIN_ID,
	};

	let update = prover
		.fetch_block_update(target_block)
		.await
		.expect("Failed to fetch block update");
	println!("Block update is for block: {}", update.block_number());
	println!("Participant keys Length: {}", update.block_proof.participant_count());

	let pharos_client = PharosClient::<Ismp, Test, Testnet>::default();

	let host = Ismp::default();
	let result = pharos_client.verify_consensus(
		&host,
		PHAROS_CONSENSUS_CLIENT_ID,
		initial_consensus_state.encode(),
		update.encode(),
	);

	match result {
		Ok((new_state_bytes, commitments)) => {
			let new_state = ConsensusState::decode(&mut &new_state_bytes[..])
				.expect("Failed to decode new state");

			println!("\nVerification Successful");
			println!("Finalized height: {}", new_state.finalized_height);
			println!("Epoch: {}", new_state.current_epoch);

			// the epoch should remain the same
			assert_eq!(
				new_state.current_epoch, initial_consensus_state.current_epoch,
				"Epoch should not change for non-epoch-boundary blocks"
			);

			let state_id = StateMachineId {
				state_id: StateMachine::Evm(PHAROS_ATLANTIC_CHAIN_ID),
				consensus_state_id: PHAROS_CONSENSUS_CLIENT_ID,
			};
			assert!(commitments.contains_key(&state_id), "Should have state commitment");
		},
		Err(e) => {
			panic!("Verification failed: {:?}", e);
		},
	}
}

#[tokio::test]
#[ignore]
async fn test_ismp_pharos_epoch_boundary_consensus_verification() {
	let prover = PharosProver::<Testnet>::new(ATLANTIC_RPC);

	let latest_block_num = prover.get_latest_block().await.expect("Failed to get block number");
	println!("Latest block: {}", latest_block_num);

	// Find the most recent epoch boundary block.
	let epoch_length = Testnet::EPOCH_LENGTH_BLOCKS;
	let current_epoch = Testnet::compute_epoch(latest_block_num);
	// last epoch boundary block is at (current_epoch * epoch_length) - 1
	let target_block = (current_epoch * epoch_length) - 1;

	assert!(
		Testnet::is_epoch_boundary(target_block),
		"Target block {} should be an epoch boundary",
		target_block
	);
	println!("Target epoch boundary block: {}", target_block);
	println!("Current epoch: {}, target epoch: {}", current_epoch, Testnet::compute_epoch(target_block));

	let validator_info = prover
		.rpc
		.get_validator_info(Some(target_block))
		.await
		.expect("Failed to get validator info");
	println!("Validators: {}", validator_info.validator_set.len());

	let target_epoch = Testnet::compute_epoch(target_block);
	let validator_set = prover
		.build_validator_set(&validator_info.validator_set, target_epoch)
		.expect("Failed to build validator set");
	println!("Total stake: {}", validator_set.total_stake);

	// trusted consensus state at the block before the epoch boundary
	let initial_block = target_block - 1;
	let initial_consensus_state = ConsensusState {
		current_validators: validator_set,
		finalized_height: initial_block,
		finalized_hash: H256::zero(),
		current_epoch: target_epoch,
		chain_id: PHAROS_ATLANTIC_CHAIN_ID,
	};

	// should include a validator_set_proof because it's an epoch boundary.
	let update = prover
		.fetch_block_update(target_block)
		.await
		.expect("Failed to fetch block update for epoch boundary");
	println!("Block update is for block: {}", update.block_number());
	println!("Participant keys Length: {}", update.block_proof.participant_count());
	assert!(
		update.validator_set_proof.is_some(),
		"Epoch boundary block should include a validator set proof"
	);

	let pharos_client = PharosClient::<Ismp, Test, Testnet>::default();

	let host = Ismp::default();
	let result = pharos_client.verify_consensus(
		&host,
		PHAROS_CONSENSUS_CLIENT_ID,
		initial_consensus_state.encode(),
		update.encode(),
	);

	match result {
		Ok((new_state_bytes, commitments)) => {
			let new_state = ConsensusState::decode(&mut &new_state_bytes[..])
				.expect("Failed to decode new state");

			println!("\nEpoch Boundary Verification Successful");
			println!("Finalized height: {}", new_state.finalized_height);
			println!("Previous epoch: {}", initial_consensus_state.current_epoch);
			println!("New epoch: {}", new_state.current_epoch);
			println!("New validator count: {}", new_state.current_validators.len());

			assert_eq!(
				new_state.current_epoch,
				initial_consensus_state.current_epoch + 1,
				"Epoch should increment by 1 at epoch boundary"
			);

			assert_eq!(
				new_state.finalized_height, target_block,
				"Finalized height should match the epoch boundary block"
			);

			assert!(
				!new_state.current_validators.is_empty(),
				"New validator set should not be empty"
			);

			let state_id = StateMachineId {
				state_id: StateMachine::Evm(PHAROS_ATLANTIC_CHAIN_ID),
				consensus_state_id: PHAROS_CONSENSUS_CLIENT_ID,
			};
			assert!(commitments.contains_key(&state_id), "Should have state commitment");
		},
		Err(e) => {
			panic!("Epoch boundary verification failed: {:?}", e);
		},
	}
}
