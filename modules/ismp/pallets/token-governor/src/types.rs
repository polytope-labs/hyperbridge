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

//! Pallet types

use alloc::vec::Vec;
use anyhow::anyhow;
use frame_support::pallet_prelude::*;
use ismp::host::StateMachine;
use primitive_types::{H160, H256, U256};

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
	pub relayer_fee: U256,
	/// Associated fee percentage for the gateway protocol
	pub protocol_fee: U256,
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
}

/// Allows a user to update their multi-chain native token potentially on multiple chains
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq, Default)]
pub struct ERC6160AssetUpdate {
	/// The asset identifier
	pub asset_id: H256,
	/// The asset logo
	pub logo: BoundedVec<u8, ConstU32<MEGABYTE>>,
	/// Chains to add support for the asset on
	pub add_chains: BoundedVec<StateMachine, ConstU32<100>>,
	/// Chains to delist the asset from
	pub remove_chains: BoundedVec<StateMachine, ConstU32<100>>,
	/// Chains to change the asset admin on
	pub new_admin: BoundedVec<(StateMachine, H160), ConstU32<100>>,
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
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq, Default)]
pub struct Params<Balance> {
	/// The address of the token gateway contract across all chains
	pub token_gateway_address: H160,
	/// The address of the token registrar contract across all chains
	pub token_registrar_address: H160,
	/// The asset registration fee in native tokens, collected by the treasury
	pub registration_fee: Balance,
}

/// Struct for updating the protocol parameters for the TokenGovernor
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq)]
pub struct ParamsUpdate<Balance> {
	/// The address of the token gateway contract across all chains
	pub token_gateway_address: Option<H160>,
	/// The address of the token registrar contract across all chains
	pub token_registrar_address: Option<H160>,
	/// The asset registration fee in native tokens, collected by the treasury
	pub registration_fee: Option<Balance>,
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

/// Protocol Parameters for the TokenRegistrar
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq, Default)]
pub struct RegistrarParams {
	// The ERC20 contract address for the wrapped version of the local native token
	pub erc20_native_token: H160,
	// Ismp host
	pub host: H160,
	// Local UniswapV2 contract address
	pub uniswap_v2: H160,
	// registration base fee
	pub base_fee: U256,
}

/// Struct for updating the protocol parameters for a TokenRegistrar
#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo, PartialEq, Hash, Eq)]
pub struct RegistrarParamsUpdate {
	// The ERC20 contract address for the wrapped version of the local native token
	pub erc20_native_token: Option<H160>,
	// Ismp host
	pub host: Option<H160>,
	// Local UniswapV2 contract address
	pub uniswap_v2: Option<H160>,
	// registration base fee
	pub base_fee: Option<U256>,
}

impl<B: Clone> Params<B> {
	pub fn update(&self, update: ParamsUpdate<B>) -> Params<B> {
		let mut params = self.clone();
		if let Some(token_gateway_address) = update.token_gateway_address {
			params.token_gateway_address = token_gateway_address;
		}

		if let Some(token_registrar_address) = update.token_registrar_address {
			params.token_registrar_address = token_registrar_address;
		}

		if let Some(registration_fee) = update.registration_fee {
			params.registration_fee = registration_fee;
		}

		params
	}
}

impl RegistrarParams {
	/// Convenience method for updating protocol params
	pub fn update(&self, update: RegistrarParamsUpdate) -> RegistrarParams {
		let mut params = self.clone();
		if let Some(erc20_native_token) = update.erc20_native_token {
			params.erc20_native_token = erc20_native_token;
		}

		if let Some(host) = update.host {
			params.host = host;
		}

		if let Some(uniswap_v2) = update.uniswap_v2 {
			params.uniswap_v2 = uniswap_v2;
		}

		if let Some(base_fee) = update.base_fee {
			params.base_fee = base_fee;
		}

		params
	}
}

alloy_sol_macro::sol! {
	#![sol(all_derives)]

	struct SolAssetFees {
		// Fee percentage paid to relayers for this asset
		uint256 relayerFee;
		// Fee percentage paid to the protocol for this asset
		uint256 protocolFee;
	}

	struct SolAssetMetadata {
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
	   SolAssetFees fees;
	}

	struct SolRequestBody {
		// The asset owner
		address owner;
		// The assetId to create
		bytes32 assetId;
		// The base fee paid for registration, used in timeouts
		uint256 baseFee;
	}

	struct SolRegistrarParams {
		// The ERC20 contract address for the wrapped version of the local native token
		address erc20NativeToken;
		// Ismp host
		address host;
		// Local UniswapV2 contract address
		address uniswapV2;
		// registration base fee
		uint256 baseFee;
	}
}

impl From<RegistrarParams> for SolRegistrarParams {
	fn from(value: RegistrarParams) -> Self {
		SolRegistrarParams {
			erc20NativeToken: value.erc20_native_token.0.into(),
			host: value.host.0.into(),
			uniswapV2: value.uniswap_v2.0.into(),
			baseFee: alloy_primitives::U256::from_limbs(value.base_fee.0),
		}
	}
}

impl From<AssetFees> for SolAssetFees {
	fn from(value: AssetFees) -> Self {
		SolAssetFees {
			protocolFee: alloy_primitives::U256::from_limbs(value.protocol_fee.0),
			relayerFee: alloy_primitives::U256::from_limbs(value.relayer_fee.0),
		}
	}
}

// This is used for updating the asset metadata on the EVM chains
impl TryFrom<AssetMetadata> for SolAssetMetadata {
	type Error = anyhow::Error;
	fn try_from(value: AssetMetadata) -> Result<Self, anyhow::Error> {
		let set_asset = SolAssetMetadata {
			name: String::from_utf8(value.name.as_slice().to_vec())
				.map_err(|err| anyhow!("Name was not valid Utf8Error: {err:?}"))?,
			symbol: String::from_utf8(value.symbol.as_slice().to_vec())
				.map_err(|err| anyhow!("Name was not valid Utf8Error: {err:?}"))?,
			fees: value.fees.into(),
			..Default::default()
		};

		Ok(set_asset)
	}
}

impl SolAssetMetadata {
	/// Encodes the SetAsste alongside the enum variant for the TokenGateway request
	pub fn encode(&self) -> Vec<u8> {
		use alloy_sol_types::SolType;

		let variant = vec![2u8]; // enum variant on token gateway
		let encoded = SolAssetMetadata::abi_encode(self);

		[variant, encoded].concat()
	}
}
