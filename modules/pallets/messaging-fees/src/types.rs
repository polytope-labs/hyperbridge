use polkadot_sdk::{
	sp_core,
	sp_core::U256,
	sp_runtime::{DispatchError, Weight},
};

use ismp::router::{Request, Response};

pub const DECIMALS_12: u128 = 1_000_000_000_000;

pub type AuthorityId = sp_core::sr25519::Public;
pub type AuthoritySignature = sp_core::sr25519::Signature;

pub enum IncentivizedMessage {
	Request(Request, bool),
	Response(Response, bool),
}

/// A trait for Bridge price oracle.
pub trait PriceOracle {
	/// Should return the price as a U256 with 18 decimals.
	fn get_bridge_price() -> Result<U256, DispatchError>;
}

/// Weight information for pallet operations
pub trait WeightInfo {
	fn set_supported_route() -> Weight;
	fn set_target_message_size() -> Weight;
}

/// Default weight implementation using sensible defaults
impl WeightInfo for () {
	fn set_supported_route() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}

	fn set_target_message_size() -> Weight {
		Weight::from_parts(10_000_000, 0)
	}
}
