use codec::{Decode, Encode};
use polkadot_sdk::frame_support::pallet_prelude::TypeInfo;
use polkadot_sdk::sp_core::RuntimeDebug;
use polkadot_sdk::sp_runtime::{DispatchError, Weight};
use polkadot_sdk::sp_runtime::traits::Zero;
use ismp::host::StateMachine;
use ismp::router::{PostRequest, Request, Response};

pub enum IncentivizedMessage {
    Post(PostRequest),
    Request(Request),
    Response(Response),
}

#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq, RuntimeDebug)]
pub struct EpochInfo<BlockNumber> {
    /// The index of the current epoch
    pub index: u64,
    /// The block number at which the epoch started
    pub start_block: BlockNumber,
}

impl<BlockNumber: Zero> Default for EpochInfo<BlockNumber> {
    fn default() -> Self {
        Self { index: 0, start_block: BlockNumber::zero() }
    }
}

/// A trait for a price oracle.
pub trait PriceOracle<Balance> {
    fn convert_to_usd(
        source_state_machine: StateMachine,
        amount: Balance,
    ) -> Result<Balance, DispatchError>;
}

/// Weight information for pallet operations
pub trait WeightInfo {
    fn set_supported_state_machines() -> Weight;
}

/// Default weight implementation using sensible defaults
impl WeightInfo for () {
    fn set_supported_state_machines() -> Weight {
        Weight::from_parts(10_000_000, 0)
    }
}

