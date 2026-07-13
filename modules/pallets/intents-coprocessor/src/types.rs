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
use ismp::consensus::StateMachineId;
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

/// Pricing and treasury parameters for a SimplexPaymaster instance. Mirrors the
/// `Params` struct in `SimplexPaymaster.sol`; the contract replaces these wholesale
/// on `UpdateParams` (no merge semantics).
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct PaymasterParams {
	/// Native asset / USD Chainlink feed
	pub native_oracle: H160,
	/// Markup in basis points applied on top of the oracle price (10000 = 100%)
	pub markup_bps: U256,
	/// Receives markup surplus and EntryPoint deposit withdrawals
	pub treasury: H160,
	/// Maximum Chainlink oracle staleness, in seconds
	pub max_oracle_age: U256,
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

/// Upper bound on the token pairs a single config may probe, and therefore on the number
/// of phantom orders active at once.
pub const MAX_PHANTOM_TOKEN_PAIRS: u32 = 64;

/// Tracks a phantom order recognised by the pallet.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct PhantomOrderInfo<BlockNumber> {
	pub created_at_block: BlockNumber,
	/// Raw state machine identifier bytes (e.g. b"EVM-8453").
	pub chain: Vec<u8>,
}

/// A single token pair the phantom generator probes for price and liquidity.
///
/// Read the note on [`standard_amount`](PhantomTokenPair::standard_amount) before you set this.
/// It is the one field that fails silently.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct PhantomTokenPair {
	/// The input token being priced (tokenA). Quotes are expressed as how much `token_b`
	/// a filler will give for `standard_amount` of this token.
	pub token_a: H160,
	/// The output token (tokenB) the price is quoted in.
	pub token_b: H160,
	/// ┌─────────────────────────────────────────────────────────────────────────────────┐
	/// │  ⚠  EXACTLY ONE (1) UNIT OF THE INPUT TOKEN. NO MORE. NO LESS. NON-NEGOTIABLE.  ⚠  │
	/// └─────────────────────────────────────────────────────────────────────────────────┘
	///
	/// This MUST be **one whole unit of `token_a`, denominated in the token's smallest unit**
	/// — that is, `10^decimals(token_a)`:
	///   • 6-decimal USDC  →  `1_000_000`
	///   • 18-decimal DAI  →  `1_000_000_000_000_000_000`
	///
	/// WHY THERE IS ZERO WIGGLE ROOM:
	///   Every exchange rate the indexer publishes is `medianPrice / standard_amount`. This number
	///   is the DENOMINATOR OF THE TRUTH. Put `2` units here and every downstream rate is silently
	///   HALVED; put half a unit and every rate silently DOUBLES. It will not revert. It will not
	///   warn. It will simply poison every price snapshot for this pair with an integer-factor
	///   error until a human eventually notices the feed has drifted — and then has to backfill it.
	///
	/// So set it to one unit. `10^decimals(token_a)`. Not a round dollar. Not a "nice" number.
	/// Not two. Not a half. ONE. UNIT.
	pub standard_amount: u128,
}

/// Governance-settable configuration for autonomous phantom order generation.
/// Stored in `PhantomOrderConfig`; the pallet hook reads it every block. The
/// `chain` carries the consensus state id so the hook can look up the latest
/// confirmed height directly instead of scanning every state machine.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct PhantomOrderConfiguration {
	pub chain: StateMachineId,
	pub token_pairs: BoundedVec<PhantomTokenPair, ConstU32<MAX_PHANTOM_TOKEN_PAIRS>>,
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
	/// Upgrade the Intent Gateway implementation behind its ERC-1967 proxy
	UpgradeContract {
		/// The new implementation contract address
		new_impl: H160,
		/// Optional migration calldata run atomically against the proxy on upgrade
		init_data: Vec<u8>,
	},
	/// Upgrade the SimplexPaymaster implementation behind its ERC-1967 proxy
	PaymasterUpgrade {
		/// The new implementation contract address
		new_impl: H160,
		/// Optional migration calldata run atomically against the proxy on upgrade
		init_data: Vec<u8>,
	},
	/// Replace the SimplexPaymaster pricing and treasury parameters
	PaymasterUpdateParams(PaymasterParams),
	/// Register or update a supported token and its token/USD feed on the paymaster
	PaymasterRegisterToken {
		/// The ERC-20 token to support
		token: H160,
		/// The token/USD Chainlink feed
		oracle: H160,
	},
	/// Deactivate a token on the paymaster (kill-switch)
	PaymasterDeactivateToken {
		/// The ERC-20 token to deactivate
		token: H160,
	},
	/// Sweep paymaster assets to its treasury (ERC-20 surplus, or the EntryPoint
	/// deposit when `token` is the zero address)
	PaymasterWithdrawAssets {
		/// The ERC-20 token to sweep, or the zero address for the native deposit
		token: H160,
		/// The amount to withdraw
		amount: U256,
	},
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

		/// Solidity representation of SimplexPaymaster.Params
		struct PaymasterParams {
			address nativeOracle;
			uint256 markupBps;
			address treasury;
			uint256 maxOracleAge;
		}
	}
}

