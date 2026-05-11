/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "messaging-relayer";

mod cli;
mod config;
pub mod logging;

pub mod fees;

pub use cli::*;
