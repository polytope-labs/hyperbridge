pub mod referenda;

use super::*;

mod origins;
mod tracks;
pub use origins::{
	custom_origins, FellowshipAdmin, ReferendumCanceller, ReferendumKiller, WhitelistedCaller, *,
};

impl origins::custom_origins::Config for Runtime {}
