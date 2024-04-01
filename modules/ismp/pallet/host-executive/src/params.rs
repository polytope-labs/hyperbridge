use alloy_rlp_derive::{RlpDecodable, RlpEncodable};
use frame_support::{BoundedVec, __private::RuntimeDebug, pallet_prelude::ConstU32};
use sp_core::H160;

/// The host parameters of all connected chains
#[derive(
    Clone, codec::Encode, codec::Decode, scale_info::TypeInfo, PartialEq, Eq, RuntimeDebug, Default,
)]
pub struct HostParam {
    /// the minimum default timeout in seconds
    pub default_timeout: u128,
    /// Base fee for GET requests
    pub base_get_request_fee: u128,
    /// The fee to charge per byte
    pub per_byte_fee: u128,
    /// The address of the fee token contract
    pub fee_token_address: H160,
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
    /// The current consensus state
    pub consensus_state: BoundedVec<u8, ConstU32<100_000>>,
    /// Timestamp for when the consensus state was last updated
    pub last_updated: u128,
    /// The latest state machine height for hyperbridge
    pub latest_state_machine_height: u128,
    /// The state machine identifier for hyperbridge
    pub hyperbridge: BoundedVec<u8, ConstU32<1_000>>,
}

impl HostParam {
    /// Update the host params with the update struct. Will only modify fields that are set.
    pub fn update(&mut self, update: HostParamUpdate) {
        if let Some(default_timeout) = update.default_timeout {
            self.default_timeout = default_timeout;
        }

        if let Some(base_get_request_fee) = update.base_get_request_fee {
            self.base_get_request_fee = base_get_request_fee;
        }

        if let Some(per_byte_fee) = update.per_byte_fee {
            self.per_byte_fee = per_byte_fee;
        }

        if let Some(fee_token_address) = update.fee_token_address {
            self.fee_token_address = fee_token_address;
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

        if let Some(consensus_state) = update.consensus_state {
            self.consensus_state = consensus_state;
        }

        if let Some(last_updated) = update.last_updated {
            self.last_updated = last_updated;
        }

        if let Some(latest_state_machine_height) = update.latest_state_machine_height {
            self.latest_state_machine_height = latest_state_machine_height;
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
pub struct HostParamUpdate {
    /// the minimum default timeout in seconds
    pub default_timeout: Option<u128>,
    /// Base fee for GET requests
    pub base_get_request_fee: Option<u128>,
    /// The fee to charge per byte
    pub per_byte_fee: Option<u128>,
    /// The address of the fee token contract
    pub fee_token_address: Option<H160>,
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
    /// The current consensus state
    pub consensus_state: Option<BoundedVec<u8, ConstU32<100_000>>>,
    /// Timestamp for when the consensus state was last updated
    pub last_updated: Option<u128>,
    /// The latest state machine height for hyperbridge
    pub latest_state_machine_height: Option<u128>,
    /// The state machine identifier for hyperbridge
    pub hyperbridge: Option<BoundedVec<u8, ConstU32<1_000>>>,
}

/// The host parameters of all connected chains, ethereum friendly version
#[derive(Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct HostParamRlp {
    /// the minimum default timeout in seconds
    pub default_timeout: alloy_primitives::U256,
    /// Base fee for GET requests
    pub base_get_request_fee: alloy_primitives::U256,
    /// The fee to charge per byte
    pub per_byte_fee: alloy_primitives::U256,
    /// The address of the fee token contract
    pub fee_token_address: alloy_primitives::Address,
    /// The admin account
    pub admin: alloy_primitives::Address,
    /// The handler contract
    pub handler: alloy_primitives::Address,
    /// The host manager contract
    pub host_manager: alloy_primitives::Address,
    /// The unstaking period in seconds
    pub un_staking_period: alloy_primitives::U256,
    /// The configured challenge period
    pub challenge_period: alloy_primitives::U256,
    /// The consensus client contract
    pub consensus_client: alloy_primitives::Address,
    /// The current consensus state
    pub consensus_state: alloy_primitives::Bytes,
    /// Timestamp for when the consensus state was last updated
    pub last_updated: alloy_primitives::U256,
    /// The latest state machine height for hyperbridge
    pub latest_state_machine_height: alloy_primitives::U256,
    /// The state machine identifier for hyperbridge
    pub hyperbridge: alloy_primitives::Bytes,
}

impl TryFrom<HostParam> for HostParamRlp {
    type Error = anyhow::Error;

    fn try_from(value: HostParam) -> Result<Self, anyhow::Error> {
        Ok(HostParamRlp {
            default_timeout: value.default_timeout.try_into()?,
            base_get_request_fee: value.base_get_request_fee.try_into()?,
            per_byte_fee: value.per_byte_fee.try_into()?,
            fee_token_address: value.fee_token_address.0.try_into()?,
            admin: value.admin.0.try_into()?,
            handler: value.handler.0.try_into()?,
            host_manager: value.host_manager.0.try_into()?,
            un_staking_period: value.un_staking_period.try_into()?,
            challenge_period: value.challenge_period.try_into()?,
            consensus_client: value.consensus_client.0.try_into()?,
            consensus_state: value.consensus_state.to_vec().try_into()?,
            last_updated: value.last_updated.try_into()?,
            latest_state_machine_height: value.latest_state_machine_height.try_into()?,
            hyperbridge: value.hyperbridge.to_vec().try_into()?,
        })
    }
}
