use super::{explorer, PalletSpec};

pub const KEY: &str = "governance";

/**
 * The events of the governance pallets, each with its arguments as JSON and accounts in SS58,
 * as a governance forum shows them. `crate::governance` builds them from the block's events,
 * so the mapper never maps an event to these.
 */
pub fn spec() -> PalletSpec {
	PalletSpec { key: KEY, metadata_name: "", events: vec![explorer::event()] }
}
