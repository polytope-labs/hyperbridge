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
use pharos_primitives::{Testnet, VerifierState, PHAROS_ATLANTIC_CHAIN_ID};
use pharos_prover::PharosProver;
use pharos_verifier::{error::Error as VerifierError, verify_pharos_block};
use primitive_types::H256;

#[tokio::test]
#[ignore]
async fn test_ismp_pharos_non_epoch_boundary_consensus_verification() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let prover = PharosProver::<Testnet>::new(&rpc_url).await.expect("Failed to create prover");

	let latest_block_num = prover.get_latest_block().await.expect("Failed to get block number");
	println!("Latest block: {}", latest_block_num);

	let mut target_block = latest_block_num.saturating_sub(5);

	// Ensure we're not at an epoch boundary by checking the epoch at consecutive blocks
	while prover.is_epoch_boundary(target_block).await.expect("epoch boundary check") {
		target_block = target_block.saturating_sub(1);
	}
	println!("Target block: {}", target_block);

	let validator_info =
		prover.rpc.get_validator_info(None).await.expect("Failed to get validator info");
	println!("Validators: {}", validator_info.validator_set.len());

	let current_epoch = prover.fetch_current_epoch(target_block).await.expect("fetch epoch");
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

			let heights = &commitments[&state_id];
			assert_eq!(heights.len(), 1, "Should have exactly one state commitment");
			assert_eq!(
				heights[0].height, target_block,
				"Commitment height should match the target block"
			);

			assert_eq!(
				new_state.finalized_height, target_block,
				"Finalized height should match the target block"
			);
		},
		Err(e) => {
			panic!("Verification failed: {:?}", e);
		},
	}
}

#[tokio::test]
#[ignore]
async fn test_ismp_pharos_epoch_boundary_consensus_verification() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let prover = PharosProver::<Testnet>::new(&rpc_url).await.expect("Failed to create prover");

	let latest_block_num = prover.get_latest_block().await.expect("Failed to get block number");
	println!("Latest block: {}", latest_block_num);

	// Find the most recent epoch boundary using find_epoch_boundary.
	let current_epoch = prover.fetch_current_epoch(latest_block_num).await.expect("fetch epoch");
	println!("Current epoch at latest block: {}", current_epoch);

	// find_epoch_boundary binary-searches for the first block where
	// currentEpoch > previous_epoch, i.e. the epoch boundary block.
	let previous_epoch = current_epoch - 1;
	let target_block = prover
		.find_epoch_boundary(
			latest_block_num.saturating_sub(5000),
			latest_block_num,
			previous_epoch,
		)
		.await
		.expect("find epoch boundary");
	let target_epoch = prover.fetch_current_epoch(target_block).await.expect("fetch epoch");

	assert!(
		prover.is_epoch_boundary(target_block).await.expect("epoch boundary check"),
		"Target block {} should be an epoch boundary",
		target_block
	);
	println!("Target epoch boundary block: {}", target_block);
	println!("Epoch at boundary: {}", target_epoch);

	let validator_info = prover
		.rpc
		.get_validator_info(Some(target_block))
		.await
		.expect("Failed to get validator info");
	println!("Validators: {}", validator_info.validator_set.len());
	// The trusted state uses the PREVIOUS epoch (before the transition)
	let validator_set = prover
		.build_validator_set(&validator_info.validator_set, previous_epoch)
		.expect("Failed to build validator set");
	println!("Total stake: {}", validator_set.total_stake);

	// trusted consensus state at the block before the epoch boundary, with previous epoch
	let initial_block = target_block - 1;
	let initial_consensus_state = ConsensusState {
		current_validators: validator_set,
		finalized_height: initial_block,
		finalized_hash: H256::zero(),
		current_epoch: previous_epoch,
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

			let heights = &commitments[&state_id];
			assert_eq!(heights.len(), 1, "Should have exactly one state commitment");
			assert_eq!(
				heights[0].height, target_block,
				"Commitment height should match the epoch boundary block"
			);
		},
		Err(e) => {
			panic!("Epoch boundary verification failed: {:?}", e);
		},
	}
}

async fn fetch_latest_boundary(
	prover: &PharosProver<Testnet>,
) -> (u64, u64, pharos_primitives::VerifierStateUpdate) {
	let latest = prover.get_latest_block().await.expect("latest block");
	let current_epoch = prover.fetch_current_epoch(latest).await.expect("fetch epoch");
	let previous_epoch = current_epoch - 1;
	let boundary = prover
		.find_epoch_boundary(latest.saturating_sub(5000), latest, previous_epoch)
		.await
		.expect("find epoch boundary");
	let update = prover.fetch_block_update(boundary).await.expect("fetch boundary update");
	(boundary, previous_epoch, update)
}

fn trusted_state_from_validators(
	prover: &PharosProver<Testnet>,
	validators: &[pharos_prover::rpc::RpcValidatorInfo],
	epoch: u64,
	finalized_height: u64,
) -> VerifierState {
	let validator_set = prover.build_validator_set(validators, epoch).expect("build validator set");
	VerifierState {
		current_validator_set: validator_set,
		finalized_block_number: finalized_height,
		finalized_hash: H256::zero(),
		current_epoch: epoch,
	}
}

