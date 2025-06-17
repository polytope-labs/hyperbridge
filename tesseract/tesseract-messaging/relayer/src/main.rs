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
use tesseract::{fees::Subcommand, Cli};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let cli = Cli::parse();
	if let Some(command) = cli.subcommand {
		match command {
			Subcommand::AccumulateFees(cmd) =>
				cmd.accumulate_fees(cli.config.clone(), cli.db.clone()).await?,
		}
		return Ok(());
	}
	cli.run().await
}
