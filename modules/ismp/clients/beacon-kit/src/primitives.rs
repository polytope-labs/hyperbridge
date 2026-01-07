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

use alloc::{vec, vec::Vec};
use codec::{Decode, Encode};
use ismp::consensus::ConsensusClientId;
use ssz_rs::prelude::*;
use sync_committee_primitives::consensus_types::{
	BeaconBlock, BeaconBlockBody, ExecutionPayload,
};
use sync_committee_primitives::constants::BlsSignature;
use tendermint_primitives::{CodecConsensusProof, CodecTrustedState};

/// BeaconKit consensus client identifier
pub const BEACON_KIT_CONSENSUS_CLIENT_ID: ConsensusClientId = *b"BKIT";

/// Berachain mainnet EVM chain ID
pub const BERACHAIN_MAINNET_CHAIN_ID: u32 = 80094;

/// Berachain Bepolia testnet EVM chain ID
pub const BERACHAIN_BEPOLIA_CHAIN_ID: u32 = 80069;

// BeaconKit block constants
pub const MAX_PROPOSER_SLASHINGS: usize = 16;
pub const MAX_VALIDATORS_PER_COMMITTEE: usize = 2048;
pub const MAX_ATTESTER_SLASHINGS: usize = 2;
pub const MAX_ATTESTATIONS: usize = 128;
pub const MAX_DEPOSITS: usize = 16;
pub const MAX_VOLUNTARY_EXITS: usize = 16;
pub const SYNC_COMMITTEE_SIZE: usize = 512;
pub const BYTES_PER_LOGS_BLOOM: usize = 256;
pub const MAX_EXTRA_DATA_BYTES: usize = 32;
pub const MAX_BYTES_PER_TRANSACTION: usize = 1073741824;
pub const MAX_TRANSACTIONS_PER_PAYLOAD: usize = 1048576;
pub const MAX_WITHDRAWALS_PER_PAYLOAD: usize = 16;
pub const MAX_BLS_TO_EXECUTION_CHANGES: usize = 16;
pub const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize = 4096;
pub const MAX_COMMITTEES_PER_SLOT: usize = 64;
pub const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize = 8192;
pub const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize = 16;
pub const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize = 1;

/// BeaconKit beacon block type alias
pub type BeaconKitBlock = BeaconBlock<
	MAX_PROPOSER_SLASHINGS,
	MAX_VALIDATORS_PER_COMMITTEE,
	MAX_ATTESTER_SLASHINGS,
	MAX_ATTESTATIONS,
	MAX_DEPOSITS,
	MAX_VOLUNTARY_EXITS,
	SYNC_COMMITTEE_SIZE,
	BYTES_PER_LOGS_BLOOM,
	MAX_EXTRA_DATA_BYTES,
	MAX_BYTES_PER_TRANSACTION,
	MAX_TRANSACTIONS_PER_PAYLOAD,
	MAX_WITHDRAWALS_PER_PAYLOAD,
	MAX_BLS_TO_EXECUTION_CHANGES,
	MAX_BLOB_COMMITMENTS_PER_BLOCK,
	MAX_COMMITTEES_PER_SLOT,
	MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
	MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
	MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
>;

/// BeaconKit beacon block body type alias
pub type BeaconKitBlockBody = BeaconBlockBody<
	MAX_PROPOSER_SLASHINGS,
	MAX_VALIDATORS_PER_COMMITTEE,
	MAX_ATTESTER_SLASHINGS,
	MAX_ATTESTATIONS,
	MAX_DEPOSITS,
	MAX_VOLUNTARY_EXITS,
	SYNC_COMMITTEE_SIZE,
	BYTES_PER_LOGS_BLOOM,
	MAX_EXTRA_DATA_BYTES,
	MAX_BYTES_PER_TRANSACTION,
	MAX_TRANSACTIONS_PER_PAYLOAD,
	MAX_WITHDRAWALS_PER_PAYLOAD,
	MAX_BLS_TO_EXECUTION_CHANGES,
	MAX_BLOB_COMMITMENTS_PER_BLOCK,
	MAX_COMMITTEES_PER_SLOT,
	MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
	MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
	MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
>;

/// BeaconKit execution payload type alias
pub type BeaconKitExecutionPayload = ExecutionPayload<
	BYTES_PER_LOGS_BLOOM,
	MAX_EXTRA_DATA_BYTES,
	MAX_BYTES_PER_TRANSACTION,
	MAX_TRANSACTIONS_PER_PAYLOAD,
	MAX_WITHDRAWALS_PER_PAYLOAD,
>;

/// The consensus update/proof for BeaconKit
#[derive(Debug, Clone, Encode, Decode)]
pub struct BeaconKitUpdate {
	/// Tendermint consensus proof (signed header + validators) with BLS aggregation
	pub tendermint_update: CodecConsensusProof,
	/// All transactions in the CometBFT block.
	/// The first transaction (txs[0]) is the SSZ-encoded SignedBeaconBlock.
	pub txs: Vec<Vec<u8>>,
}

/// The trusted consensus state for BeaconKit
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ConsensusState {
	/// Tendermint trusted state
	pub tendermint_state: CodecTrustedState,
	/// Chain ID for the BeaconKit network (EVM chain ID)
	pub chain_id: u32,
}