/// Builds the IntentGatewayV2 `Order` for a phantom order and returns both the
/// ABI-encoded bytes and its `keccak256` commitment.
///
/// `deadline` is the EVM block number beyond which the gateway treats the order
/// as expired (`order.deadline < block.number` reverts).
pub fn phantom_order_commitment(
	block: u64,
	chain: &[u8],
	token_a: &H160,
	token_b: &H160,
	standard_amount: u128,
	deadline: u64,
) -> (H256, Vec<u8>) {
	use alloy_primitives::{Bytes, FixedBytes, U256 as AlloyU256};

	let mut token_a_bytes = [0u8; 32];
	token_a_bytes[12..].copy_from_slice(token_a.as_bytes());
	let mut token_b_bytes = [0u8; 32];
	token_b_bytes[12..].copy_from_slice(token_b.as_bytes());

	let order = sol_types::Order {
		user: FixedBytes::from([0u8; 32]),
		source: Bytes::copy_from_slice(chain),
		destination: Bytes::copy_from_slice(chain),
		deadline: AlloyU256::from(deadline),
		nonce: AlloyU256::from(block),
		fees: AlloyU256::ZERO,
		session: alloy_primitives::Address::ZERO,
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

impl From<PaymasterParams> for sol_types::PaymasterParams {
	fn from(params: PaymasterParams) -> Self {
		use alloy_primitives::{Address, U256 as AlloyU256};
		sol_types::PaymasterParams {
			nativeOracle: Address::from_slice(&params.native_oracle.0),
			markupBps: AlloyU256::from_limbs(params.markup_bps.0),
			treasury: Address::from_slice(&params.treasury.0),
			maxOracleAge: AlloyU256::from_limbs(params.max_oracle_age.0),
		}
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
	UpgradeContract = 5,
}

/// Mirrors the `RequestKind` enum in `VWAPOracle.sol`.
#[repr(u8)]
enum VWAPOracleRequestKind {
	UpdateTokenDecimals = 0,
}

/// Mirrors the `RequestKind` enum in `SimplexPaymaster.sol`. Note the
/// discriminators differ from `IntentGatewayRequestKind` (e.g. UpgradeContract
/// is 0 here, 5 there), so this must not be conflated with it.
#[repr(u8)]
enum SimplexPaymasterRequestKind {
	UpgradeContract = 0,
	UpdateParams = 1,
	RegisterToken = 2,
	DeactivateToken = 3,
	WithdrawAssets = 4,
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
			RequestKind::UpgradeContract { new_impl, init_data } => {
				use alloy_primitives::{Address, Bytes};
				// Mirrors the Solidity `abi.decode(body[1:], (address, bytes))` in
				// `ExtrinsicIntents.onAccept`. `abi_encode_params` emits the two values as bare
				// ABI parameters (no outer tuple wrapper), matching `abi.encode(newImpl, initData)`.
				let payload = (Address::from_slice(&new_impl.0), Bytes::from(init_data.clone()));

				let mut body = vec![IntentGatewayRequestKind::UpgradeContract as u8];
				body.extend_from_slice(&payload.abi_encode_params());
				body
			},
			RequestKind::PaymasterUpgrade { new_impl, init_data } => {
				use alloy_primitives::{Address, Bytes};
				// Matches `abi.decode(payload, (address, bytes))` in `SimplexPaymaster.onAccept`.
				let payload = (Address::from_slice(&new_impl.0), Bytes::from(init_data.clone()));

				let mut body = vec![SimplexPaymasterRequestKind::UpgradeContract as u8];
				body.extend_from_slice(&payload.abi_encode_params());
				body
			},
			RequestKind::PaymasterUpdateParams(params) => {
				let params_sol: sol_types::PaymasterParams = params.clone().into();

				let mut body = vec![SimplexPaymasterRequestKind::UpdateParams as u8];
				// Single struct: `abi_encode` is tuple-wrapped, matching `abi.decode(payload, (Params))`.
				body.extend_from_slice(&params_sol.abi_encode());
				body
			},
			RequestKind::PaymasterRegisterToken { token, oracle } => {
				use alloy_primitives::Address;
				let payload = (Address::from_slice(&token.0), Address::from_slice(&oracle.0));

				let mut body = vec![SimplexPaymasterRequestKind::RegisterToken as u8];
				body.extend_from_slice(&payload.abi_encode_params());
				body
			},
			RequestKind::PaymasterDeactivateToken { token } => {
				use alloy_primitives::Address;

				let mut body = vec![SimplexPaymasterRequestKind::DeactivateToken as u8];
				// Single value: matches `abi.decode(payload, (address))`.
				body.extend_from_slice(&Address::from_slice(&token.0).abi_encode());
				body
			},
			RequestKind::PaymasterWithdrawAssets { token, amount } => {
				use alloy_primitives::{Address, U256 as AlloyU256};
				let payload = (Address::from_slice(&token.0), AlloyU256::from_limbs(amount.0));

				let mut body = vec![SimplexPaymasterRequestKind::WithdrawAssets as u8];
				body.extend_from_slice(&payload.abi_encode_params());
				body
			},
		}
	}
}

#[cfg(test)]
mod request_kind_tests {
	use super::*;
	use alloy_primitives::{Address, Bytes};

	// Proves the UpgradeContract body matches Solidity `abi.decode(body[1:], (address, bytes))`.
	#[test]
	fn upgrade_contract_encode_matches_solidity_two_param_abi() {
		let new_impl = H160::repeat_byte(0x11);
		let body = RequestKind::UpgradeContract { new_impl, init_data: Vec::new() }.encode_body();

		// Discriminator byte must equal the EVM enum value (UpgradeContract = 5).
		assert_eq!(body[0], 5, "discriminator must match IntentsBase.RequestKind.UpgradeContract");

		// Hand-computed `abi.encode(address, bytes)` for (0x11 * 20, "") — exactly 96 bytes:
		//   word0: address right-aligned in 32 bytes
		//   word1: offset to the bytes tail (0x40)
		//   word2: byte length (0)
		let mut expected = Vec::new();
		expected.extend_from_slice(&[0u8; 12]);
		expected.extend_from_slice(&[0x11u8; 20]);
		let mut offset = [0u8; 32];
		offset[31] = 0x40;
		expected.extend_from_slice(&offset);
		expected.extend_from_slice(&[0u8; 32]);
		assert_eq!(&body[1..], expected.as_slice(), "payload must equal abi.encode(addr, bytes)");

		// And it decodes through the exact Solidity analogue.
		let (decoded_impl, decoded_data) =
			<(Address, Bytes)>::abi_decode_params(&body[1..]).expect("decodes as (address, bytes)");
		assert_eq!(decoded_impl.as_slice(), &new_impl.0);
		assert!(decoded_data.is_empty());
	}

	#[test]
	fn upgrade_contract_encode_with_init_data_round_trips() {
		let new_impl = H160::repeat_byte(0xAB);
		let init_data = vec![0xDE, 0xAD, 0xBE, 0xEF];
		let body =
			RequestKind::UpgradeContract { new_impl, init_data: init_data.clone() }.encode_body();
		assert_eq!(body[0], 5);

		let (decoded_impl, decoded_data) =
			<(Address, Bytes)>::abi_decode_params(&body[1..]).expect("decodes as (address, bytes)");
		assert_eq!(decoded_impl.as_slice(), &new_impl.0);
		assert_eq!(decoded_data.as_ref(), init_data.as_slice());
	}

	// SimplexPaymaster discriminators differ from IntentGateway's; pin each one.
	#[test]
	fn paymaster_upgrade_matches_solidity_two_param_abi() {
		let new_impl = H160::repeat_byte(0x22);
		let init_data = vec![0x01, 0x02];
		let body =
			RequestKind::PaymasterUpgrade { new_impl, init_data: init_data.clone() }.encode_body();

		assert_eq!(body[0], 0, "SimplexPaymaster.RequestKind.UpgradeContract == 0");
		let (decoded_impl, decoded_data) =
			<(Address, Bytes)>::abi_decode_params(&body[1..]).expect("decodes as (address, bytes)");
		assert_eq!(decoded_impl.as_slice(), &new_impl.0);
		assert_eq!(decoded_data.as_ref(), init_data.as_slice());
	}

	#[test]
	fn paymaster_update_params_matches_solidity_struct_abi() {
		use alloy_primitives::U256 as AlloyU256;
		let params = PaymasterParams {
			native_oracle: H160::repeat_byte(0x33),
			markup_bps: U256::from(200),
			treasury: H160::repeat_byte(0x44),
			max_oracle_age: U256::from(90_000),
		};
		let body = RequestKind::PaymasterUpdateParams(params.clone()).encode_body();

		assert_eq!(body[0], 1, "SimplexPaymaster.RequestKind.UpdateParams == 1");
		// Decodes as a single tuple-wrapped struct, matching `abi.decode(payload, (Params))`.
		let decoded =
			sol_types::PaymasterParams::abi_decode(&body[1..]).expect("decodes as Params");
		assert_eq!(decoded.nativeOracle.as_slice(), &params.native_oracle.0);
		assert_eq!(decoded.markupBps, AlloyU256::from(200));
		assert_eq!(decoded.treasury.as_slice(), &params.treasury.0);
		assert_eq!(decoded.maxOracleAge, AlloyU256::from(90_000));
	}

	#[test]
	fn paymaster_register_token_matches_solidity_two_address_abi() {
		let token = H160::repeat_byte(0x55);
		let oracle = H160::repeat_byte(0x66);
		let body = RequestKind::PaymasterRegisterToken { token, oracle }.encode_body();

		assert_eq!(body[0], 2, "SimplexPaymaster.RequestKind.RegisterToken == 2");
		let (dt, dor) =
			<(Address, Address)>::abi_decode_params(&body[1..]).expect("(address, address)");
		assert_eq!(dt.as_slice(), &token.0);
		assert_eq!(dor.as_slice(), &oracle.0);
	}

	#[test]
	fn paymaster_deactivate_token_matches_solidity_single_address_abi() {
		let token = H160::repeat_byte(0x77);
		let body = RequestKind::PaymasterDeactivateToken { token }.encode_body();

		assert_eq!(body[0], 3, "SimplexPaymaster.RequestKind.DeactivateToken == 3");
		let decoded = Address::abi_decode(&body[1..]).expect("decodes as address");
		assert_eq!(decoded.as_slice(), &token.0);
	}

	#[test]
	fn paymaster_withdraw_assets_matches_solidity_address_uint_abi() {
		use alloy_primitives::U256 as AlloyU256;
		let token = H160::repeat_byte(0x88);
		let amount = U256::from(1_000_000u64);
		let body = RequestKind::PaymasterWithdrawAssets { token, amount }.encode_body();

		assert_eq!(body[0], 4, "SimplexPaymaster.RequestKind.WithdrawAssets == 4");
		let (dt, da) =
			<(Address, AlloyU256)>::abi_decode_params(&body[1..]).expect("(address, uint256)");
		assert_eq!(dt.as_slice(), &token.0);
		assert_eq!(da, AlloyU256::from(1_000_000u64));
	}
}
