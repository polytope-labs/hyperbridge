/// Log/tracing target for this crate.
pub const LOG_TARGET: &str = "consensus-relayer";

/// Re-export of the consensus host configs. Extracted to the shared
/// [`tesseract_consensus_config`] crate so the consolidated relayer
/// (`tesseract/relayer`) can consume the same enums without pulling in the
/// bin/cli side of this crate. Kept here under the old `any` name so in-tree
/// callers compile without churn.
pub use tesseract_consensus_config as any;

pub mod cli;
pub mod config;
pub mod logging;
pub mod monitor;
pub mod subcommand;
