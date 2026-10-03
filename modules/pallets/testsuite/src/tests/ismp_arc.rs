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
use arc_primitives::{
	CommitCertificate, ValidatorSet, ValidatorSetProof, VerifierStateUpdate, ARC_TESTNET_CHAIN_ID,
};
use arc_prover::{header_hash, ArcProver};
use codec::Encode;
use geth_primitives::CodecHeader;
use ismp::{
	consensus::{ConsensusClient, StateMachineId},
	host::StateMachine,
};
use ismp_arc::{ArcClient, ConsensusState, ARC_CONSENSUS_CLIENT_ID};
use primitive_types::{H160, H256, U256};

/// Public Arc testnet endpoint, serving both `eth_getProof` (at `"latest"`
/// only) and `arc_getCertificate`.
const DEFAULT_ARC_TESTNET_RPC: &str = "https://arc-testnet.drpc.org";

/// Read an endpoint override, treating an unset or empty variable as absent.
fn endpoint(var: &str) -> String {
	std::env::var(var)
		.ok()
		.filter(|url| !url.trim().is_empty())
		.unwrap_or_else(|| DEFAULT_ARC_TESTNET_RPC.to_string())
}

fn prover() -> ArcProver {
	ArcProver::with_certificate_endpoint(endpoint("ARC_RPC_URL"), endpoint("ARC_CERT_RPC_URL"))
		.expect("Failed to create the Arc prover")
}

fn client() -> ArcClient<Ismp, Test> {
	ArcClient::<Ismp, Test>::default()
}

/// An all-zero header, for the checks that reject an update before the header
/// is ever hashed.
fn empty_header(number: u64) -> CodecHeader {
	CodecHeader {
		parent_hash: H256::zero(),
		uncle_hash: H256::zero(),
		coinbase: H160::zero(),
		state_root: H256::zero(),
		transactions_root: H256::zero(),
		receipts_root: H256::zero(),
		logs_bloom: Default::default(),
		difficulty: U256::zero(),
		number: U256::from(number),
		gas_limit: 0,
		gas_used: 0,
		timestamp: 0,
		extra_data: Default::default(),
		mix_hash: H256::zero(),
		nonce: Default::default(),
		base_fee_per_gas: None,
		withdrawals_hash: None,
		blob_gas_used: None,
		excess_blob_gas_used: None,
		parent_beacon_root: None,
		requests_hash: None,
	}
}

fn update(height: u64, block_hash: H256) -> VerifierStateUpdate {
	VerifierStateUpdate {
		header: empty_header(height),
		certificate: CommitCertificate {
			height,
			round: 0,
			block_hash,
			commit_signatures: Default::default(),
		},
		validator_set_proof: ValidatorSetProof::default(),
	}
}

fn consensus_state(validators: ValidatorSet, finalized_height: u64) -> ConsensusState {
	ConsensusState {
		current_validators: validators,
		finalized_height,
		finalized_hash: H256::zero(),
		chain_id: ARC_TESTNET_CHAIN_ID,
	}
}

