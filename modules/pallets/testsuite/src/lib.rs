#![recursion_limit = "256"]

use polkadot_sdk::frame_support::__private::sp_tracing::tracing_subscriber;

mod runtime;
mod tests;

pub mod relay_chain;
pub mod xcm;

pub fn init_tracing() {
	let _ = tracing_subscriber::fmt()
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.try_init();
}
