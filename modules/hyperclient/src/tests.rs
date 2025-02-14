// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg(not(target_arch = "wasm32"))]
use crate::testing::{get_request_handling, subscribe_to_request_status, test_timeout_request};

pub fn setup_logging() {
	use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};
	let filter =
		tracing_subscriber::EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());
	let _ = tracing_subscriber::fmt().with_env_filter(filter).finish().try_init();
}

#[tokio::test]
#[ignore]
async fn hyperclient_integration_tests() -> Result<(), anyhow::Error> {
	setup_logging();
	get_request_handling().await?;

	test_timeout_request().await?;
	subscribe_to_request_status().await?;

	Ok(())
}
