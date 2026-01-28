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

//! Constants and configuration for Pharos consensus.

use alloy_primitives::Address;

/// Re-export BLS types from crypto-utils
pub use crypto_utils::{
	BlsPublicKey, BlsSignature, BLS_PUBLIC_KEY_BYTES_LEN, BLS_SIGNATURE_BYTES_LEN,
};

/// The staking contract address where validator set is stored.
/// Address: 0x4100000000000000000000000000000000000000
pub const STAKING_CONTRACT_ADDRESS: Address =
	Address::new([0x41, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

/// Consensus ID for Pharos network
pub const PHAROS_CONSENSUS_ID: [u8; 4] = *b"PHAR";

/// Mainnet epoch length in seconds (4 hours)
pub const MAINNET_EPOCH_LENGTH_SECS: u64 = 4 * 60 * 60; // 14400 seconds

/// Testnet (Atlantic) epoch length in seconds (0.5 hours)
pub const TESTNET_EPOCH_LENGTH_SECS: u64 = 30 * 60; // 1800 seconds

/// Pharos Mainnet chain ID
pub const PHAROS_MAINNET_CHAIN_ID: u32 = 688600;

/// Pharos Atlantic Testnet chain ID
pub const PHAROS_ATLANTIC_CHAIN_ID: u32 = 688689;

/// Configuration trait for Pharos network parameters.
pub trait Config: Clone + Send + Sync {
	/// The epoch length in seconds
	const EPOCH_LENGTH_SECS: u64;

	/// The epoch length in blocks (derived from epoch length and block time)
	const EPOCH_LENGTH_BLOCKS: u64;

	/// The chain ID for this network
	const CHAIN_ID: u64;

	/// Network identifier
	const ID: [u8; 4];

	/// Calculate the epoch number for a given block number
	fn compute_epoch(block_number: u64) -> u64 {
		block_number / Self::EPOCH_LENGTH_BLOCKS
	}

	/// Check if a block is an epoch boundary block (last block of an epoch).
	///
	/// The epoch boundary is defined as the last block of an epoch, i.e.,
	/// `(block_number + 1) % epoch_length == 0`.
	///
	/// At epoch boundaries, the validator set for the next epoch is finalized
	fn is_epoch_boundary(block_number: u64) -> bool {
		(block_number + 1) % Self::EPOCH_LENGTH_BLOCKS == 0
	}

	/// Get the first block number of the next epoch
	fn next_epoch_start(current_block: u64) -> u64 {
		let current_epoch = Self::compute_epoch(current_block);
		(current_epoch + 1) * Self::EPOCH_LENGTH_BLOCKS
	}
}

/// Pharos Mainnet configuration
#[derive(Clone, Default, Debug)]
pub struct Mainnet;

impl Config for Mainnet {
	/// 4 hours epoch length
	const EPOCH_LENGTH_SECS: u64 = MAINNET_EPOCH_LENGTH_SECS;

	/// With ~1 second finality (sub-second), assuming 1 block per second
	/// 4 hours = 14400 blocks
	const EPOCH_LENGTH_BLOCKS: u64 = 14400;

	/// Mainnet chain ID - TBD
	/// Placeholder based on testnet pattern
	const CHAIN_ID: u64 = 688600;

	const ID: [u8; 4] = PHAROS_CONSENSUS_ID;
}

/// Pharos Testnet configuration
#[derive(Clone, Default, Debug)]
pub struct Testnet;

impl Config for Testnet {
	/// 0.5 hours epoch length
	const EPOCH_LENGTH_SECS: u64 = TESTNET_EPOCH_LENGTH_SECS;

	/// With ~1 second finality, 0.5 hours = 1800 blocks
	const EPOCH_LENGTH_BLOCKS: u64 = 1800;

	/// Pharos Testnet chain ID
	const CHAIN_ID: u64 = 688689;

	const ID: [u8; 4] = PHAROS_CONSENSUS_ID;
}
