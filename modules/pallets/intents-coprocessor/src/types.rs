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
use crypto_utils::verification::Signature;
use ismp::{host::StateMachine, messaging::Proof};
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

/// A recognized token pair for price tracking
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct TokenPair {
	/// The base token address
	pub base: H160,
	/// The quote token address
	pub quote: H160,
}

impl TokenPair {
	/// Compute a unique identifier for this token pair
	pub fn pair_id(&self) -> H256 {
		let mut data = alloc::vec::Vec::with_capacity(40);
		data.extend_from_slice(&self.base.0);
		data.extend_from_slice(&self.quote.0);
		sp_io::hashing::keccak_256(&data).into()
	}
}

/// Caller-provided price data for a specific range of base token amounts.
/// The pallet fills in the filler address and timestamp when storing.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct PriceInput {
	/// Lower bound of the base token amount range (inclusive), with 18 decimal places
	pub range_start: U256,
	/// Upper bound of the base token amount range (inclusive), with 18 decimal places
	pub range_end: U256,
	/// The price of the base token in the quote token, with 18 decimal places
	pub price: U256,
}

/// An individual price submission stored on-chain. The price applies to a specific
/// range of base token amounts, allowing fillers to quote different rates for
/// different order sizes (e.g. USDC/CNGN: 0-999 -> 1414, 1000-5000 -> 1420).
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct PriceEntry {
	/// The filler's EVM address. Set to `H160::zero()` for unverified submissions.
	pub filler: H160,
	/// Lower bound of the base token amount range (inclusive), with 18 decimal places
	pub range_start: U256,
	/// Upper bound of the base token amount range (inclusive), with 18 decimal places
	pub range_end: U256,
	/// The price of the base token in the quote token, with 18 decimal places
	pub price: U256,
	/// Timestamp of submission (seconds)
	pub timestamp: u64,
}

/// Verification data for proven price submissions (high confidence)
///
/// When provided, the submission is treated as a verified filler price.
/// The EVM signature proves the substrate account owner also controls the
/// EVM account that filled the order.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct PriceVerificationData {
	/// The state machine where the order was filled
	pub state_machine: StateMachine,
	/// The filled order commitment hash
	pub commitment: H256,
	/// Proof that the order was filled at some height
	pub membership_proof: Proof,
	/// Proof that the order was not filled at an earlier height
	pub non_membership_proof: Proof,
	/// EVM signature proving ownership of the filler's EVM account.
	/// The signer must sign `keccak256(encode(nonce, pair_id, price))`.
	pub evm_signature: Signature,
}

/// Compute the message hash that the filler must sign with their EVM key.
///
/// Message = keccak256(SCALE_encode(nonce, pair_id, price))
pub fn price_signature_message(nonce: u64, pair_id: &H256, price: &U256) -> [u8; 32] {
	sp_io::hashing::keccak_256(&(nonce, pair_id, price).encode())
}

/// The storage slot index for the `_filled` mapping in IntentGateway.sol
pub const FILLED_SLOT: [u8; 32] =
	hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000005");

/// Compute the EVM state proof key for `_filled[commitment]` on the given gateway contract.
///
/// Returns a 52-byte key: 20-byte contract address + 32-byte storage slot.
/// The EVM state machine client uses the first 20 bytes to locate the contract
/// and hashes the last 32 bytes to derive the storage trie key.
pub fn filled_storage_key(gateway: &H160, commitment: &H256) -> Vec<u8> {
	// Compute the raw storage slot: keccak256(commitment ++ FILLED_SLOT)
	let mut slot_preimage = Vec::with_capacity(64);
	slot_preimage.extend_from_slice(commitment.as_bytes());
	slot_preimage.extend_from_slice(&FILLED_SLOT);
	let slot = sp_io::hashing::keccak_256(&slot_preimage);

	// 52-byte key: gateway address (20) + slot (32)
	let mut key = Vec::with_capacity(52);
	key.extend_from_slice(&gateway.0);
	key.extend_from_slice(&slot);
	key
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

/// Mirrors the `RequestKind` enum in `IntentsBase.sol`.
#[repr(u8)]
enum IntentGatewayRequestKind {
	RedeemEscrow = 0,
	NewDeployment = 1,
	UpdateParams = 2,
	SweepDust = 3,
	RefundEscrow = 4,
}

/// Mirrors the `RequestKind` enum in `VWAPOracle.sol`.
#[repr(u8)]
enum VWAPOracleRequestKind {
	UpdateTokenDecimals = 0,
}

impl RequestKind {
	/// Encode the request kind for cross-chain dispatch
	pub fn encode_body(&self) -> Vec<u8> {
		match self {
			RequestKind::UpdateParams(update) => {
				let params_sol: sol_types::Params = update.params.clone().into();
				let dest_fees_sol: Vec<sol_types::DestinationFee> =
					update.destination_fees.iter().cloned().map(Into::into).collect();

				let params_update_sol =
					sol_types::ParamsUpdate { params: params_sol, destinationFees: dest_fees_sol };

				let mut body = vec![IntentGatewayRequestKind::UpdateParams as u8];
				body.extend_from_slice(&params_update_sol.abi_encode());
				body
			},
			RequestKind::AddDeployment(deployment) => {
				use alloy_primitives::Address;
				let deployment_sol = sol_types::NewDeployment {
					stateMachineId: deployment.state_machine_id.clone().into(),
					gateway: Address::from_slice(&deployment.gateway.0),
				};

				let mut body = vec![IntentGatewayRequestKind::NewDeployment as u8];
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

				let mut body = vec![IntentGatewayRequestKind::SweepDust as u8];
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

				let mut body = vec![VWAPOracleRequestKind::UpdateTokenDecimals as u8];
				body.extend_from_slice(&updates_sol.abi_encode());
				body
			},
		}
	}
}
