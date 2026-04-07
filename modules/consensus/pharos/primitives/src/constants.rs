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

/// Testnet (Atlantic) epoch length in seconds (30 minutes).
pub const TESTNET_EPOCH_LENGTH_SECS: u64 = 30 * 60; // 1800 seconds

/// Storage slot index for `currentEpoch` on the Pharos staking precompile.
/// This stores the current epoch **number** (increments every epoch duration).
pub const CURRENT_EPOCH_SLOT: u64 = 5;

/// Pharos Mainnet chain ID
pub const PHAROS_MAINNET_CHAIN_ID: u32 = 688600;

/// Pharos Atlantic Testnet chain ID
pub const PHAROS_ATLANTIC_CHAIN_ID: u32 = 688689;

/// Default withdraw window in epochs from the Pharos staking contract.
pub const DEFAULT_WITHDRAW_WINDOW_EPOCHS: u64 = 84;

/// Configuration trait for Pharos network parameters.
///
/// Pharos epochs are **time-based** (not block-count-based). The epoch number
/// is stored on-chain at slot 5 of the staking precompile and increments every
/// `EPOCH_LENGTH_SECS` seconds. There is no fixed number of blocks per epoch.
///
/// Epoch determination requires either:
/// - Reading `currentEpoch` from the staking contract (off-chain prover)
/// - Verifying a storage proof of slot 5 against the block's state root (on-chain verifier)
pub trait Config: Clone + Send + Sync {
	/// The epoch length in seconds
	const EPOCH_LENGTH_SECS: u64;

	/// The chain ID for this network
	const CHAIN_ID: u64;

	/// Network identifier
	const ID: [u8; 4];

	/// The unstaking period in seconds (withdraw_window_epochs × epoch_length_secs).
	/// Defaults to `DEFAULT_WITHDRAW_WINDOW_EPOCHS × EPOCH_LENGTH_SECS`.
	const UNBONDING_PERIOD: u64 = DEFAULT_WITHDRAW_WINDOW_EPOCHS * Self::EPOCH_LENGTH_SECS;
}

/// Pharos Mainnet configuration
#[derive(Clone, Default, Debug)]
pub struct Mainnet;

impl Config for Mainnet {
	/// 4 hours epoch length
	const EPOCH_LENGTH_SECS: u64 = MAINNET_EPOCH_LENGTH_SECS;

	const CHAIN_ID: u64 = 688600;

	const ID: [u8; 4] = PHAROS_CONSENSUS_ID;
}

/// Pharos Testnet configuration
#[derive(Clone, Default, Debug)]
pub struct Testnet;

impl Config for Testnet {
	/// 30 minutes epoch length
	const EPOCH_LENGTH_SECS: u64 = TESTNET_EPOCH_LENGTH_SECS;

	/// Pharos Testnet chain ID
	const CHAIN_ID: u64 = 688689;

	const ID: [u8; 4] = PHAROS_CONSENSUS_ID;
}
