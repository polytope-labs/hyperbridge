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

//! SCALE-encoded mirrors of the EVM-side `HostParams` and `WithdrawParams`
//! structs, plus their `TryFrom`/`From` conversions to the ABI types generated
//! in [`crate::evm_host`]. Keeping the SCALE and ABI representations in the
//! same crate sidesteps the orphan rule so the conversions live next to both
//! type definitions.

use crate::evm_host::EvmHost::{
	HostParams as EvmHostParamsAbi, WithdrawParams as WithdrawParamsAbi,
};
use alloc::{vec, vec::Vec};
use alloy_sol_types::SolValue;
use codec::{Decode, DecodeWithMemTracking, Encode};
use polkadot_sdk::frame_support;
use primitive_types::{H160, U256};

use frame_support::{pallet_prelude::ConstU32, BoundedVec};

/// The host parameters of all connected chains.
#[derive(
	Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq, Debug,
)]
pub enum HostParam {
	/// Host params for evm-based hosts
	EvmHostParam(EvmHostParam),
}

/// Struct for modifying the host parameters of all connected chains.
#[derive(
	Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq, Debug,
)]
pub enum HostParamUpdate {
	/// Host params updates for evm-based hosts
	EvmHostParam(EvmHostParamUpdate),
}

/// The host parameters for evm-based hosts
#[derive(
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	Debug,
	Default,
)]
pub struct EvmHostParam {
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
	/// The state machine identifier for hyperbridge
	pub hyperbridge: BoundedVec<u8, ConstU32<1_000>>,
}

impl EvmHostParam {
	/// Update the host params with the update struct. Will only modify fields that are set.
	pub fn update(&mut self, update: EvmHostParamUpdate) {
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

		if let Some(hyperbridge) = update.hyperbridge {
			self.hyperbridge = hyperbridge;
		}
	}
}

/// Struct for modifying the host params
#[derive(
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	PartialEq,
	Eq,
	Debug,
	Default,
)]
pub struct EvmHostParamUpdate {
	/// The address of the fee token contract.
	/// It's important that before changing this parameter,
	/// that all funds have been drained from the previous feeToken
	pub fee_token: Option<H160>,
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
	/// The state machine identifier for hyperbridge
	pub hyperbridge: Option<BoundedVec<u8, ConstU32<1_000>>>,
}

impl EvmHostParam {
	/// Encode the host params alongside the host-manager request variant tag.
	pub fn abi_encode_with_variant(&self) -> Result<Vec<u8>, anyhow::Error> {
		let abi = EvmHostParamsAbi::try_from(self.clone())?;
		Ok(encode_host_params(&abi))
	}
}

/// Encode an [`EvmHostParamsAbi`] alongside the host-manager `SetHostParam`
/// request variant tag (`1`).
pub fn encode_host_params(params: &EvmHostParamsAbi) -> Vec<u8> {
	let variant = vec![1u8]; // enum variant for the host manager
	[variant, params.abi_encode()].concat()
}

impl TryFrom<EvmHostParam> for EvmHostParamsAbi {
	type Error = anyhow::Error;

	fn try_from(value: EvmHostParam) -> Result<Self, Self::Error> {
		Ok(EvmHostParamsAbi {
			feeToken: value.fee_token.0.into(),
			admin: value.admin.0.into(),
			handler: value.handler.0.into(),
			hostManager: value.host_manager.0.into(),
			uniswapV2: value.uniswap_v2.0.into(),
			unStakingPeriod: value.un_staking_period.try_into().map_err(anyhow::Error::msg)?,
			challengePeriod: value.challenge_period.try_into().map_err(anyhow::Error::msg)?,
			consensusClient: value.consensus_client.0.into(),
			stateMachines: value
				.state_machines
				.into_iter()
				.map(|id| id.try_into().map_err(anyhow::Error::msg))
				.collect::<Result<Vec<_>, anyhow::Error>>()?,
			hyperbridge: value.hyperbridge.to_vec().into(),
		})
	}
}

/// SCALE-encoded payload for the `withdraw` extrinsic. Mirrors
/// the on-EVM `struct WithdrawParams { address beneficiary; uint256 amount;
/// address token; }`.
#[derive(
	Debug, Clone, Encode, Decode, DecodeWithMemTracking, scale_info::TypeInfo, PartialEq, Eq,
)]
pub struct WithdrawalParams {
	/// 20-byte EVM beneficiary address. Stored as a `Vec<u8>` so the SCALE
	/// extrinsic input doesn't have to pre-validate the length.
	pub beneficiary_address: Vec<u8>,
	/// Amount to withdraw, in the token's smallest unit.
	pub amount: U256,
	/// ERC20 token contract to withdraw. The EVM host treats the zero address
	/// as the chain's native asset.
	pub token: H160,
}

impl WithdrawalParams {
	/// Encode the withdrawal request as the EVM host expects: a single action
	/// byte (`0`) followed by the ABI-encoded `WithdrawParams` tuple.
	pub fn abi_encode(&self) -> Result<Vec<u8>, anyhow::Error> {
		let abi: WithdrawParamsAbi = self.try_into()?;
		let variant = vec![0u8]; // host manager action: withdraw
		Ok([variant, abi.abi_encode()].concat())
	}
}

/// Errors raised when converting a [`WithdrawalParams`] to its ABI form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WithdrawalParamsError {
	/// The beneficiary address was not exactly 20 bytes. Carries the length
	/// that was actually received.
	#[error("invalid beneficiary address: expected 20 bytes, got {0}")]
	InvalidBeneficiaryAddressLength(usize),
}

impl TryFrom<&WithdrawalParams> for WithdrawParamsAbi {
	type Error = WithdrawalParamsError;

	fn try_from(value: &WithdrawalParams) -> Result<Self, Self::Error> {
		// Reject anything that isn't exactly a 20-byte EVM address.
		if value.beneficiary_address.len() != 20 {
			Err(WithdrawalParamsError::InvalidBeneficiaryAddressLength(
				value.beneficiary_address.len(),
			))?;
		}
		let beneficiary = H160::from_slice(&value.beneficiary_address[..]);

		Ok(WithdrawParamsAbi {
			beneficiary: beneficiary.0.into(),
			amount: alloy_primitives::U256::from_be_bytes(value.amount.to_big_endian()),
			token: value.token.0.into(),
		})
	}
}

#[cfg(test)]
mod test {
	use super::WithdrawalParams;
	use primitive_types::{H160, U256};

	#[test]
	fn check_encoding() {
		let params = WithdrawalParams {
			beneficiary_address: H160::random().0.to_vec(),
			amount: U256::from(500_00_000_000u128),
			token: H160::random(),
		};

		let encoding = params.abi_encode().expect("20-byte address encodes cleanly");

		// 1 action byte + 3 * 32-byte ABI-encoded fields.
		assert_eq!(encoding.len(), 1 + 96);
	}

	#[test]
	fn rejects_non_20_byte_beneficiary() {
		let short = WithdrawalParams {
			beneficiary_address: vec![0u8; 19],
			amount: U256::from(1u128),
			token: H160::zero(),
		};
		assert!(short.abi_encode().is_err());

		let long = WithdrawalParams {
			beneficiary_address: vec![0u8; 32],
			amount: U256::from(1u128),
			token: H160::zero(),
		};
		assert!(long.abi_encode().is_err());
	}
}
