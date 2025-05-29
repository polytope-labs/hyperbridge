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

use alloc::{
	string::{String, ToString},
	vec,
	vec::Vec,
};
use anyhow::anyhow;
use frame_support::pallet_prelude::*;
use ismp::host::StateMachine;
use pallet_ismp_host_executive::EvmHosts;
use polkadot_sdk::*;
use primitive_types::{H160, H256, U256};
use token_gateway_primitives::{
	AssetMetadata, GatewayAssetRegistration as GatewayAssetReg, GatewayAssetUpdate,
};

/// Allows a user to update their multi-chain native token potentially on multiple chains
#[derive(
	Debug,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	Default,
)]
pub struct ERC6160AssetUpdate {
	/// The asset identifier
	pub asset_id: H256,
	/// Chains to add support for the asset on
	pub add_chains: BoundedVec<ChainWithSupply, ConstU32<100>>,
	/// Chains to delist the asset from
	pub remove_chains: BoundedVec<StateMachine, ConstU32<100>>,
	/// Chains to change the asset admin on
	pub new_admins: BoundedVec<(StateMachine, H160), ConstU32<100>>,
}

impl From<GatewayAssetReg> for ERC6160AssetRegistration {
	fn from(value: GatewayAssetReg) -> Self {
		ERC6160AssetRegistration {
			name: value.name,
			symbol: value.symbol,
			chains: value
				.chains
				.into_iter()
				.map(|chain| ChainWithSupply { chain, supply: None })
				.collect(),
			minimum_balance: value.minimum_balance,
		}
	}
}

impl From<GatewayAssetUpdate> for ERC6160AssetUpdate {
	fn from(value: GatewayAssetUpdate) -> Self {
		ERC6160AssetUpdate {
			asset_id: value.asset_id,
			add_chains: value
				.add_chains
				.into_iter()
				.map(|chain| ChainWithSupply { chain, supply: None })
				.collect::<Vec<_>>()
				.try_into()
				.expect("Bothe vectors are bounded by the same value"),
			remove_chains: value.remove_chains,
			new_admins: value.new_admins,
		}
	}
}
/// Initial supply options on a per-chain basis
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct InitialSupply {
	/// The beneficiary for the initial supply
	pub beneficiary: H160,
	/// The total initial supply
	pub initial_supply: U256,
}

/// Initial supply options on a per-chain basis
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct ChainWithSupply {
	/// The supported chain
	pub chain: StateMachine,
	/// Initial supply for this chain
	pub supply: Option<InitialSupply>,
}

/// Protocol parameters
#[derive(
	Debug,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	Default,
)]
pub struct Params<Balance> {
	/// The asset registration fee in native tokens, collected by the treasury
	pub registration_fee: Balance,
}

/// Struct for updating the protocol parameters for the TokenGovernor
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct ParamsUpdate<Balance> {
	/// The asset registration fee in native tokens, collected by the treasury
	pub registration_fee: Option<Balance>,
}

/// Holds data required for multi-chain native asset registration
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct ERC6160AssetRegistration {
	/// The asset name
	pub name: BoundedVec<u8, ConstU32<50>>,
	/// The asset symbol
	pub symbol: BoundedVec<u8, ConstU32<20>>,
	/// The list of chains to create the asset on along with their the initial supply on the
	/// provided chains
	pub chains: Vec<ChainWithSupply>,
	/// Minimum balance for the asset, for substrate chains,
	pub minimum_balance: Option<u128>,
}

/// Holds data required for multi-chain native asset registration (unsigned)
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct UnsignedERC6160AssetRegistration<AccountId> {
	/// Registration information
	pub asset: ERC6160AssetRegistration,
	/// Proof of payment
	pub signature: BoundedVec<u8, ConstU32<65>>,
	/// Substrate account which owns this asset and is able to update it
	pub owner: AccountId,
}

