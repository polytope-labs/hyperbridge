use codec::{Decode, Encode};
use ismp::{
	router::{Request, Response},
};
use polkadot_sdk::{
	frame_support::pallet_prelude::TypeInfo,
	sp_core,
	sp_core::RuntimeDebug,
	sp_runtime::{traits::Zero, DispatchError, Weight},
};

pub type AuthorityId = sp_core::sr25519::Public;
pub type AuthoritySignature = sp_core::sr25519::Signature;

pub enum IncentivizedMessage {
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

/// A trait for Bridge price oracle.
pub trait PriceOracle<Balance> {
	fn get_bridge_price() -> Result<Balance, DispatchError>;
}

/// Weight information for pallet operations
pub trait WeightInfo {
	fn set_supported_route() -> Weight;
}

/// Default weight implementation using sensible defaults
impl WeightInfo for () {
	fn set_supported_route() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
}
