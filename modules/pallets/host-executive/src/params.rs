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
use codec::DecodeWithMemTracking;
use frame_support::{pallet_prelude::ConstU32, BoundedVec};
use pallet_hyperbridge::VersionedHostParams;
use polkadot_sdk::*;
use primitive_types::{H160, H256, U256};
use sp_runtime::RuntimeDebug;

/// The host parameters of all connected chains
#[derive(
	Clone,
	codec::Encode,
	codec::Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	RuntimeDebug,
)]
pub enum HostParam<T> {
	/// Host params for substrate-based hosts
	SubstrateHostParam(VersionedHostParams<T>),
	/// Host params for evm-based hosts
	EvmHostParam(EvmHostParam),
}

/// Struct for modifying the host parameters of all connected chains
#[derive(
	Clone,
	codec::Encode,
	codec::Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	RuntimeDebug,
)]
pub enum HostParamUpdate<T> {
	/// Host param updates for substrate-based hosts
	SubstrateHostParam(VersionedHostParams<T>),
	/// Host params updates for evm-based hosts
	EvmHostParam(EvmHostParamUpdate),
}

/// Per-byte-fee for chains
#[derive(
	Clone,
	codec::Encode,
	codec::Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	RuntimeDebug,
	Default,
)]
pub struct PerByteFee {
	/// keccak256 hash of the state machine id
	pub state_id: H256,
	/// The fee to charge per byte
	pub per_byte_fee: U256,
}

/// The host parameters for evm-based hosts
#[derive(
	Clone,
	codec::Encode,
	codec::Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	RuntimeDebug,
	Default,
)]
pub struct EvmHostParam {
	/// the minimum default timeout in seconds
	pub default_timeout: u128,
	/// The fee to charge per byte
	pub default_per_byte_fee: U256,
	/// The cost for applications to access the hyperbridge state commitment.
	/// They might do so because the hyperbridge state contains the verified state commitments
	/// for all chains and they want to directly read the state of these chains state bypassing
	/// the ISMP protocol entirely.
	pub state_commitment_fee: U256,
	/// The address of the fee token contract
	pub fee_token: H160,
	/// The admin account
	pub admin: H160,
	/// The handler contract
	pub handler: H160,
	/// The host manager contract
	pub host_manager: H160,
	// The local UniswapV2Router02 contract, used for swapping the native token to the feeToken.
	pub uniswap_v2: H160,
	/// The unstaking period in seconds
	pub un_staking_period: u128,
	/// The configured challenge period
	pub challenge_period: u128,
	/// The consensus client contract
	pub consensus_client: H160,
	/// The state machine identifier for hyperbridge
	pub state_machines: BoundedVec<u32, ConstU32<1_000>>,
	/// The cost of cross-chain requests charged in the feeToken, per byte.
	/// Different destination chains can have different per byte fees.
	pub per_byte_fees: BoundedVec<PerByteFee, ConstU32<1_000>>,
	/// The state machine identifier for hyperbridge
	pub hyperbridge: BoundedVec<u8, ConstU32<1_000>>,
}

impl EvmHostParam {
	/// Update the host params with the update struct. Will only modify fields that are set.
	pub fn update(&mut self, update: EvmHostParamUpdate) {
		if let Some(default_timeout) = update.default_timeout {
			self.default_timeout = default_timeout;
		}

		if let Some(per_byte_fee) = update.default_per_byte_fee {
			self.default_per_byte_fee = per_byte_fee;
		}

		if let Some(state_commitment_fee) = update.state_commitment_fee {
			self.state_commitment_fee = state_commitment_fee;
		}

		if let Some(uniswap_v2) = update.uniswap_v2 {
			self.uniswap_v2 = uniswap_v2;
		}

		if let Some(fee_token_address) = update.fee_token {
			self.fee_token = fee_token_address;
		}

		if let Some(admin) = update.admin {
			self.admin = admin;
		}

		if let Some(handler) = update.handler {
			self.handler = handler;
		}

		if let Some(host_manager) = update.host_manager {
			self.host_manager = host_manager;
		}

		if let Some(un_staking_period) = update.un_staking_period {
			self.un_staking_period = un_staking_period;
		}

		if let Some(challenge_period) = update.challenge_period {
			self.challenge_period = challenge_period;
		}

		if let Some(consensus_client) = update.consensus_client {
			self.consensus_client = consensus_client;
		}

		if let Some(state_machine_whitelist) = update.state_machines {
			self.state_machines = state_machine_whitelist;
		}

		if let Some(per_byte_fees) = update.per_byte_fees {
			self.per_byte_fees = per_byte_fees;
		}

		if let Some(hyperbridge) = update.hyperbridge {
			self.hyperbridge = hyperbridge;
		}
	}
}

