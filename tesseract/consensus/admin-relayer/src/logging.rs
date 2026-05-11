// Copyright (C) 2023 Polytope Labs.
// SPDX-License-Identifier: Apache-2.0

use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};

/// Initialise the process-wide tracing subscriber.
///
/// Respects `RUST_LOG`; defaults to `info` when unset.
pub fn setup() -> anyhow::Result<()> {
	let filter =
		tracing_subscriber::EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());
	tracing_subscriber::fmt().with_env_filter(filter).finish().try_init()?;
	Ok(())
}