/// Registration parameters for existing ERC20 tokens
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct ERC20AssetRegistration {
	/// The asset name
	pub name: BoundedVec<u8, ConstU32<50>>,
	/// The asset symbol
	pub symbol: BoundedVec<u8, ConstU32<20>>,
	/// Chains to support as well as the current ERC20 address on that chain
	pub chains: Vec<AssetRegistration>,
}

#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct AssetRegistration {
	/// Chain to register this asset on
	pub chain: StateMachine,
	/// Optional ERC20 address
	pub erc20: Option<H160>,
	/// Optional ERC6160 address
	pub erc6160: Option<H160>,
}

/// Protocol Parameters for the TokenRegistrar contract
#[derive(
	Debug,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	Default,
)]
pub struct RegistrarParams {
	// Ismp host
	pub host: H160,
	// registration base fee
	pub base_fee: U256,
	/// registrar address
	pub address: H160,
}

/// Struct for updating the protocol parameters for a TokenRegistrar
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct RegistrarParamsUpdate {
	// registration base fee
	pub base_fee: Option<U256>,
	/// registrar address
	pub address: Option<H160>,
}

/// Protocol Parameters for the TokenGateway contract
#[derive(
	Debug,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	Default,
)]
pub struct GatewayParams<T> {
	/// The Ismp host address
	pub host: H160,
	/// Contract for dispatching calls in `AssetWithCall`
	pub call_dispatcher: H160,
	/// Token gateway address
	pub address: T,
}

/// Struct for updating the protocol parameters for a TokenGateway
#[derive(
	Debug,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	Default,
)]
pub struct GatewayParamsUpdate<T> {
	/// Contract for dispatching calls in `AssetWithCall`
	pub call_dispatcher: Option<H160>,
	/// Token gateway  address
	pub address: Option<T>,
}

/// Describes the token gateway module on a given chain
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct ContractInstance {
	/// The associated chain
	pub chain: StateMachine,
	// The token gateway module id on this chain
	pub module_id: H160,
}

/// Describes the token gateway module on a given chain
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct NewIntentGatewayDeployment {
	/// The associated chain
	pub chain: StateMachine,
	// The intent gateway module id on this chain
	pub module_id: H256,
}

impl<B: Clone> Params<B> {
	pub fn update(&self, update: ParamsUpdate<B>) -> Params<B> {
		let mut params = self.clone();

		if let Some(registration_fee) = update.registration_fee {
			params.registration_fee = registration_fee;
		}

		params
	}
}

impl RegistrarParams {
	/// Convenience method for updating protocol params
	pub fn update<T: crate::Config>(
		&self,
		state_machine: &StateMachine,
		update: RegistrarParamsUpdate,
	) -> RegistrarParams {
		let mut params = self.clone();

		if let Some(host) = EvmHosts::<T>::get(state_machine) {
			params.host = host;
		}

		if let Some(address) = update.address {
			params.address = address;
		}

		if let Some(base_fee) = update.base_fee {
			params.base_fee = base_fee;
		}

		params
	}
}

impl<H: Clone> GatewayParams<H> {
	/// Convenience method for updating protocol params
	pub fn update<T: crate::Config>(
		&self,
		state_machine: &StateMachine,
		update: GatewayParamsUpdate<H>,
	) -> GatewayParams<H> {
		let mut params = self.clone();

		if let Some(host) = EvmHosts::<T>::get(state_machine) {
			params.host = host;
		}

		if let Some(address) = update.address {
			params.address = address;
		}

		if let Some(call_dispatcher) = update.call_dispatcher {
			params.call_dispatcher = call_dispatcher;
		}

		params
	}
}

