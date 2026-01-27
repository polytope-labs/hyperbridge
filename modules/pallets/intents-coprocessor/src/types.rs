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

//! Types for the intents pallet

use alloc::{vec, vec::Vec};
use alloy_sol_types::SolValue;
use codec::{Decode, DecodeWithMemTracking, Encode};

use primitive_types::{H160, H256, U256};
use scale_info::TypeInfo;

/// Represents a token and amount pair for cross-chain transfers
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct TokenInfo {
	/// The address of the token (address(0) for native token)
	pub token: H256,
	/// The amount of the token
	pub amount: U256,
}

/// Parameters for an Intent Gateway instance
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct IntentGatewayParams {
	/// The address of the host contract
	pub host: H160,
	/// Address of the dispatcher contract
	pub dispatcher: H160,
	/// Flag indicating whether solver selection is enabled
	pub solver_selection: bool,
	/// The percentage of surplus (in basis points) that goes to the protocol
	/// 10000 = 100%, 5000 = 50%, etc.
	pub surplus_share_bps: U256,
	/// The protocol fee in basis points charged on order inputs
	/// 10000 = 100%, 100 = 1%, etc.
	pub protocol_fee_bps: U256,
	/// The address of the price oracle contract
	pub price_oracle: H160,
}

/// Destination fee configuration for a specific chain
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct DestinationFee {
	/// The percentage of fee (in basis points) charged for the destination chain
	/// 10000 = 100%, 5000 = 50%, etc.
	pub destination_fee_bps: U256,
	/// The state machine ID associated with the destination fee
	pub state_machine_id: H256,
}

/// Parameter update request from users/governance (optional fields pattern)
/// All fields are optional, only specified fields will be updated
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq, Default)]
pub struct ParamsUpdate {
	/// The address of the host contract
	pub host: Option<H160>,
	/// Address of the dispatcher contract
	pub dispatcher: Option<H160>,
	/// Flag indicating whether solver selection is enabled
	pub solver_selection: Option<bool>,
	/// The percentage of surplus (in basis points) that goes to the protocol
	pub surplus_share_bps: Option<U256>,
	/// The protocol fee in basis points charged on order inputs
	pub protocol_fee_bps: Option<U256>,
	/// The address of the price oracle contract
	pub price_oracle: Option<H160>,
	/// The destination fee parameters for specific chains
	pub destination_fees: Option<Vec<DestinationFee>>,
}

/// Complete parameter update for cross-chain dispatch (no optional fields)
/// This is used internally after merging ParamsUpdate with current parameters
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct CompleteParamsUpdate {
	/// The complete gateway parameters
	pub params: IntentGatewayParams,
	/// The destination fee parameters for specific chains
	pub destination_fees: Vec<DestinationFee>,
}

/// Request to add a new Intent Gateway deployment
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct NewDeployment {
	/// Identifier for the state machine
	pub state_machine_id: Vec<u8>,
	/// The gateway contract address
	pub gateway: H160,
}

/// Request to sweep accumulated dust/fees from an Intent Gateway
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct SweepDust {
	/// The address of the beneficiary
	pub beneficiary: H160,
	/// The tokens to be withdrawn
	pub outputs: Vec<TokenInfo>,
}

/// Token decimal configuration for a single token
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct TokenDecimal {
	/// Token address
	pub token: H160,
	/// Number of decimals
	pub decimals: u8,
}

/// Batch update for token decimals in the VWAP Oracle
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct TokenDecimalsUpdate {
	/// The source chain identifier
	pub source_chain: Vec<u8>,
	/// Array of token decimal configurations
	pub tokens: Vec<TokenDecimal>,
}

/// Information about a deployed Intent Gateway instance
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct GatewayInfo {
	/// The gateway contract address
	pub gateway: H160,
	/// Current parameters for this gateway
	pub params: IntentGatewayParams,
}

/// A bid placed by a filler for an order
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct Bid<AccountId> {
	/// The filler who placed this bid
	pub filler: AccountId,
	/// The signed user operation (opaque bytes)
	pub user_op: Vec<u8>,
}

impl IntentGatewayParams {
	/// Apply an update to the current parameters, returning a new instance
	pub fn update(&self, update: ParamsUpdate) -> Self {
		let mut params = self.clone();

		if let Some(host) = update.host {
			params.host = host;
		}
		if let Some(dispatcher) = update.dispatcher {
			params.dispatcher = dispatcher;
		}
		if let Some(solver_selection) = update.solver_selection {
			params.solver_selection = solver_selection;
		}
		if let Some(surplus_share_bps) = update.surplus_share_bps {
			params.surplus_share_bps = surplus_share_bps;
		}
		if let Some(protocol_fee_bps) = update.protocol_fee_bps {
			params.protocol_fee_bps = protocol_fee_bps;
		}
		if let Some(price_oracle) = update.price_oracle {
			params.price_oracle = price_oracle;
		}

		params
	}
}

/// Request kinds for cross-chain messages
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub enum RequestKind {
	/// Update Intent Gateway parameters (contains the complete merged params)
	UpdateParams(CompleteParamsUpdate),
	/// Add a new Intent Gateway deployment
	AddDeployment(NewDeployment),
	/// Sweep dust from an Intent Gateway
	SweepDust(SweepDust),
	/// Update token decimals in VWAP Oracle
	UpdateTokenDecimals(Vec<TokenDecimalsUpdate>),
}