/// Drives a live Arc testnet update through the pallet's consensus client,
/// which is the code path `pallet_ismp` actually executes on a consensus
/// message.
#[tokio::test]
#[ignore = "requires network access to the Arc testnet — set ARC_RPC_URL / ARC_CERT_RPC_URL to override the default RPC and run with --ignored"]
async fn test_ismp_arc_consensus_verification() {
	let prover = prover();

	let trusted = prover
		.fetch_latest_verifier_state()
		.await
		.expect("Failed to bootstrap the trusted state");
	println!(
		"Bootstrapped at height {} with {} validators (total power {})",
		trusted.finalized_height,
		trusted.current_validators.len(),
		trusted.current_validators.total_voting_power,
	);
	assert!(!trusted.current_validators.is_empty());

	let update = loop {
		let update = prover.fetch_latest_update().await.expect("Failed to fetch an update");
		if update.certificate.height > trusted.finalized_height {
			break update;
		}
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	};
	let height = update.certificate.height;
	println!(
		"Verifying certificate for height {height} ({} signatures)",
		update.certificate.commit_signatures.len()
	);

	let trusted_state =
		consensus_state(trusted.current_validators.clone(), trusted.finalized_height);

	let host = Ismp::default();
	let (new_state, commitments) = client()
		.verify_consensus(&host, ARC_CONSENSUS_CLIENT_ID, trusted_state.encode(), update.encode())
		.expect("Consensus verification failed");

	let new_state = <ConsensusState as codec::Decode>::decode(&mut &new_state[..])
		.expect("Failed to decode the new consensus state");
	assert_eq!(new_state.finalized_height, height);
	assert_eq!(new_state.finalized_hash, update.certificate.block_hash);
	assert_eq!(new_state.chain_id, ARC_TESTNET_CHAIN_ID);
	assert!(!new_state.current_validators.is_empty());

	let state_machine_id = StateMachineId {
		state_id: StateMachine::Evm(ARC_TESTNET_CHAIN_ID),
		consensus_state_id: ARC_CONSENSUS_CLIENT_ID,
	};
	let heights = commitments.get(&state_machine_id).expect("Missing state commitment");
	assert_eq!(heights.len(), 1);
	assert_eq!(heights[0].height, height);
	assert_eq!(heights[0].commitment.state_root, update.header.state_root);
	assert_eq!(heights[0].commitment.timestamp, update.header.timestamp);
	assert_eq!(heights[0].commitment.overlay_root, None);

	// The same update must not replay against the state it just produced.
	let stale = client().verify_consensus(
		&host,
		ARC_CONSENSUS_CLIENT_ID,
		new_state.encode(),
		update.encode(),
	);
	assert!(stale.is_err(), "a replayed update must be rejected as stale");
}

/// The certificate binds to a header, so an update carrying a header that
/// doesn't hash to the certified block hash must be rejected before any
/// signature is checked.
#[tokio::test]
#[ignore = "requires network access to the Arc testnet — set ARC_RPC_URL / ARC_CERT_RPC_URL to override the default RPC and run with --ignored"]
async fn test_ismp_arc_rejects_mismatched_header() {
	let prover = prover();

	let trusted = prover
		.fetch_latest_verifier_state()
		.await
		.expect("Failed to bootstrap the trusted state");

	let mut update = prover.fetch_latest_update().await.expect("Failed to fetch an update");
	// Swap in the parent header while leaving the certificate untouched.
	let parent = prover
		.rpc
		.get_block_by_number(update.certificate.height - 1)
		.await
		.expect("Failed to fetch the parent header");
	assert_ne!(header_hash(&parent), update.certificate.block_hash);
	update.header = parent;

	let host = Ismp::default();
	let result = client().verify_consensus(
		&host,
		ARC_CONSENSUS_CLIENT_ID,
		consensus_state(trusted.current_validators, trusted.finalized_height).encode(),
		update.encode(),
	);
	assert!(result.is_err(), "an update whose header doesn't match the certificate must fail");
}

/// Two certificates at different heights aren't equivocation.
#[test]
fn test_arc_fraud_proof_rejects_different_heights() {
	let host = Ismp::default();
	let result = client().verify_fraud_proof(
		&host,
		consensus_state(ValidatorSet::default(), 0).encode(),
		update(100, H256::repeat_byte(1)).encode(),
		update(101, H256::repeat_byte(2)).encode(),
	);
	assert!(result.is_err(), "certificates at different heights are not a fraud proof");
}

/// Neither is the same certificate submitted twice.
#[test]
fn test_arc_fraud_proof_rejects_identical_block_hashes() {
	let host = Ismp::default();
	let block_hash = H256::repeat_byte(1);
	let result = client().verify_fraud_proof(
		&host,
		consensus_state(ValidatorSet::default(), 0).encode(),
		update(100, block_hash).encode(),
		update(100, block_hash).encode(),
	);
	assert!(result.is_err(), "identical block hashes are not a fraud proof");
}

/// Only the Arc chain ids the client knows about get a state machine.
#[test]
fn test_arc_state_machine_dispatch() {
	assert!(client().state_machine(StateMachine::Evm(ARC_TESTNET_CHAIN_ID)).is_ok());
	assert!(client().state_machine(StateMachine::Evm(1)).is_err());
	assert!(client().state_machine(StateMachine::Polkadot(2000)).is_err());
}

/// The client must report the id the runtime registers it under.
#[test]
fn test_arc_consensus_client_id() {
	assert_eq!(client().consensus_client_id(), ARC_CONSENSUS_CLIENT_ID);
	assert_eq!(&ARC_CONSENSUS_CLIENT_ID, b"ARCC");
}
