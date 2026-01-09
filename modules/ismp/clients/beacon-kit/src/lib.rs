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

//! BeaconKit consensus client implementation for ISMP.
//!
//! This module provides a consensus client for BeaconKit that verifies Tendermint light client
//! updates with BLS aggregated signatures, along with transaction proofs to verify the
//! embedded signed beacon block.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod primitives;

pub use primitives::*;

use alloc::{boxed::Box, collections::BTreeMap, format, string::ToString, vec, vec::Vec};
use codec::{Decode, Encode};
use cometbft::merkle::simple_hash_from_byte_vectors;
use evm_state_machine::EvmStateMachine;
use ismp::{
	consensus::{
		ConsensusClient, ConsensusClientId, ConsensusStateId, StateCommitment, StateMachineClient,
		StateMachineId, VerifiedCommitments,
	},
	error::Error,
	host::{IsmpHost, StateMachine},
	messaging::StateCommitmentHeight,
};
use pallet_ismp_host_executive::Config as HostExecutiveConfig;
use primitive_types::H256;
use sha2::{Digest, Sha256};
use ssz_rs::prelude::*;
use sync_committee_primitives::consensus_types::BeaconBlock;
use tendermint_primitives::{CodecTrustedState, TrustedState};
use tendermint_verifier::verify_header_update;

/// Verify that the transactions hash to the expected data_hash.
///
/// BeaconKit blocks contain at most 2 transactions. This function:
/// 1. Computes the merkle root of all transactions using simple_hash_from_byte_vectors
/// 2. Compares to the expected data_hash from the block header
pub fn verify_tx_proof(txs: &[Vec<u8>], data_hash: H256) -> Result<(), Error> {
	if txs.is_empty() {
		return Err(Error::Custom("No transactions provided".to_string()));
	}

	let computed_hash = compute_data_hash(txs);

	if computed_hash != data_hash.0 {
		return Err(Error::Custom(format!(
			"Data hash mismatch: computed {:?}, expected {:?}",
			H256::from(computed_hash),
			data_hash
		)));
	}

	Ok(())
}

/// BeaconKit computes data_hash:
/// 1. First hash each transaction: tx_hash = sha256(tx) (plain SHA256, no prefix)
/// 2. Then compute merkle root from those hashes using standard merkle algorithm
///
/// This differs from standard CometBFT which uses raw tx bytes directly.
pub fn compute_data_hash(txs: &[impl AsRef<[u8]>]) -> [u8; 32] {
	let tx_hashes: Vec<[u8; 32]> = txs
		.iter()
		.map(|tx| {
			let mut hasher = Sha256::new();
			hasher.update(tx.as_ref());
			hasher.finalize().into()
		})
		.collect();

	simple_hash_from_byte_vectors::<Sha256>(&tx_hashes)
}

/// BeaconKit consensus client implementation
pub struct BeaconKitClient<H: IsmpHost, T: HostExecutiveConfig>(
	core::marker::PhantomData<(H, T)>,
);

impl<H: IsmpHost, T: HostExecutiveConfig> Default for BeaconKitClient<H, T> {
	fn default() -> Self {
		Self(core::marker::PhantomData)
	}
}