alloy_sol_macro::sol! {
	#![sol(all_derives)]

	// Params for both the IntentGateway and TokenGateway
	struct SolGatewayParams {
		// address of the IsmpHost contract on this chain
		address host;
		// dispatcher for delegating external calls
		address dispatcher;
	}

	struct SolNewIntentGatewayDeployment {
		// Identifier for the state machine.
		bytes stateMachineId;
		// A bytes32 variable to store the gateway identifier.
		bytes32 gateway;
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
	}

	struct SolDeregsiterAsset {
	   // List of assets to deregister
		bytes32[] assetIds;
	}

	struct SolChangeAssetAdmin {
		// Address of the asset
		bytes32 assetId;
		// The address of the new admin
		address newAdmin;
	}

	struct SolContractInstance {
		// The state machine identifier for this chain
		bytes chain;
		// The token gateway contract address on this chain
		address moduleId;
	}

	struct SolRegistrarParams {
		// Ismp host
		address host;
		// registration base fee
		uint256 baseFee;
	}

	struct SolRequestBody {
		// The asset owner
		address owner;
		// The assetId to create
		bytes32 assetId;
	}
}

impl From<NewIntentGatewayDeployment> for SolNewIntentGatewayDeployment {
	fn from(value: NewIntentGatewayDeployment) -> Self {
		SolNewIntentGatewayDeployment {
			stateMachineId: value.chain.to_string().as_bytes().to_vec().into(),
			gateway: value.module_id.0.into(),
		}
	}
}

impl From<GatewayParams<H160>> for SolGatewayParams {
	fn from(value: GatewayParams<H160>) -> Self {
		SolGatewayParams { host: value.host.0.into(), dispatcher: value.call_dispatcher.0.into() }
	}
}

impl From<RegistrarParams> for SolRegistrarParams {
	fn from(value: RegistrarParams) -> Self {
		SolRegistrarParams {
			host: value.host.0.into(),
			baseFee: alloy_primitives::U256::from_limbs(value.base_fee.0),
		}
	}
}

impl From<ContractInstance> for SolContractInstance {
	fn from(value: ContractInstance) -> Self {
		SolContractInstance {
			chain: value.chain.to_string().as_bytes().to_vec().into(),
			moduleId: value.module_id.0.into(),
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
			..Default::default()
		};

		Ok(set_asset)
	}
}

/// Provides a way to encode the request body intended for the `TokenGateway` contract
pub trait TokenGatewayRequest {
	/// Should encode a request to be processed by the `TokenGateway` contract
	fn encode_request(&self) -> Vec<u8>;
}

impl TokenGatewayRequest for SolGatewayParams {
	/// Encodes the SetAsste alongside the enum variant for the TokenGateway request
	fn encode_request(&self) -> Vec<u8> {
		use alloy_sol_types::SolType;

		let variant = vec![1u8]; // enum variant on token gateway
		let encoded = SolGatewayParams::abi_encode(self);

		[variant, encoded].concat()
	}
}

impl TokenGatewayRequest for SolAssetMetadata {
	/// Encodes the SetAsste alongside the enum variant for the TokenGateway request
	fn encode_request(&self) -> Vec<u8> {
		use alloy_sol_types::SolType;

		let variant = vec![2u8]; // enum variant on token gateway
		let encoded = SolAssetMetadata::abi_encode(self);

		[variant, encoded].concat()
	}
}

impl TokenGatewayRequest for SolDeregsiterAsset {
	fn encode_request(&self) -> Vec<u8> {
		use alloy_sol_types::SolType;

		let variant = vec![3u8]; // enum variant on token gateway
		let encoded = SolDeregsiterAsset::abi_encode(self);

		[variant, encoded].concat()
	}
}

impl TokenGatewayRequest for SolChangeAssetAdmin {
	fn encode_request(&self) -> Vec<u8> {
		use alloy_sol_types::SolType;

		let variant = vec![4u8]; // enum variant on token gateway
		let encoded = SolChangeAssetAdmin::abi_encode(self);

		[variant, encoded].concat()
	}
}

impl TokenGatewayRequest for SolContractInstance {
	fn encode_request(&self) -> Vec<u8> {
		use alloy_sol_types::SolType;

		let variant = vec![5u8]; // enum variant on token gateway
		let encoded = SolContractInstance::abi_encode(self);

		[variant, encoded].concat()
	}
}
