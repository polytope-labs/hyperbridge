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
use alloc::vec::Vec;
use frame_support::pallet_prelude::*;
use ismp::host::StateMachine;
use primitive_types::{H160, U256};

/// Number of bytes in a megabyte (MB)
const MEGABYTE: u32 = 1024;

/// Holds metadata relevant to a multi-chain native asset
#[derive(
	Debug,
	Clone,
	Encode,
	Decode,
	scale_info::TypeInfo,
	PartialEq,
	Hash,
	Eq,
	codec::MaxEncodedLen,
	Default,
)]
pub struct AssetFees {
	/// Associated fee percentage for liquidity providers
	pub relayer_fee: u128,
	/// Associated fee percentage for the gateway protocol
	pub protocol_fee: u128,
}

/// Holds metadata relevant to a multi-chain native asset
#[derive(
	Debug,
	Clone,
	Encode,
	Decode,
	scale_info::TypeInfo,
	PartialEq,
	Hash,
	Eq,
	codec::MaxEncodedLen,
	Default,
)]
pub struct AssetMetadata {
	/// The asset name
	pub name: BoundedVec<u8, ConstU32<20>>,
	/// The asset symbol
	pub symbol: BoundedVec<u8, ConstU32<20>>,
	/// The asset logo
	pub logo: BoundedVec<u8, ConstU32<MEGABYTE>>,
	/// Associated protocol fees
	pub fees: AssetFees,
	/// The Associated ERC20 token contract
	pub erc20: H160,
	/// The Associated ERC6160 token contract
	pub erc6160: H160,
}

/// Initial supply options on a per-chain basis
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq)]
pub struct InitialSupply {
	/// The beneficiary for the initial supply
	pub beneficiary: H160,
	/// The total initial supply
	pub initial_supply: U256,
}

/// Initial supply options on a per-chain basis
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq)]
pub struct ChainWithSupply {
	/// The supported chain
	pub chain: StateMachine,
	/// Initial supply for this chain
	pub supply: Option<InitialSupply>,
}

/// Protocol parameters
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq)]
pub struct Params<Balance> {
	/// The chain to set the initial suuply
	pub token_gateway_address: H160,
	/// The chain to set the initial suuply
	pub token_registrar_address: H160,
	/// The beneficiary for the initial supply
	pub registration_fee: Balance,
}

/// Holds data required for multi-chain native asset registration
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq)]
pub struct ERC6160AssetRegistration {
	/// The asset name
	pub name: BoundedVec<u8, ConstU32<20>>,
	/// The asset symbol
	pub symbol: BoundedVec<u8, ConstU32<20>>,
	/// The asset logo
	pub logo: BoundedVec<u8, ConstU32<MEGABYTE>>,
	/// The list of chains to create the asset on along with their the initial supply on the
	/// provided chains
	pub chains: Vec<ChainWithSupply>,
}

alloy_sol_macro::sol! {
	#![sol(all_derives)]

	struct Fees {
		// Fee percentage paid to relayers for this asset
		uint256 relayerFee;
		// Fee percentage paid to the protocol for this asset
		uint256 protocolFee;
	}

	struct SetAsset {
	   // ERC20 token contract address for the asset
	   address erc20;
	   // ERC6160 token contract address for the asset
	   address erc6160;
	   // Asset's name
	   string name;
	   // Asset's symbol
	   string symbol;
	   // The initial supply of asset
	   uint256 initialSupply;
	   // Initial beneficiary of the total supply
	   address beneficiary;
	   // Associated fees for this asset
	   Fees fees;
	}

	struct RequestBody {
		// The asset owner
		address owner;
		// The assetId to create
		bytes32 assetId;
		// The base fee paid for registration, used in timeouts
		uint256 baseFee;
	}
}

impl SetAsset {
	/// Encodes the SetAsste alongside the enum variant for the TokenGateway request
	pub fn encode(&self) -> Vec<u8> {
		use alloy_sol_types::SolType;

		let variant = vec![2u8]; // enum variant on token gateway
		let encoded = SetAsset::abi_encode(self);

		[variant, encoded].concat()
	}
}
