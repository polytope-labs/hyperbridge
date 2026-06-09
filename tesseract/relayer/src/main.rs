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

use clap::Parser;
use tesseract::cli::{Cli, Subcommand};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let cli = Cli::parse();

	// Short one-shot subcommands bypass the long-running relayer setup.
	// Matching on `&cli.subcommand` rather than moving it keeps `cli` intact
	// so we can still call `.run()` on the `None` arm below.
	match &cli.subcommand {
		Some(Subcommand::LogConsensusState { state_machine }) =>
			return cli.log_consensus_state(state_machine.clone()).await,
		Some(Subcommand::Withdraw) => return cli.withdraw_once().await,
		Some(Subcommand::AccumulateFees(cmd)) => return cmd.run(&cli.config, &cli.db).await,
		Some(Subcommand::ClaimRewards(cmd)) => return cmd.run(&cli.config, &cli.db).await,
		None => {},
	}

	cli.run().await
}
