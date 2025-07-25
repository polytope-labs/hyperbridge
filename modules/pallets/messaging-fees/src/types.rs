use polkadot_sdk::{
	sp_core,
	sp_core::U256,
	sp_runtime::{DispatchError, Weight},
};

use ismp::router::{Request, Response};

pub type AuthorityId = sp_core::sr25519::Public;
pub type AuthoritySignature = sp_core::sr25519::Signature;

pub enum IncentivizedMessage {
	Request(Request),
	Response(Response),
}

/// A trait for Bridge price oracle.
pub trait PriceOracle {
	fn get_bridge_price() -> Result<U256, DispatchError>;
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
