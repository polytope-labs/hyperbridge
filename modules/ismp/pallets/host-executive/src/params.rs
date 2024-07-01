use alloc::{vec, vec::Vec};
use frame_support::{pallet_prelude::ConstU32, BoundedVec};
use pallet_hyperbridge::VersionedHostParams;
use sp_core::H160;
use sp_runtime::RuntimeDebug;

/// The host parameters of all connected chains
#[derive(
	Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug,
)]
pub enum HostParam<T> {
	/// Host params for substrate-based hosts
	SubstrateHostParam(VersionedHostParams<T>),
	/// Host params for evm-based hosts
	EvmHostParam(EvmHostParam),
}

/// Struct for modifying the host parameters of all connected chains
#[derive(
	Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug,
)]
pub enum HostParamUpdate<T> {
	/// Host param updates for substrate-based hosts
	SubstrateHostParam(VersionedHostParams<T>),
	/// Host params updates for evm-based hosts
	EvmHostParam(EvmHostParamUpdate),
}

/// The host parameters for evm-based hosts
#[derive(
	Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug, Default,
)]
pub struct EvmHostParam {
	/// the minimum default timeout in seconds
	pub default_timeout: u128,
	/// The fee to charge per byte
	pub per_byte_fee: u128,
	/// The address of the fee token contract
	pub fee_token: H160,
	/// The admin account
	pub admin: H160,
	/// The handler contract
	pub handler: H160,
	/// The host manager contract
	pub host_manager: H160,
	/// The unstaking period in seconds
	pub un_staking_period: u128,
	/// The configured challenge period
	pub challenge_period: u128,
	/// The consensus client contract
	pub consensus_client: H160,
	/// The state machine identifier for hyperbridge
	pub state_machine_whitelist: BoundedVec<u32, ConstU32<1_000>>,
	/// List of fishermen
	pub fishermen: BoundedVec<H160, ConstU32<1_000>>,
	/// The state machine identifier for hyperbridge
	pub hyperbridge: BoundedVec<u8, ConstU32<1_000>>,
}

impl EvmHostParam {
	/// Update the host params with the update struct. Will only modify fields that are set.
	pub fn update(&mut self, update: EvmHostParamUpdate) {
		if let Some(default_timeout) = update.default_timeout {
			self.default_timeout = default_timeout;
		}

		if let Some(per_byte_fee) = update.per_byte_fee {
			self.per_byte_fee = per_byte_fee;
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

		if let Some(state_machine_whitelist) = update.state_machine_whitelist {
			self.state_machine_whitelist = state_machine_whitelist;
		}

		if let Some(fishermen) = update.fishermen {
			self.fishermen = fishermen;
		}

		if let Some(hyperbridge) = update.hyperbridge {
			self.hyperbridge = hyperbridge;
		}
	}
}

/// Struct for modifying the host params
#[derive(
	Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug, Default,
)]
pub struct EvmHostParamUpdate {
	/// the minimum default timeout in seconds
	pub default_timeout: Option<u128>,
	/// The fee to charge per byte
	pub per_byte_fee: Option<u128>,
	/// The address of the fee token contract
	pub fee_token: Option<H160>,
	/// The admin account
	pub admin: Option<H160>,
	/// The handler contract
	pub handler: Option<H160>,
	/// The host manager contract
	pub host_manager: Option<H160>,
	/// The unstaking period in seconds
	pub un_staking_period: Option<u128>,
	/// The configured challenge period
	pub challenge_period: Option<u128>,
	/// The consensus client contract
	pub consensus_client: Option<H160>,
	/// The state machine identifier for hyperbridge
	pub state_machine_whitelist: Option<BoundedVec<u32, ConstU32<1_000>>>,
	/// List of fishermen
	pub fishermen: Option<BoundedVec<H160, ConstU32<1_000>>>,
	/// The state machine identifier for hyperbridge
	pub hyperbridge: Option<BoundedVec<u8, ConstU32<1_000>>>,
}

alloy_sol_macro::sol! {
	#![sol(all_derives)]

	// The IsmpHost parameters
	struct EvmHostParamsAbi {
		// default timeout in seconds for requests.
		uint256 defaultTimeout;
		// cost of cross-chain requests in the fee token per byte
		uint256 perByteFee;
		// The fee token contract. This will typically be DAI.
		// but we allow it to be configurable to prevent future regrets.
		address feeToken;
		// admin account, this only has the rights to freeze, or unfreeze the bridge
		address admin;
		// Ismp request/response handler
		address handler;
		// the authorized host manager contract
		address hostManager;
		// unstaking period
		uint256 unStakingPeriod;
		// minimum challenge period in seconds;
		uint256 challengePeriod;
		// consensus client contract
		address consensusClient;
		// whitelisted state machines
		uint256[] stateMachines;
		// white list of fishermen accounts
		address[] fishermen;
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
			perByteFee: value.per_byte_fee.try_into().map_err(anyhow::Error::msg)?,
			feeToken: value.fee_token.0.try_into().map_err(anyhow::Error::msg)?,
			admin: value.admin.0.try_into().map_err(anyhow::Error::msg)?,
			handler: value.handler.0.try_into().map_err(anyhow::Error::msg)?,
			hostManager: value.host_manager.0.try_into().map_err(anyhow::Error::msg)?,
			unStakingPeriod: value.un_staking_period.try_into().map_err(anyhow::Error::msg)?,
			challengePeriod: value.challenge_period.try_into().map_err(anyhow::Error::msg)?,
			consensusClient: value.consensus_client.0.try_into().map_err(anyhow::Error::msg)?,
			stateMachines: value
				.state_machine_whitelist
				.into_iter()
				.map(|id| id.try_into().map_err(anyhow::Error::msg))
				.collect::<Result<Vec<_>, anyhow::Error>>()?,
			hyperbridge: value.hyperbridge.to_vec().into(),
			fishermen: value
				.fishermen
				.into_iter()
				.map(|address| address.0.try_into().map_err(anyhow::Error::msg))
				.collect::<Result<Vec<_>, _>>()?,
		})
	}
}
