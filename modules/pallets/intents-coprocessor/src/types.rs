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
use ismp::host::StateMachine;
use polkadot_sdk::frame_support::{traits::ConstU32, BoundedVec};
use primitive_types::{H160, H256, U256};
use scale_info::TypeInfo;
use sp_io;

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
	/// The raw state machine ID bytes for the destination chain (hashed on-chain)
	pub chain: Vec<u8>,
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
	/// The raw state machine ID bytes for the deployment chain
	pub chain: Vec<u8>,
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

/// Tracks the single active phantom order recognised by the pallet.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct PhantomOrderInfo<BlockNumber> {
	pub created_at_block: BlockNumber,
	/// Raw state machine identifier bytes (e.g. b"EVM-8453").
	pub chain: Vec<u8>,
}

/// A single token pair the phantom generator probes for price and liquidity.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct PhantomTokenPair {
	pub token_a: H160,
	pub token_b: H160,
	pub standard_amount: u128,
	pub min_output: u128,
}

/// Governance-settable configuration for autonomous phantom order generation.
/// Stored in `PhantomOrderConfig`; the pallet hook reads it every block.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct PhantomOrderConfiguration {
	pub chain: StateMachine,
	pub token_pairs: BoundedVec<PhantomTokenPair, ConstU32<10>>,
	pub interval_blocks: u32,
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
pub(crate) mod sol_types {
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
			bytes chain;
		}

		/// Solidity representation of ParamsUpdate
		struct ParamsUpdate {
			Params params;
			DestinationFee[] destinationFees;
		}

		/// Solidity representation of NewDeployment
		struct NewDeployment {
			bytes chain;
			address gateway;
		}

		/// Solidity representation of TokenInfo
		struct TokenInfo {
			bytes32 token;
			uint256 amount;
		}

		struct DispatchInfo {
			TokenInfo[] assets;
			bytes call;
		}

		struct PaymentInfo {
			bytes32 beneficiary;
			TokenInfo[] assets;
			bytes call;
		}

		struct Order {
			bytes32 user;
			bytes source;
			bytes destination;
			uint256 deadline;
			uint256 nonce;
			uint256 fees;
			address session;
			DispatchInfo predispatch;
			TokenInfo[] inputs;
			PaymentInfo output;
		}

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

/// Fixed session address for all phantom orders, derived from private key 0x01.
/// The indexer uses the matching private key when signing simulated bids.
pub const PHANTOM_SESSION_ADDRESS: alloy_primitives::Address =
	alloy_primitives::address!("7E5F4552091A69125d5DfCb7b8C2659029395Bdf");

/// Builds the IntentGatewayV2 `Order` for a phantom order and returns both the
/// ABI-encoded bytes and its `keccak256` commitment.
///
/// `deadline_secs` is the Unix timestamp (in seconds) beyond which the order is
/// considered expired by the gateway.  Callers should set this to a non-zero
/// value; zero prevents the order from simulating correctly.
pub fn phantom_order_commitment(
	block: u64,
	chain: &[u8],
	token_a: &H160,
	token_b: &H160,
	standard_amount: u128,
	deadline_secs: u64,
) -> (H256, Vec<u8>) {
	use alloy_primitives::{Address, Bytes, FixedBytes, U256 as AlloyU256};

	let mut token_a_bytes = [0u8; 32];
	token_a_bytes[12..].copy_from_slice(token_a.as_bytes());
	let mut token_b_bytes = [0u8; 32];
	token_b_bytes[12..].copy_from_slice(token_b.as_bytes());

	let order = sol_types::Order {
		user: FixedBytes::from([0u8; 32]),
		source: Bytes::copy_from_slice(chain),
		destination: Bytes::copy_from_slice(chain),
		deadline: AlloyU256::from(deadline_secs),
		nonce: AlloyU256::from(block),
		fees: AlloyU256::ZERO,
		session: PHANTOM_SESSION_ADDRESS,
		predispatch: sol_types::DispatchInfo { assets: vec![], call: Bytes::new() },
		inputs: vec![sol_types::TokenInfo {
			token: FixedBytes::from(token_a_bytes),
			amount: AlloyU256::from(standard_amount),
		}],
		output: sol_types::PaymentInfo {
			beneficiary: FixedBytes::from([0u8; 32]),
			assets: vec![sol_types::TokenInfo {
				token: FixedBytes::from(token_b_bytes),
				amount: AlloyU256::ZERO,
			}],
			call: Bytes::new(),
		},
	};

	let encoded = order.abi_encode();
	let commitment = sp_io::hashing::keccak_256(&encoded).into();
	(commitment, encoded)
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
			chain: fee.chain.into(),
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
					chain: deployment.chain.clone().into(),
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