/// Struct for modifying the host params
#[derive(
	Clone,
	codec::Encode,
	codec::Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	RuntimeDebug,
	Default,
)]
pub struct EvmHostParamUpdate {
	/// the minimum default timeout in seconds
	pub default_timeout: Option<u128>,
	/// The default per byte fee
	pub default_per_byte_fee: Option<U256>,
	/// The address of the fee token contract.
	/// It's important that before changing this parameter,
	/// that all funds have been drained from the previous feeToken
	pub fee_token: Option<H160>,
	/// The cost for applications to access the hyperbridge state commitment.
	/// They might do so because the hyperbridge state contains the verified state commitments
	/// for all chains and they want to directly read the state of these chains state bypassing
	/// the ISMP protocol entirely.
	pub state_commitment_fee: Option<U256>,
	/// The admin account
	pub admin: Option<H160>,
	/// The handler contract
	pub handler: Option<H160>,
	/// The host manager contract
	pub host_manager: Option<H160>,
	// The local UniswapV2Router02 contract, used for swapping the native token to the feeToken.
	pub uniswap_v2: Option<H160>,
	/// The unstaking period in seconds
	pub un_staking_period: Option<u128>,
	/// The configured challenge period
	pub challenge_period: Option<u128>,
	/// The consensus client contract
	pub consensus_client: Option<H160>,
	/// The state machine identifier for hyperbridge
	pub state_machines: Option<BoundedVec<u32, ConstU32<1_000>>>,
	/// The cost of cross-chain requests charged in the feeToken, per byte.
	/// Different destination chains can have different per byte fees.
	pub per_byte_fees: Option<BoundedVec<PerByteFee, ConstU32<1_000>>>,
	/// The state machine identifier for hyperbridge
	pub hyperbridge: Option<BoundedVec<u8, ConstU32<1_000>>>,
}

alloy_sol_macro::sol! {
	#![sol(all_derives)]

	// Per-byte-fee for chains
	struct PerByteFeeAbi {
		// keccak256 hash of the state machine id
		bytes32 stateIdHash;
		// Per byte fee for this destination chain
		uint256 perByteFee;
	}

	// The IsmpHost parameters
	struct EvmHostParamsAbi {
		// default timeout in seconds for requests.
		uint256 defaultTimeout;
		// cost of cross-chain requests in the fee token per byte
		uint256 defaultPerByteFee;
		// The cost for applications to access the hyperbridge state commitment.
		// They might do so because the hyperbridge state contains the verified state commitments
		// for all chains and they want to directly read the state of these chains state bypassing
		// the ISMP protocol entirely.
		uint256 stateCommitmentFee;
		// The fee token contract. This will typically be DAI.
		// but we allow it to be configurable to prevent future regrets.
		address feeToken;
		// admin account, this only has the rights to freeze, or unfreeze the bridge
		address admin;
		// Ismp request/response handler
		address handler;
		// the authorized host manager contract
		address hostManager;
		// The local UniswapV2Router02 contract, used for swapping the native token to the feeToken.
		address uniswapV2;
		// unstaking period
		uint256 unStakingPeriod;
		// minimum challenge period in seconds;
		uint256 challengePeriod;
		// consensus client contract
		address consensusClient;
		// whitelisted state machines
		uint256[] stateMachines;
		// The cost of cross-chain requests charged in the feeToken, per byte.
		// Different destination chains can have different per byte fees.
		PerByteFeeAbi[] perByteFees;
		// state machine identifier for hyperbridge
		bytes hyperbridge;
	}
}

impl EvmHostParamsAbi {
	/// Encodes the HostParams alongside the enum variant for the HostManager request
	pub fn encode(&self) -> Vec<u8> {
		use alloy_sol_types::SolType;

		let variant = vec![1u8]; // enum variant for the host manager
		let encoded = EvmHostParamsAbi::abi_encode(self);

		[variant, encoded].concat()
	}
}

impl TryFrom<EvmHostParam> for EvmHostParamsAbi {
	type Error = anyhow::Error;

	fn try_from(value: EvmHostParam) -> Result<Self, anyhow::Error> {
		Ok(EvmHostParamsAbi {
			defaultTimeout: value.default_timeout.try_into().map_err(anyhow::Error::msg)?,
			defaultPerByteFee: {
				alloy_primitives::U256::from_le_bytes(value.default_per_byte_fee.to_little_endian())
			},
			stateCommitmentFee: {
				alloy_primitives::U256::from_le_bytes(value.state_commitment_fee.to_little_endian())
			},
			feeToken: value.fee_token.0.try_into().map_err(anyhow::Error::msg)?,
			admin: value.admin.0.try_into().map_err(anyhow::Error::msg)?,
			handler: value.handler.0.try_into().map_err(anyhow::Error::msg)?,
			hostManager: value.host_manager.0.try_into().map_err(anyhow::Error::msg)?,
			uniswapV2: value.uniswap_v2.0.try_into().map_err(anyhow::Error::msg)?,
			unStakingPeriod: value.un_staking_period.try_into().map_err(anyhow::Error::msg)?,
			challengePeriod: value.challenge_period.try_into().map_err(anyhow::Error::msg)?,
			consensusClient: value.consensus_client.0.try_into().map_err(anyhow::Error::msg)?,
			stateMachines: value
				.state_machines
				.into_iter()
				.map(|id| id.try_into().map_err(anyhow::Error::msg))
				.collect::<Result<Vec<_>, anyhow::Error>>()?,
			hyperbridge: value.hyperbridge.to_vec().into(),
			perByteFees: value
				.per_byte_fees
				.into_iter()
				.map(|p| {
					Ok::<_, anyhow::Error>(PerByteFeeAbi {
						stateIdHash: p.state_id.0.try_into().map_err(anyhow::Error::msg)?,
						perByteFee: {
							alloy_primitives::U256::from_le_bytes(p.per_byte_fee.to_little_endian())
						},
					})
				})
				.collect::<Result<Vec<_>, _>>()?,
		})
	}
}
