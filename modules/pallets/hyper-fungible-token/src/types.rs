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

//! Type definitions for the hyper-fungible-token pallet

use alloc::{collections::BTreeMap, vec::Vec};
use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use ismp::host::StateMachine;
use polkadot_sdk::*;
use sp_core::{ConstU32, H160};

use crate::Config;

/// Local asset ID type alias
pub type AssetId<T> =
	<<T as Config>::Assets as fungibles::Inspect<<T as frame_system::Config>::AccountId>>::AssetId;

use frame_support::traits::fungibles;

// ABI-compatible Message matching the Solidity HyperFungibleToken.Message struct:
// struct Message { bytes from; bytes to; uint256 amount; bytes data; }
alloy_sol_macro::sol! {
	#![sol(all_derives)]
	struct Message {
		bytes from;
		bytes to;
		uint256 amount;
		bytes data;
	}
}

/// Parameters for initiating a cross-chain token transfer
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct SendParams<AssetId, Balance> {
	/// Local asset ID
	pub asset_id: AssetId,
	/// Destination state machine
	pub destination: StateMachine,
	/// Recipient account on the destination chain (up to 32 bytes)
	pub recipient: BoundedVec<u8, ConstU32<32>>,
	/// Amount to send (in local denomination)
	pub amount: Balance,
	/// Request timeout in seconds
	pub timeout: u64,
	/// Relayer fee
	pub relayer_fee: Balance,
	/// Optional calldata to execute on the destination chain
	pub call_data: Option<Vec<u8>>,
}

/// Per-chain configuration for a registered token
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct ChainConfig {
	/// The HyperFungibleToken/WrappedHyperFungibleToken EVM contract address on this chain.
	/// A fixed 20-byte EVM address: this pallet bridges substrate <-> EVM only, so a
	/// (non-EVM) substrate peer module id cannot be registered here.
	pub token_contract: H160,
	/// ERC20 decimals on this chain
	pub decimals: u8,
}

/// Registration parameters for a new token
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct TokenRegistration<AssetId> {
	/// Local asset ID (must already exist in the runtime's asset registry)
	pub local_id: AssetId,
	/// Whether this asset is native to this chain (custody model) or non-native (mint/burn)
	pub native: bool,
	/// Per-chain configuration
	pub chains: BTreeMap<StateMachine, ChainConfig>,
}

/// Parameters for updating an existing token's chain configuration
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct TokenUpdate<AssetId> {
	/// Local asset ID
	pub asset_id: AssetId,
	/// Chains to add or update
	pub add_chains: BTreeMap<StateMachine, ChainConfig>,
	/// Chains to remove
	pub remove_chains: Vec<StateMachine>,
}

/// SCALE-encoded calldata for executing a runtime call on the destination substrate chain
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Eq)]
pub struct SubstrateCalldata {
	/// Optional SCALE-encoded [MultiSignature](sp_runtime::MultiSignature) of the beneficiary's
	/// account nonce and the encoded runtime call
	pub signature: Option<Vec<u8>>,
	/// SCALE-encoded runtime call to execute
	pub runtime_call: Vec<u8>,
}

/// Setup the runtime provides so this pallet's benchmarks can work with a bridged asset.
#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkHelper<T: Config> {
	/// Creates an asset with the given metadata `decimals`, mints `amount` of it to `who` and
	/// returns its id. `who` pays any asset creation and metadata deposits, so it must already
	/// be funded with the native currency.
	fn create_asset(decimals: u8, who: &T::AccountId, amount: u128) -> AssetId<T>;
}

/// Converts an EVM address to a substrate AccountId
pub trait EvmToSubstrate<T: frame_system::Config> {
	fn convert(addr: H160) -> T::AccountId;
}

/// Default implementation: zero-pads the 20-byte address into a 32-byte AccountId
impl<T: frame_system::Config> EvmToSubstrate<T> for ()
where
	<T as frame_system::Config>::AccountId: From<[u8; 32]>,
{
	fn convert(addr: H160) -> <T as frame_system::Config>::AccountId {
		let mut account = [0u8; 32];
		account[12..].copy_from_slice(&addr.0);
		account.into()
	}
}
