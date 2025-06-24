// Copyright (C) 2023 Polytope Labs.
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

use clap::Parser;
use tesseract_consensus::{cli::Cli, subcommand::Subcommand};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	rustls::crypto::ring::default_provider()
		.install_default()
		.expect("Failed to install rustls crypto provider");
	let cli = Cli::parse();
	if let Some(subcommand) = cli.subcommand {
		match subcommand {
			Subcommand::LogConsensusState(set_consensus_state) => {
				set_consensus_state.log_consensus_state(cli.config.clone()).await?;
			},
			Subcommand::LogHostParams(set_consensus_state) => {
				set_consensus_state.log_host_param(cli.config.clone()).await?;
			},
		}
		return Ok(());
	}

	cli.start_consensus().await
}
