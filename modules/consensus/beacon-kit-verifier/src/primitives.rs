// Copyright (C) 2022 Polytope Labs.
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
#![cfg_attr(not(feature = "std"), no_std)]

use alloc::vec::Vec;
use codec::{Decode, Encode};
use hex_literal::hex;
use primitive_types::H256;
use ssz_rs::prelude::*;
use sync_committee_primitives::{
	consensus_types::{BeaconBlockHeader, ExecutionPayloadHeader},
	constants::{BlsPublicKey, BlsSignature},
};

const BYTES_PER_LOGS_BLOOM: usize = 256;
const MAX_EXTRA_DATA_BYTES: usize = 32;

pub trait Config: Clone + Send + Sync {
	const EXECUTION_PAYLOAD_INDEX: usize;
	const EXECTION_PAYLOAD_INDEX_LOG2: usize;
	const VALIDATOR_REGSITRY_INDEX: usize;
	const VALIDATOR_REGISTRY_INDEX_LOG2: usize;
	const GENESIS_VALIDATORS_ROOT: [u8; 32];
	const GENESIS_FORK_VERSION: [u8; 4];
	const BEACON_KIT_FORK_VERSION: [u8; 4];
}

/// Config for the Berachain Bepolia Testnet
#[derive(Clone, Default)]
pub struct BepoliaConfig;

impl Config for BepoliaConfig {
	const EXECUTION_PAYLOAD_INDEX: usize = 25;
	const EXECTION_PAYLOAD_INDEX_LOG2: usize = 5;
	const VALIDATOR_REGSITRY_INDEX: usize = 11;
	const VALIDATOR_REGISTRY_INDEX_LOG2: usize = 5;
	const GENESIS_VALIDATORS_ROOT: [u8; 32] =
		hex!("3cbcf75b02fe4750c592f1c1ff8b5500a74406f80f038e9ff250e2e294c5615e");
	const GENESIS_FORK_VERSION: [u8; 4] = hex!("04000000");
	const BEACON_KIT_FORK_VERSION: [u8; 4] = hex!("05010000");
}

/// Config for the Berachain mainnet
#[derive(Clone, Default)]
pub struct BerachainConfig;

impl Config for BerachainConfig {
	const EXECUTION_PAYLOAD_INDEX: usize = 25;
	const EXECTION_PAYLOAD_INDEX_LOG2: usize = 5;
	const VALIDATOR_REGSITRY_INDEX: usize = 11;
	const VALIDATOR_REGISTRY_INDEX_LOG2: usize = 5;
	const GENESIS_VALIDATORS_ROOT: [u8; 32] =
		hex!("df609e3b062842c6425ff716aec2d2092c46455d9b2e1a2c9e32c6ba63ff0bda");
	const GENESIS_FORK_VERSION: [u8; 4] = hex!("04000000");
	const BEACON_KIT_FORK_VERSION: [u8; 4] = hex!("05010000");
}

/// Represents a light client update for Beacon Kit consensus
#[derive(Debug, Clone, Encode, Decode)]
pub struct BeaconKitUpdate {
	/// The header of the Beacon Block being verified
	pub beacon_header: BeaconBlockHeader,
	/// The aggregate BLS signature covering the header
	pub signature: BlsSignature,
	/// The public keys of the validators that signed this update
	pub signers: Vec<BlsPublicKey>,
	/// The execution payload header to be verified against the Beacon state
	pub execution_payload: ExecutionPayloadHeader<BYTES_PER_LOGS_BLOOM, MAX_EXTRA_DATA_BYTES>,
	/// The SSZ Merkle proof of the execution payload
	pub execution_payload_proof: Vec<H256>,
	/// Optional proof for the validator set, for auhtority set rotation
	pub validator_set_proof: Option<ValidatorSetProof>,
}

/// Proof data for verifying the next validator set
#[derive(Debug, Clone, Encode, Decode)]
pub struct ValidatorSetProof {
	/// The list of validators in the new set
	pub validators: Vec<BlsPublicKey>,
	/// The SSZ Merkle proof of the validator registry
	pub proof: Vec<H256>,
}

/// The result of a successful light client verification
#[derive(Debug, Clone, Encode, Decode)]
pub struct VerificationResult {
	/// The hash of the verified block (signing root)
	pub hash: H256,
	/// The verified beacon block header
	pub finalized_header: BeaconBlockHeader,
	/// The next authority set for a verified rotation
	pub next_validators: Option<Vec<BlsPublicKey>>,
}
