#![recursion_limit = "256"]


mod runtime;
mod tests;

pub mod relay_chain;
pub mod xcm;
pub mod asset_hub_runtime;
pub mod asset_hub_xcm;

pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}