#[tokio::test]
#[ignore]
async fn test_in_epoch_update_with_unexpected_validator_set_proof_rejected() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let prover = PharosProver::<Testnet>::new(&rpc_url).await.expect("create prover");

	let (boundary, previous_epoch, boundary_update) = fetch_latest_boundary(&prover).await;

	let mut target = boundary + 1;
	while prover.is_epoch_boundary(target).await.expect("epoch boundary check") {
		target += 1;
	}

	let validators_at_boundary =
		prover.rpc.get_validator_info(Some(boundary)).await.expect("validator info");
	let trusted = trusted_state_from_validators(
		&prover,
		&validators_at_boundary.validator_set,
		previous_epoch + 1,
		target - 1,
	);

	let mut update = prover.fetch_block_update(target).await.expect("fetch update");
	update.validator_set_proof = boundary_update.validator_set_proof.clone();

	let err = verify_pharos_block::<Testnet, Ismp>(trusted, update).expect_err("should reject");
	assert!(matches!(err, VerifierError::UnexpectedValidatorSetProof { .. }), "got: {err:?}");
}

#[tokio::test]
#[ignore]
async fn test_boundary_update_without_validator_set_proof_rejected() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let prover = PharosProver::<Testnet>::new(&rpc_url).await.expect("create prover");

	let (boundary, previous_epoch, mut update) = fetch_latest_boundary(&prover).await;
	let validators_at_boundary =
		prover.rpc.get_validator_info(Some(boundary)).await.expect("validator info");
	let trusted = trusted_state_from_validators(
		&prover,
		&validators_at_boundary.validator_set,
		previous_epoch,
		boundary - 1,
	);

	update.validator_set_proof = None;

	let err = verify_pharos_block::<Testnet, Ismp>(trusted, update).expect_err("should reject");
	assert!(matches!(err, VerifierError::MissingValidatorSetProof { .. }), "got: {err:?}");
}

#[tokio::test]
#[ignore]
async fn test_epoch_skip_rejected() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let prover = PharosProver::<Testnet>::new(&rpc_url).await.expect("create prover");

	let latest = prover.get_latest_block().await.expect("latest block");
	let mut target = latest.saturating_sub(5);
	while prover.is_epoch_boundary(target).await.expect("epoch boundary check") {
		target = target.saturating_sub(1);
	}
	let observed_epoch = prover.fetch_current_epoch(target).await.expect("fetch epoch");

	let validators = prover.rpc.get_validator_info(Some(target)).await.expect("validator info");
	let trusted = trusted_state_from_validators(
		&prover,
		&validators.validator_set,
		observed_epoch.saturating_sub(2),
		target - 1,
	);

	let update = prover.fetch_block_update(target).await.expect("fetch update");

	let err = verify_pharos_block::<Testnet, Ismp>(trusted, update).expect_err("should reject");
	assert!(matches!(err, VerifierError::EpochSkipped { .. }), "got: {err:?}");
}

#[tokio::test]
#[ignore]
async fn test_epoch_regression_rejected() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let prover = PharosProver::<Testnet>::new(&rpc_url).await.expect("create prover");

	let latest = prover.get_latest_block().await.expect("latest block");
	let mut target = latest.saturating_sub(5);
	while prover.is_epoch_boundary(target).await.expect("epoch boundary check") {
		target = target.saturating_sub(1);
	}
	let observed_epoch = prover.fetch_current_epoch(target).await.expect("fetch epoch");

	let validators = prover.rpc.get_validator_info(Some(target)).await.expect("validator info");
	let trusted = trusted_state_from_validators(
		&prover,
		&validators.validator_set,
		observed_epoch + 10,
		target - 1,
	);

	let update = prover.fetch_block_update(target).await.expect("fetch update");

	let err = verify_pharos_block::<Testnet, Ismp>(trusted, update).expect_err("should reject");
	assert!(matches!(err, VerifierError::EpochRegressed { .. }), "got: {err:?}");
}

#[tokio::test]
#[ignore]
async fn test_tampered_epoch_proof_rejected() {
	let rpc_url =
		std::env::var("PHAROS_ATLANTIC_RPC").expect("PHAROS_ATLANTIC_RPC env variable must be set");
	let prover = PharosProver::<Testnet>::new(&rpc_url).await.expect("create prover");

	let latest = prover.get_latest_block().await.expect("latest block");
	let mut target = latest.saturating_sub(5);
	while prover.is_epoch_boundary(target).await.expect("epoch boundary check") {
		target = target.saturating_sub(1);
	}
	let observed_epoch = prover.fetch_current_epoch(target).await.expect("fetch epoch");

	let validators = prover.rpc.get_validator_info(Some(target)).await.expect("validator info");
	let trusted = trusted_state_from_validators(
		&prover,
		&validators.validator_set,
		observed_epoch,
		target - 1,
	);

	let mut update = prover.fetch_block_update(target).await.expect("fetch update");
	let mut tampered = update.current_epoch_proof.value.clone();
	if tampered.is_empty() {
		tampered.push(0xff);
	} else {
		let last = tampered.len() - 1;
		tampered[last] ^= 0xff;
	}
	update.current_epoch_proof.value = tampered;

	let err = verify_pharos_block::<Testnet, Ismp>(trusted, update).expect_err("should reject");
	assert!(matches!(err, VerifierError::StorageProofVerificationFailed(_)), "got: {err:?}");
}