impl<H: IsmpHost + Send + Sync + Default + 'static, T: HostExecutiveConfig> ConsensusClient
	for BeaconKitClient<H, T>
{
	fn verify_consensus(
		&self,
		host: &dyn IsmpHost,
		consensus_state_id: ConsensusStateId,
		trusted_consensus_state: Vec<u8>,
		proof: Vec<u8>,
	) -> Result<(Vec<u8>, VerifiedCommitments), Error> {
		let beacon_kit_update: BeaconKitUpdate = Decode::decode(&mut &proof[..])
			.map_err(|e| Error::Custom(format!("Failed to decode BeaconKitUpdate: {}", e)))?;

		let consensus_state = ConsensusState::decode(&mut &trusted_consensus_state[..])
			.map_err(|e| Error::Custom(format!("Failed to decode ConsensusState: {}", e)))?;

		let consensus_proof = beacon_kit_update
			.tendermint_update
			.to_consensus_proof()
			.map_err(|e| Error::Custom(format!("Failed to convert consensus proof: {}", e)))?;

		let trusted_state: TrustedState = consensus_state.tendermint_state.clone().into();
		let time = host.timestamp().as_secs();

		let updated_state = verify_header_update(trusted_state, consensus_proof.clone(), time)
			.map_err(|e| Error::Custom(format!("Tendermint header verification failed: {}", e)))?;

		let data_hash = consensus_proof
			.signed_header
			.header
			.data_hash
			.ok_or_else(|| Error::Custom("No data_hash in verified header".to_string()))?;

		let data_hash_h256 = H256::from_slice(data_hash.as_bytes());

		// Verify the transaction merkle proof
		verify_tx_proof(&beacon_kit_update.txs, data_hash_h256)?;

		// The first transaction is the beacon block
		// The first 100 bytes of the beacon block is a prefix
		let beacon_block = beacon_kit_update
			.txs
			.first()
			.ok_or_else(|| Error::Custom("No transactions in update".to_string()))?;
		let ssz_beacon_block = &beacon_block[100..];

		let signed_beacon_block:BeaconBlock<MAX_PROPOSER_SLASHINGS, MAX_VALIDATORS_PER_COMMITTEE, MAX_ATTESTER_SLASHINGS, MAX_ATTESTATIONS, MAX_DEPOSITS, MAX_VOLUNTARY_EXITS, SYNC_COMMITTEE_SIZE, BYTES_PER_LOGS_BLOOM, MAX_EXTRA_DATA_BYTES, MAX_BYTES_PER_TRANSACTION, MAX_TRANSACTIONS_PER_PAYLOAD, MAX_WITHDRAWALS_PER_PAYLOAD, MAX_BLS_TO_EXECUTION_CHANGES, MAX_BLOB_COMMITMENTS_PER_BLOCK, MAX_COMMITTEES_PER_SLOT, MAX_DEPOSIT_REQUESTS_PER_PAYLOAD, MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD> =
			deserialize(&ssz_beacon_block)
				.map_err(|e| Error::Custom(format!("Failed to SSZ decode SignedBeaconBlock: {:?}", e)))?;

		let execution_payload = &signed_beacon_block.body.execution_payload;

		let state_root_bytes: [u8; 32] = execution_payload
			.state_root
			.as_ref()
			.try_into()
			.map_err(|_| Error::Custom("Invalid state root length".to_string()))?;

		let state_commitment = StateCommitmentHeight {
			commitment: StateCommitment {
				timestamp: execution_payload.timestamp,
				overlay_root: None,
				state_root: H256::from(state_root_bytes),
			},
			height: execution_payload.block_number,
		};

		let mut state_machine_map: BTreeMap<StateMachineId, Vec<StateCommitmentHeight>> =
			BTreeMap::new();

		state_machine_map.insert(
			StateMachineId {
				state_id: StateMachine::Evm(consensus_state.chain_id),
				consensus_state_id,
			},
			vec![state_commitment],
		);

		let updated_consensus_state = ConsensusState {
			tendermint_state: CodecTrustedState::from(&updated_state.trusted_state),
			chain_id: consensus_state.chain_id,
		};

		Ok((updated_consensus_state.encode(), state_machine_map))
	}

	fn verify_fraud_proof(
		&self,
		host: &dyn IsmpHost,
		trusted_consensus_state: Vec<u8>,
		proof_1: Vec<u8>,
		proof_2: Vec<u8>,
	) -> Result<(), Error> {
		let update_1: BeaconKitUpdate =
			Decode::decode(&mut &proof_1[..]).map_err(|e| Error::Custom(e.to_string()))?;
		let update_2: BeaconKitUpdate =
			Decode::decode(&mut &proof_2[..]).map_err(|e| Error::Custom(e.to_string()))?;

		let consensus_state: ConsensusState = Decode::decode(&mut &trusted_consensus_state[..])
			.map_err(|e| Error::Custom(e.to_string()))?;


		let height_1 = update_1.tendermint_update.signed_header.header.height;
		let height_2 = update_2.tendermint_update.signed_header.header.height;
		if height_1 != height_2 {
			return Err(Error::Custom("Fraud proofs must be for the same block height".to_string()));
		}


		if proof_1 == proof_2 {
			return Err(Error::Custom("Fraud proofs are identical".to_string()));
		}

		let trusted_state: TrustedState = consensus_state.tendermint_state.into();
		let time = host.timestamp().as_secs();


		let consensus_proof_1 = update_1
			.tendermint_update
			.to_consensus_proof()
			.map_err(|e| Error::Custom(e.to_string()))?;

		let consensus_proof_2 = update_2
			.tendermint_update
			.to_consensus_proof()
			.map_err(|e| Error::Custom(e.to_string()))?;

		verify_header_update(trusted_state.clone(), consensus_proof_1, time)
			.map_err(|e| Error::Custom(e.to_string()))?;
		verify_header_update(trusted_state, consensus_proof_2, time)
			.map_err(|e| Error::Custom(e.to_string()))?;

		Ok(())
	}

	fn consensus_client_id(&self) -> ConsensusClientId {
		BEACON_KIT_CONSENSUS_CLIENT_ID
	}

	fn state_machine(&self, id: StateMachine) -> Result<Box<dyn StateMachineClient>, Error> {
		match id {
			StateMachine::Evm(chain_id)
				if chain_id == BERACHAIN_MAINNET_CHAIN_ID
					|| chain_id == BERACHAIN_BEPOLIA_CHAIN_ID =>
			{
				Ok(Box::new(EvmStateMachine::<H, T>::default()))
			}
			_ => Err(Error::Custom("Unsupported state machine or chain ID".to_string())),
		}
	}
}