// Solidity type definitions for cross-chain encoding
mod sol_types {
	use alloy_sol_types::sol;

	sol! {
		/// Solidity representation of Params
		struct Params {
			address host;
			address dispatcher;
			bool solverSelection;
			uint256 surplusShareBps;
			uint256 protocolFeeBps;
			address priceOracle;
		}

		/// Solidity representation of DestinationFee
		struct DestinationFee {
			uint256 destinationFeeBps;
			bytes32 stateMachineId;
		}

		/// Solidity representation of ParamsUpdate
		struct ParamsUpdate {
			Params params;
			DestinationFee[] destinationFees;
		}

		/// Solidity representation of NewDeployment
		struct NewDeployment {
			bytes stateMachineId;
			address gateway;
		}

		/// Solidity representation of TokenInfo
		struct TokenInfo {
			bytes32 token;
			uint256 amount;
		}

		/// Solidity representation of SweepDust
		struct SweepDust {
			address beneficiary;
			TokenInfo[] outputs;
		}

		/// Solidity representation of TokenDecimal
		struct TokenDecimal {
			address token;
			uint8 decimals;
		}

		/// Solidity representation of TokenDecimalsUpdate
		struct TokenDecimalsUpdate {
			bytes sourceChain;
			TokenDecimal[] tokens;
		}
	}
}

impl From<IntentGatewayParams> for sol_types::Params {
	fn from(params: IntentGatewayParams) -> Self {
		use alloy_primitives::{Address, U256 as AlloyU256};
		sol_types::Params {
			host: Address::from_slice(&params.host.0),
			dispatcher: Address::from_slice(&params.dispatcher.0),
			solverSelection: params.solver_selection,
			surplusShareBps: AlloyU256::from_limbs(params.surplus_share_bps.0),
			protocolFeeBps: AlloyU256::from_limbs(params.protocol_fee_bps.0),
			priceOracle: Address::from_slice(&params.price_oracle.0),
		}
	}
}

impl From<DestinationFee> for sol_types::DestinationFee {
	fn from(fee: DestinationFee) -> Self {
		use alloy_primitives::U256 as AlloyU256;
		sol_types::DestinationFee {
			destinationFeeBps: AlloyU256::from_limbs(fee.destination_fee_bps.0),
			stateMachineId: fee.state_machine_id.0.into(),
		}
	}
}

impl From<TokenInfo> for sol_types::TokenInfo {
	fn from(info: TokenInfo) -> Self {
		use alloy_primitives::U256 as AlloyU256;
		sol_types::TokenInfo {
			token: info.token.0.into(),
			amount: AlloyU256::from_limbs(info.amount.0),
		}
	}
}

impl From<TokenDecimal> for sol_types::TokenDecimal {
	fn from(td: TokenDecimal) -> Self {
		use alloy_primitives::Address;
		sol_types::TokenDecimal { token: Address::from_slice(&td.token.0), decimals: td.decimals }
	}
}

impl RequestKind {
	/// Encode the request kind for cross-chain dispatch
	pub fn encode_body(&self) -> Vec<u8> {
		match self {
			RequestKind::UpdateParams(update) => {
				// Convert complete params to Solidity format
				let params_sol: sol_types::Params = update.params.clone().into();
				let dest_fees_sol: Vec<sol_types::DestinationFee> =
					update.destination_fees.iter().cloned().map(Into::into).collect();

				let params_update_sol =
					sol_types::ParamsUpdate { params: params_sol, destinationFees: dest_fees_sol };

				// Prepend request kind identifier (0 for UpdateParams)
				let mut body = vec![0u8];
				body.extend_from_slice(&params_update_sol.abi_encode());
				body
			},
			RequestKind::AddDeployment(deployment) => {
				use alloy_primitives::Address;
				let deployment_sol = sol_types::NewDeployment {
					stateMachineId: deployment.state_machine_id.clone().into(),
					gateway: Address::from_slice(&deployment.gateway.0),
				};

				// Prepend request kind identifier (1 for AddDeployment)
				let mut body = vec![1u8];
				body.extend_from_slice(&deployment_sol.abi_encode());
				body
			},
			RequestKind::SweepDust(sweep) => {
				use alloy_primitives::Address;
				let outputs_sol: Vec<sol_types::TokenInfo> =
					sweep.outputs.iter().cloned().map(Into::into).collect();

				let sweep_sol = sol_types::SweepDust {
					beneficiary: Address::from_slice(&sweep.beneficiary.0),
					outputs: outputs_sol,
				};

				// Prepend request kind identifier (2 for SweepDust)
				let mut body = vec![2u8];
				body.extend_from_slice(&sweep_sol.abi_encode());
				body
			},
			RequestKind::UpdateTokenDecimals(updates) => {
				let updates_sol: Vec<sol_types::TokenDecimalsUpdate> = updates
					.iter()
					.map(|update| {
						let tokens_sol: Vec<sol_types::TokenDecimal> =
							update.tokens.iter().cloned().map(Into::into).collect();
						sol_types::TokenDecimalsUpdate {
							sourceChain: update.source_chain.clone().into(),
							tokens: tokens_sol,
						}
					})
					.collect();

				// Prepend request kind identifier (0 for UpdateTokenDecimals in VWAPOracle)
				let mut body = vec![0u8];
				body.extend_from_slice(&updates_sol.abi_encode());
				body
			},
		}
	}
}
